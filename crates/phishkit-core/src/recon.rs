use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use std::sync::Arc;

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

/// Opt-in: allow recon against loopback / RFC1918 / link-local / ULA.
/// Set `PHISHKIT_RECON_ALLOW_PRIVATE=1` for lab demos that deliberately target
/// 127.0.0.1 or a private staging host. Default is fail-closed (block).
fn recon_allow_private() -> bool {
    matches!(
        std::env::var("PHISHKIT_RECON_ALLOW_PRIVATE").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

/// Opt-in: skip TLS certificate verification for recon fetches.
/// Default is verify. Only for lab targets with self-signed certs.
fn recon_insecure_tls() -> bool {
    matches!(
        std::env::var("PHISHKIT_RECON_INSECURE_TLS").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

/// True if this IPv4 address is not a safe default recon target.
pub fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    if ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
    {
        return true;
    }
    // CGNAT / shared address space (RFC 6598)
    let octets = ip.octets();
    if octets[0] == 100 && (octets[1] & 0xc0) == 64 {
        return true;
    }
    // IETF protocol assignments / documentation (RFC 5737, RFC 2544-ish)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
        return true;
    }
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
        return true;
    }
    if octets[0] == 198 && (octets[1] == 18 || octets[1] == 19) {
        return true;
    }
    if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
        return true;
    }
    if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
        return true;
    }
    // "This network" 0.0.0.0/8 already covered by unspecified for 0.0.0.0;
    // also block the rest of 0/8.
    if octets[0] == 0 {
        return true;
    }
    false
}

/// True if this IPv6 address is not a safe default recon target.
pub fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() {
        return true;
    }
    // IPv4-mapped: apply the IPv4 rules to the embedded address.
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_blocked_ipv4(v4);
    }
    let segments = ip.segments();
    // Link-local fe80::/10
    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    // Unique local fc00::/7
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    // Site-local deprecated fec0::/10
    if (segments[0] & 0xffc0) == 0xfec0 {
        return true;
    }
    false
}

pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => is_blocked_ipv6(v6),
    }
}

/// Hostnames that must never be probed without an explicit private allow.
fn is_blocked_hostname(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    matches!(
        h.as_str(),
        "localhost"
            | "localhost.localdomain"
            | "ip6-localhost"
            | "ip6-loopback"
            | "metadata.google.internal"
            | "metadata"
    ) || h.ends_with(".localhost")
        || h.ends_with(".localdomain")
        || h == "0"
}

/// Resolve `host` and refuse if any answer is a blocked IP, or if the host
/// itself is a blocked name / IP literal. When `allow_private` is set the
/// check is a no-op (lab opt-in).
pub fn assert_host_allowed(host: &str, allow_private: bool) -> AppResult<()> {
    if allow_private {
        return Ok(());
    }
    let host = host.trim().trim_end_matches('.');
    if host.is_empty() {
        return Err(AppError::msg("Empty host is not allowed for recon"));
    }
    if is_blocked_hostname(host) {
        return Err(AppError::msg(format!(
            "Refusing recon against blocked host `{host}` (loopback/metadata). Set PHISHKIT_RECON_ALLOW_PRIVATE=1 for lab targets."
        )));
    }
    // IP literal?
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(AppError::msg(format!(
                "Refusing recon against blocked address `{ip}` (loopback/private/link-local/ULA). Set PHISHKIT_RECON_ALLOW_PRIVATE=1 for lab targets."
            )));
        }
        return Ok(());
    }
    // Bracketed IPv6 literal without port (url crate may pass without brackets)
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = bare.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(AppError::msg(format!(
                "Refusing recon against blocked address `{ip}` (loopback/private/link-local/ULA). Set PHISHKIT_RECON_ALLOW_PRIVATE=1 for lab targets."
            )));
        }
        return Ok(());
    }

    // DNS resolve and check every answer. Fail closed on resolve errors only when
    // we can prove a blocked answer; a total resolve failure is left to the HTTP
    // client (surface as a fetch error), but any blocked answer refuses the fetch.
    let lookup = format!("{host}:443");
    match lookup.to_socket_addrs() {
        Ok(addrs) => {
            let mut saw_any = false;
            for addr in addrs {
                saw_any = true;
                if is_blocked_ip(addr.ip()) {
                    return Err(AppError::msg(format!(
                        "Refusing recon against `{host}`: resolves to blocked address {} (loopback/private/link-local/ULA). Set PHISHKIT_RECON_ALLOW_PRIVATE=1 for lab targets.",
                        addr.ip()
                    )));
                }
            }
            if !saw_any {
                return Err(AppError::msg(format!(
                    "Refusing recon against `{host}`: DNS returned no addresses"
                )));
            }
            Ok(())
        }
        Err(e) => {
            // Do not silently probe an unresolvable name through a weird path;
            // surface a clear error (fail closed on the guard itself).
            Err(AppError::msg(format!(
                "Refusing recon against `{host}`: DNS resolve failed ({e})"
            )))
        }
    }
}

/// Parse a URL (or host) and refuse if the authority is blocked.
pub fn assert_fetch_url_allowed(url_or_host: &str, allow_private: bool) -> AppResult<()> {
    let raw = url_or_host.trim();
    if raw.is_empty() {
        return Err(AppError::msg("Empty URL is not allowed for recon"));
    }
    let to_parse = if raw.contains("://") {
        raw.to_string()
    } else if raw.starts_with("//") {
        format!("https:{raw}")
    } else {
        format!("https://{raw}")
    };
    let parsed = url::Url::parse(&to_parse)
        .map_err(|e| AppError::msg(format!("Refusing recon: invalid URL `{raw}` ({e})")))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(AppError::msg(format!(
                "Refusing recon: scheme `{other}` is not allowed (http/https only)"
            )));
        }
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::msg("Refusing recon: URL has no host"))?;
    assert_host_allowed(host, allow_private)
}

fn build_recon_client(
    allow_private: bool,
    insecure_tls: bool,
) -> AppResult<reqwest::blocking::Client> {
    let allow_private = Arc::new(allow_private);
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("phishkit-desktop/0.1 (authorized-assessment-recon)")
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.error("too many redirects");
            }
            let url = attempt.url().as_str();
            if let Err(e) = assert_fetch_url_allowed(url, *allow_private) {
                return attempt.error(e.to_string());
            }
            attempt.follow()
        }));
    if insecure_tls {
        builder = builder.danger_accept_invalid_certs(true);
    }
    Ok(builder.build()?)
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

    let allow_private = recon_allow_private();
    let insecure_tls = recon_insecure_tls();
    // Fail closed on the operator-supplied target before any HTTP happens.
    assert_host_allowed(&host, allow_private)?;
    assert_fetch_url_allowed(&base, allow_private)?;
    let client = build_recon_client(allow_private, insecure_tls)?;

    let mut corpus = String::new();
    let mut header_blob = String::new();
    let mut error = None;
    let mut pages = vec![base.clone(), format!("{base}/"), format!("{base}/login")];
    pages.sort();
    pages.dedup();

    let mut assets: Vec<String> = Vec::new();
    for page in &pages {
        if let Err(e) = assert_fetch_url_allowed(page, allow_private) {
            if error.is_none() {
                error = Some(e.to_string());
            }
            continue;
        }
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
        // Re-check every asset URL (may point off-host after HTML parsing).
        if assert_fetch_url_allowed(&abs, allow_private).is_err() {
            continue;
        }
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

#[cfg(test)]
mod ssrf_tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn blocks_loopback_rfc1918_link_local_cgnat() {
        assert!(is_blocked_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(127, 255, 255, 255)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(172, 31, 255, 255)));
        assert!(!is_blocked_ipv4(Ipv4Addr::new(172, 32, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(169, 254, 169, 254))); // cloud metadata
        assert!(is_blocked_ipv4(Ipv4Addr::new(169, 254, 1, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(100, 64, 0, 1))); // CGNAT
        assert!(is_blocked_ipv4(Ipv4Addr::new(0, 0, 0, 0)));
        assert!(is_blocked_ipv4(Ipv4Addr::BROADCAST));
        // Public-looking addresses stay allowed
        assert!(!is_blocked_ipv4(Ipv4Addr::new(1, 1, 1, 1)));
        assert!(!is_blocked_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
        assert!(!is_blocked_ipv4(Ipv4Addr::new(93, 184, 216, 34)));
    }

    #[test]
    fn blocks_ipv6_loopback_ula_link_local_and_mapped() {
        assert!(is_blocked_ipv6(Ipv6Addr::LOCALHOST));
        assert!(is_blocked_ipv6(Ipv6Addr::UNSPECIFIED));
        assert!(is_blocked_ipv6("fc00::1".parse().unwrap()));
        assert!(is_blocked_ipv6("fd12:3456:789a::1".parse().unwrap()));
        assert!(is_blocked_ipv6("fe80::1".parse().unwrap()));
        // IPv4-mapped loopback / RFC1918
        assert!(is_blocked_ipv6("::ffff:127.0.0.1".parse().unwrap()));
        assert!(is_blocked_ipv6("::ffff:10.0.0.1".parse().unwrap()));
        assert!(is_blocked_ipv6("::ffff:169.254.169.254".parse().unwrap()));
        // Public IPv6 stays allowed
        assert!(!is_blocked_ipv6("2001:4860:4860::8888".parse().unwrap()));
    }

    #[test]
    fn assert_host_blocks_literals_and_localhost_name() {
        let err = assert_host_allowed("127.0.0.1", false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("blocked"), "{err}");
        let err = assert_host_allowed("10.1.2.3", false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("blocked"), "{err}");
        let err = assert_host_allowed("169.254.169.254", false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("blocked"), "{err}");
        let err = assert_host_allowed("localhost", false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("blocked"), "{err}");
        let err = assert_host_allowed("metadata.google.internal", false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("blocked"), "{err}");
        // Opt-in clears the gate
        assert!(assert_host_allowed("127.0.0.1", true).is_ok());
        assert!(assert_host_allowed("localhost", true).is_ok());
    }

    #[test]
    fn assert_fetch_url_blocks_private_and_non_http_schemes() {
        assert!(assert_fetch_url_allowed("https://127.0.0.1:8888/", false).is_err());
        assert!(assert_fetch_url_allowed("http://192.168.0.5/login", false).is_err());
        assert!(assert_fetch_url_allowed("file:///etc/passwd", false).is_err());
        assert!(assert_fetch_url_allowed("ftp://example.com/", false).is_err());
        // Public host with https is allowed at the URL-shape layer (DNS checked separately)
        // Use an IP that is clearly public so we do not depend on live DNS here.
        assert!(assert_fetch_url_allowed("https://1.1.1.1/", false).is_ok());
    }

    #[test]
    fn detect_target_refuses_loopback_without_opt_in() {
        // Repro of the audit finding: detect against 127.0.0.1 used to fetch.
        // With the guard it must fail closed before any HTTP.
        let err = detect_target("http://127.0.0.1:8888/")
            .unwrap_err()
            .to_string();
        assert!(
            err.to_lowercase().contains("blocked") || err.to_lowercase().contains("refusing"),
            "expected SSRF refusal, got: {err}"
        );
        let err2 = detect_target("localhost").unwrap_err().to_string();
        assert!(
            err2.to_lowercase().contains("blocked") || err2.to_lowercase().contains("refusing"),
            "expected SSRF refusal, got: {err2}"
        );
    }
}
