use regex::Regex;
use serde::Serialize;

use crate::engagement::{normalize_target_host, upstream_domain};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
pub struct StackInfo {
    pub stack: String,
    pub label: String,
    pub signals: Vec<String>,
    pub firebase_keys: Vec<String>,
    pub login_path: Option<String>,
    /// Cloudflare / CDN bot wall detected
    pub cloudflare: bool,
    /// Turnstile or challenge-platform markers
    pub turnstile: bool,
    /// "good" | "caution" | "poor" for AiTM via evilginx
    pub suitability: String,
    pub suitability_notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ReconResult {
    pub ok: bool,
    pub target_host: String,
    pub upstream_domain: String,
    pub stack_info: StackInfo,
    pub error: Option<String>,
}

const MARKERS: &[(&str, &str, &str, i32)] = &[
    (
        r"signInWithPassword",
        "Firebase signInWithPassword",
        "firebase",
        5,
    ),
    (
        r"identitytoolkit\.googleapis\.com",
        "Google Identity Toolkit",
        "firebase",
        5,
    ),
    (
        r"securetoken\.googleapis\.com",
        "Firebase securetoken",
        "firebase",
        4,
    ),
    (
        r"firebase(?:-|\.)(?:app|auth|messaging)",
        "Firebase SDK",
        "firebase",
        3,
    ),
    (r"firebase/auth", "Firebase Auth SDK", "firebase", 4),
    (
        r"getAuth\s*\(|initializeAuth\s*\(",
        "Firebase getAuth",
        "firebase",
        3,
    ),
    (
        r"firebasestorage\.googleapis\.com",
        "Firebase Storage host",
        "firebase",
        1,
    ),
    (r"auth0\.com", "Auth0", "auth0", 4),
    (r"okta\.com", "Okta", "okta", 4),
    (r"cognito-idp\.", "AWS Cognito", "cognito", 4),
    (r"/api/token", "OAuth token endpoint", "jwt_body", 2),
    (
        r"grant_type=password",
        "OAuth password grant",
        "jwt_body",
        3,
    ),
    (
        r"login\.microsoftonline\.com",
        "Microsoft OAuth",
        "oauth",
        2,
    ),
    (r"accounts\.google\.com", "Google OAuth", "oauth", 1),
    (
        r"localStorage\.(?:get|set)Item",
        "localStorage session",
        "cookie_session",
        1,
    ),
];

fn stack_label(stack: &str) -> &'static str {
    match stack {
        "firebase" => "Firebase Auth (React / SPA)",
        "jwt_body" => "JWT / API body token",
        "auth0" => "Auth0",
        "okta" => "Okta",
        "cognito" => "AWS Cognito",
        "oauth" => "OAuth / SSO",
        "cookie_session" => "Cookie / browser storage",
        _ => "SPA / generic",
    }
}

fn resolve_asset(host: &str, src: &str) -> Option<String> {
    let src = src.trim();
    if src.is_empty() || src.starts_with("data:") {
        return None;
    }
    if src.starts_with("http://") || src.starts_with("https://") {
        return Some(src.to_string());
    }
    if src.starts_with("//") {
        return Some(format!("https:{src}"));
    }
    if src.starts_with('/') {
        return Some(format!("https://{host}{src}"));
    }
    Some(format!("https://{host}/{src}"))
}

/// Collect JS asset URLs from HTML (script src + modulepreload/module href).
fn collect_js_assets(host: &str, html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let patterns = [
        r#"(?i)<script[^>]+src=["']([^"']+)["']"#,
        r#"(?i)<link[^>]+rel=["'](?:modulepreload|preload|module)["'][^>]+href=["']([^"']+)["']"#,
        r#"(?i)<link[^>]+href=["']([^"']+\.js[^"']*)["'][^>]+rel=["'](?:modulepreload|preload|module)["']"#,
    ];
    for pat in patterns {
        if let Ok(re) = Regex::new(pat) {
            for cap in re.captures_iter(html) {
                if let Some(m) = cap.get(1) {
                    if let Some(abs) = resolve_asset(host, m.as_str()) {
                        if abs.contains(".js") || abs.contains("/assets/") {
                            out.push(abs);
                        }
                    }
                }
            }
        }
    }
    // Also catch bare /assets/*.js strings in inline bootstraps
    if let Ok(re) = Regex::new(r#"["'](/assets/[^"']+\.js)["']"#) {
        for cap in re.captures_iter(html) {
            if let Some(abs) = resolve_asset(host, &cap[1]) {
                out.push(abs);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn asset_priority(url: &str) -> i32 {
    let u = url.to_ascii_lowercase();
    let file = u.rsplit('/').next().unwrap_or(&u);
    if file.contains("firebase") || file.contains("identity") || file.contains("signin") {
        return 0;
    }
    // Vite auth route chunk (auth-xxxxx.js) — follow its imports next
    if file.starts_with("auth-") || file.contains("/auth-") || file == "auth.js" {
        return 0;
    }
    if file.contains("login") || file.contains("sign-in") || file.contains("signin") {
        return 1;
    }
    if file.contains("main")
        || file.contains("index")
        || file.contains("app")
        || file.contains("bundle")
    {
        return 2;
    }
    if file.contains("constants") || file.contains("config") || file.contains("shared") {
        return 3;
    }
    // Heavy UI / admin chunks rarely hold Identity Toolkit; dig later
    if file.contains("vendor")
        || file.contains("react")
        || file.contains("mui")
        || file.contains("ag-grid")
        || file.contains("d3-")
        || file.contains("tiptap")
        || file.contains("super-admin")
    {
        return 9;
    }
    5
}

/// Pull nested `assets/*.js` references out of a Vite chunk (import maps / dynamic imports).
fn nested_js_assets(host: &str, js: &str) -> Vec<String> {
    let mut out = Vec::new();
    let re = Regex::new(r#"["']((?:/)?assets/[^"']+\.js)["']"#).unwrap();
    for cap in re.captures_iter(js) {
        if let Some(abs) = resolve_asset(host, &cap[1]) {
            out.push(abs);
        }
    }
    out
}

/// Prefer primary login over MFA / step-up paths.
fn pick_login_path(candidates: &[String]) -> Option<String> {
    let mut scored: Vec<(i32, String)> = candidates
        .iter()
        .map(|p| {
            let pl = p.to_ascii_lowercase();
            let mut score = 0i32;
            if pl == "/login" || pl == "/signin" || pl == "/sign-in" {
                score -= 50;
            }
            if pl.contains("mfa")
                || pl.contains("2fa")
                || pl.contains("otp")
                || pl.contains("verify")
            {
                score += 40;
            }
            if pl.matches('/').count() > 2 {
                score += 5;
            }
            score += pl.len() as i32 / 10;
            (score, p.clone())
        })
        .collect();
    scored.sort_by_key(|(s, _)| *s);
    scored.into_iter().map(|(_, p)| p).next()
}

fn find_login_paths(corpus: &str) -> Vec<String> {
    let mut found = Vec::new();
    let re =
        Regex::new(r#"['\"](/(?:login|signin|sign-in|auth(?:/login)?)[^'\"]{0,40})['\"]"#).unwrap();
    for cap in re.captures_iter(corpus) {
        let p = cap[1].to_string();
        if !found.contains(&p) {
            found.push(p);
        }
    }
    found
}

pub fn detect_target(url_or_host: &str) -> AppResult<ReconResult> {
    let host = normalize_target_host(url_or_host);
    if host.is_empty() {
        return Err(AppError::msg("Enter a target domain or URL"));
    }
    let base = if url_or_host.trim().starts_with("http") {
        let t = url_or_host.trim().trim_end_matches('/');
        // If path-less URL, use host root; also probe /login
        t.to_string()
    } else {
        format!("https://{host}")
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .user_agent("phishkit-desktop/0.1 (authorized-assessment-recon)")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    let mut corpus = String::new();
    let mut header_blob = String::new();
    let mut error = None;
    let mut pages = vec![base.clone(), format!("{base}/"), format!("{base}/login")];
    pages.sort();
    pages.dedup();

    let mut assets: Vec<String> = Vec::new();
    for page in &pages {
        match client.get(page).send() {
            Ok(resp) => {
                for (k, v) in resp.headers().iter() {
                    header_blob.push_str(k.as_str());
                    header_blob.push(':');
                    if let Ok(s) = v.to_str() {
                        header_blob.push_str(s);
                    }
                    header_blob.push('\n');
                }
                if let Ok(text) = resp.text() {
                    corpus.push_str(&text);
                    corpus.push('\n');
                    assets.extend(collect_js_assets(&host, &text));
                }
            }
            Err(e) => {
                if error.is_none() {
                    error = Some(format!("Fetch failed for {page}: {e}"));
                }
            }
        }
    }

    // Stable order: auth-related first, then expand their Vite import maps.
    assets.sort_by(|a, b| {
        asset_priority(a)
            .cmp(&asset_priority(b))
            .then_with(|| a.cmp(b))
    });
    assets.dedup();

    let mut queued = assets;
    let mut seen = std::collections::HashSet::new();
    let mut auth_deps: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut fetched = 0usize;
    const MAX_FETCH: usize = 18;
    const SCAN_BYTES: usize = 1_600_000;

    let prio = |url: &str, auth_deps: &std::collections::HashSet<String>| -> i32 {
        if auth_deps.contains(url) {
            // Auth-graph chunks beat generic preloads (Firebase often lives in a sibling home/app chunk)
            return asset_priority(url).min(2);
        }
        asset_priority(url)
    };

    while fetched < MAX_FETCH {
        queued.sort_by(|a, b| {
            prio(a, &auth_deps)
                .cmp(&prio(b, &auth_deps))
                .then_with(|| a.cmp(b))
        });
        let Some(abs) = queued.iter().find(|u| !seen.contains(*u)).cloned() else {
            break;
        };
        seen.insert(abs.clone());
        let Ok(r) = client.get(&abs).send() else {
            continue;
        };
        let Ok(js) = r.text() else {
            continue;
        };
        // Expand Vite import maps from the auth-* chunk only (login/main graphs are too noisy)
        let file = abs.rsplit('/').next().unwrap_or(&abs).to_ascii_lowercase();
        if file.starts_with("auth-") || file == "auth.js" {
            for nested in nested_js_assets(&host, &js) {
                auth_deps.insert(nested.clone());
                if !seen.contains(&nested) && !queued.iter().any(|q| q == &nested) {
                    queued.push(nested);
                }
            }
        }
        let slice = if js.len() > SCAN_BYTES {
            &js[..SCAN_BYTES]
        } else {
            &js
        };
        corpus.push_str(slice);
        corpus.push('\n');
        fetched += 1;

        // Early stop once Firebase is obvious
        if corpus.contains("signInWithPassword")
            || corpus.contains("identitytoolkit.googleapis.com")
        {
            break;
        }
    }

    let key_re = Regex::new(r"AIza[0-9A-Za-z_-]{30,40}").unwrap();
    let firebase_keys: Vec<String> = key_re
        .find_iter(&corpus)
        .map(|m| m.as_str().to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .take(5)
        .collect();

    let mut scores: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let mut signals = Vec::new();
    for (pat, label, stack, weight) in MARKERS {
        if let Ok(re) = Regex::new(pat) {
            if re.is_match(&corpus) {
                *scores.entry((*stack).to_string()).or_insert(0) += weight;
                if !signals.iter().any(|s| s == label) {
                    signals.push((*label).to_string());
                }
            }
        }
    }
    // API key alone is a strong Firebase signal
    if !firebase_keys.is_empty() {
        *scores.entry("firebase".into()).or_insert(0) += 4;
        signals.push("Firebase API key in bundle".into());
    }

    let stack = scores
        .into_iter()
        .max_by_key(|(_, w)| *w)
        .map(|(s, _)| s)
        .unwrap_or_else(|| "generic_spa".into());

    let login_path = pick_login_path(&find_login_paths(&corpus)).or_else(|| {
        // Sensible default for SPAs
        if stack == "firebase" || stack == "generic_spa" || stack == "jwt_body" {
            Some("/login".into())
        } else {
            None
        }
    });

    let combined = format!("{header_blob}\n{corpus}");
    let combined_l = combined.to_ascii_lowercase();
    let cloudflare = combined_l.contains("cf-ray")
        || combined_l.contains("cloudflare")
        || combined_l.contains("__cf_bm")
        || combined_l.contains("cf-challenge")
        || combined_l.contains("cdn-cgi/");
    let turnstile = combined_l.contains("turnstile")
        || combined_l.contains("challenge-platform")
        || combined_l.contains("challenges.cloudflare.com")
        || combined_l.contains("cf-chl");

    if cloudflare {
        signals.push("Cloudflare CDN / Bot Management".into());
    }
    if turnstile {
        signals.push("Cloudflare Turnstile / challenge".into());
    }

    let (suitability, suitability_notes) =
        assess_suitability(&stack, cloudflare, turnstile, &signals);

    Ok(ReconResult {
        ok: true,
        target_host: host.clone(),
        upstream_domain: upstream_domain(&host),
        stack_info: StackInfo {
            label: stack_label(&stack).to_string(),
            stack,
            signals: signals.into_iter().take(14).collect(),
            firebase_keys,
            login_path,
            cloudflare,
            turnstile,
            suitability,
            suitability_notes,
        },
        error,
    })
}

fn assess_suitability(
    stack: &str,
    cloudflare: bool,
    turnstile: bool,
    signals: &[String],
) -> (String, Vec<String>) {
    let mut notes = Vec::new();
    if turnstile {
        notes.push(
            "Turnstile / Bot Management will usually block full AiTM — expect failure without a staging CF exception."
                .into(),
        );
    } else if cloudflare {
        notes.push(
            "Cloudflare is in front of this site — AiTM may work for softer CF modes but often fails under Bot Management."
                .into(),
        );
    }
    if stack == "oauth" {
        notes.push(
            "OAuth / SSO ejects to the IdP (Google/Microsoft). Consent-phishing or staging exceptions beat reverse-proxy AiTM."
                .into(),
        );
    }
    if stack == "firebase" && !cloudflare && !turnstile {
        notes.push(
            "Firebase Auth SPA is a strong fit for evilginx (credential + token capture + restore)."
                .into(),
        );
    }
    if matches!(stack, "jwt_body" | "cookie_session" | "generic_spa") && !cloudflare && !turnstile {
        notes.push(
            "First-party login looks proxyable — verify auth_tokens / sub_filters after a test login."
                .into(),
        );
    }
    if signals.iter().any(|s| s.contains("Google OAuth")) {
        notes.push(
            "Google login itself is a poor evilginx target — prefer capturing app session cookies after SSO returns."
                .into(),
        );
    }

    let suitability = if turnstile || (cloudflare && stack == "oauth") {
        "poor"
    } else if cloudflare || stack == "oauth" {
        "caution"
    } else if matches!(
        stack,
        "firebase" | "jwt_body" | "cookie_session" | "generic_spa" | "auth0" | "okta" | "cognito"
    ) {
        "good"
    } else {
        "caution"
    }
    .to_string();

    if notes.is_empty() {
        notes.push(match suitability.as_str() {
            "good" => "Looks suitable for AiTM dry-run with evilginx.".into(),
            "poor" => {
                "Poor AiTM candidate — document as needs exemption or use a different technique."
                    .into()
            }
            _ => "Proceed carefully; validate with a controlled test account.".into(),
        });
    }
    (suitability, notes)
}
