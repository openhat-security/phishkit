use std::path::Path;

use regex::Regex;
use serde::Serialize;

use crate::error::AppResult;
use crate::kit::kit_root;

pub fn normalize_target_host(target: &str) -> String {
    let mut host = target.trim().to_ascii_lowercase();
    if let Some(rest) = host.strip_prefix("https://") {
        host = rest.to_string();
    } else if let Some(rest) = host.strip_prefix("http://") {
        host = rest.to_string();
    }
    host.split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_string()
}

pub fn slugify(name: &str) -> String {
    let re = Regex::new(r"[^a-zA-Z0-9_-]+").unwrap();
    let s = re
        .replace_all(name, "-")
        .trim_matches('-')
        .to_ascii_lowercase();
    if s.is_empty() {
        "target".into()
    } else {
        s
    }
}

/// Multi-label public suffixes where the registrable domain (eTLD+1) needs three
/// labels, not two. This is a curated shortlist — not the full Public Suffix List —
/// covering the compound-suffix and platform-host cases the audit flagged (M1).
/// Keeping it small avoids a heavyweight PSL dependency while fixing the common
/// `*.co.uk` / `*.github.io` breakage that fed the broken phishlet generation.
const MULTI_LABEL_SUFFIXES: &[&str] = &[
    // ccTLD second levels
    "co.uk",
    "org.uk",
    "gov.uk",
    "ac.uk",
    "me.uk",
    "ltd.uk",
    "plc.uk",
    "net.uk",
    "sch.uk",
    "com.au",
    "net.au",
    "org.au",
    "edu.au",
    "gov.au",
    "id.au",
    "co.nz",
    "net.nz",
    "org.nz",
    "govt.nz",
    "co.jp",
    "or.jp",
    "ne.jp",
    "ac.jp",
    "go.jp",
    "co.in",
    "co.za",
    "co.kr",
    "co.il",
    "co.th",
    "com.br",
    "com.cn",
    "com.sg",
    "com.hk",
    "com.tr",
    "com.mx",
    "com.ar",
    "com.tw",
    // Common app-platform hosts whose subdomain is the tenant, not a subsite
    "github.io",
    "gitlab.io",
    "pages.dev",
    "web.app",
    "firebaseapp.com",
    "vercel.app",
    "netlify.app",
    "herokuapp.com",
    "azurewebsites.net",
    "cloudfront.net",
];

/// Registrable domain (eTLD+1) for `target_host`.
///
/// - Single-label internal hosts (e.g. `intranet`) are returned unchanged — there
///   is no registrable suffix to strip.
/// - Hosts under a known multi-label suffix keep three labels (`app.acme.co.uk`
///   -> `acme.co.uk`).
/// - Everything else collapses to the last two labels (`app.acme.com` -> `acme.com`).
pub fn upstream_domain(target_host: &str) -> String {
    let host = normalize_target_host(target_host);
    if host.is_empty() {
        return host;
    }
    let parts: Vec<&str> = host.split('.').collect();
    // Single-label internal host — nothing to strip.
    if parts.len() < 2 {
        return host;
    }
    for suffix in MULTI_LABEL_SUFFIXES {
        if host == *suffix {
            return host;
        }
        let dotted = format!(".{suffix}");
        if host.ends_with(&dotted) {
            let suffix_labels = suffix.matches('.').count() + 1;
            if parts.len() > suffix_labels {
                return parts[parts.len() - (suffix_labels + 1)..].join(".");
            }
            return host;
        }
    }
    parts[parts.len() - 2..].join(".")
}

pub fn default_dryrun_domain(target_host: &str) -> String {
    let base = upstream_domain(target_host);
    let label = base.split('.').next().unwrap_or("target");
    format!("{label}.phishkit")
}

pub fn phishlet_name_for_target(target_host: &str) -> String {
    let host = normalize_target_host(target_host);
    slugify(&host)
}

/// Exact website hostname from the URL the operator entered (no invented www./login. prefix).
pub fn site_host(target_host: &str) -> String {
    normalize_target_host(target_host)
}

/// Left-most label for evilginx phish_sub / orig_sub.
/// Subdomains (app.x.com, www.x.com) → that label only; apex → empty.
pub fn landing_sub(target_host: &str) -> String {
    let host = normalize_target_host(target_host);
    if host.is_empty() {
        return String::new();
    }
    let upstream = upstream_domain(&host);
    if host == upstream {
        return String::new();
    }
    host.strip_suffix(&format!(".{upstream}"))
        .unwrap_or("")
        .to_string()
}

/// Join an evilginx `phish_sub`/`orig_sub` with its base domain. An empty sub is an
/// apex host (just the domain), matching evilginx's own `combineHost` behaviour.
pub fn combine_host(sub: &str, domain: &str) -> String {
    let sub = sub.trim();
    let domain = domain.trim();
    if sub.is_empty() {
        domain.to_string()
    } else {
        format!("{sub}.{domain}")
    }
}

fn inline_field(entry: &str, key: &str) -> Option<String> {
    // Matches `key: 'value'` or `key: value` inside a YAML flow map entry.
    let re = Regex::new(&format!(
        r#"{key}\s*:\s*(?:'([^']*)'|"([^"]*)"|([^,}}\s]+))"#
    ))
    .ok()?;
    let caps = re.captures(entry)?;
    caps.get(1)
        .or_else(|| caps.get(2))
        .or_else(|| caps.get(3))
        .map(|m| m.as_str().trim().to_string())
}

/// Phishing landing hosts (`is_landing: true`) declared by a phishlet YAML body.
///
/// evilginx rejects two enabled phishlets that resolve to the same landing host
/// ("hostname collision"), which is exactly the C1.5 failure where
/// `ensure-destination` scaffolds a duplicate. Computing these hosts lets us dedupe
/// before writing a colliding file.
pub fn landing_hosts_in_yaml(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if !(trimmed.contains("is_landing: true") || trimmed.contains("is_landing:true")) {
            continue;
        }
        let Some(domain) = inline_field(trimmed, "domain") else {
            continue;
        };
        let phish = inline_field(trimmed, "phish_sub").unwrap_or_default();
        let host = combine_host(&phish, &domain);
        if !host.is_empty() && !out.contains(&host) {
            out.push(host);
        }
    }
    out
}

/// Find an existing phishlet whose landing host equals `target_host`, excluding
/// `skip_stem`. ensure-destination uses this to reuse the existing phishlet instead
/// of scaffolding a duplicate that collides at evilginx load time (C1.5).
pub fn find_reusable_phishlet(
    phishlets_dir: &Path,
    target_host: &str,
    skip_stem: &str,
) -> Option<String> {
    let host = normalize_target_host(target_host);
    if host.is_empty() {
        return None;
    }
    let mut stems: Vec<(String, std::path::PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(phishlets_dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("yaml") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem == skip_stem {
            continue;
        }
        stems.push((stem.to_string(), path));
    }
    // Deterministic order so reuse is stable across runs/filesystems.
    stems.sort_by(|a, b| a.0.cmp(&b.0));
    for (stem, path) in stems {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if landing_hosts_in_yaml(&text).iter().any(|h| h == &host) {
            return Some(stem);
        }
    }
    None
}

#[derive(Debug, Serialize)]
pub struct ResolveResult {
    pub ok: bool,
    pub target_domain: String,
    pub dryrun_domain: String,
    pub phishlet: String,
    pub upstream_domain: String,
    pub needs_generate: bool,
    pub error: Option<String>,
}

const PLACEHOLDERS: &[&str] = &["TARGET_DOMAIN", "LOOKALIKE_DOMAIN", "PORTAL_HOST"];

pub fn phishlet_is_valid(path: &std::path::Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    for line in text.lines() {
        let stripped = line.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        if PLACEHOLDERS.iter().any(|m| line.contains(m)) {
            return false;
        }
    }
    true
}

pub fn resolve_engagement(
    target_domain: Option<String>,
    dryrun_domain: Option<String>,
    phishlet: Option<String>,
) -> AppResult<ResolveResult> {
    let root = kit_root()?;
    let phishlets_dir = root.join("kit/evilginx/phishlets");
    let target = normalize_target_host(target_domain.as_deref().unwrap_or(""));
    let explicit_dryrun = dryrun_domain.unwrap_or_default().trim().to_string();
    let explicit_phishlet = phishlet.unwrap_or_default().trim().to_string();

    if target.is_empty() {
        return Ok(ResolveResult {
            ok: false,
            target_domain: String::new(),
            dryrun_domain: explicit_dryrun,
            phishlet: explicit_phishlet,
            upstream_domain: String::new(),
            needs_generate: false,
            error: Some("Enter the real website URL or hostname.".into()),
        });
    }

    let resolved_dryrun = if explicit_dryrun.is_empty() {
        default_dryrun_domain(&target)
    } else {
        explicit_dryrun
    };
    let upstream = upstream_domain(&target);
    let suggested = phishlet_name_for_target(&target);

    let resolved_phishlet = if !explicit_phishlet.is_empty() {
        explicit_phishlet
    } else {
        // find matching
        let preferred = phishlets_dir.join(format!("{suggested}.yaml"));
        if phishlet_is_valid(&preferred) {
            suggested.clone()
        } else {
            let mut found = None;
            if let Ok(rd) = std::fs::read_dir(&phishlets_dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.extension().and_then(|x| x.to_str()) != Some("yaml") {
                        continue;
                    }
                    if p.file_name().and_then(|x| x.to_str()) == Some("generic.yaml") {
                        continue;
                    }
                    if !phishlet_is_valid(&p) {
                        continue;
                    }
                    if let Ok(text) = std::fs::read_to_string(&p) {
                        if text.contains(&upstream) || text.contains(&target) {
                            found = p.file_stem().map(|s| s.to_string_lossy().to_string());
                            break;
                        }
                    }
                }
            }
            found.unwrap_or(suggested.clone())
        }
    };

    let path = phishlets_dir.join(format!("{resolved_phishlet}.yaml"));
    if !phishlet_is_valid(&path) {
        let err = format!("No ready phishlet for '{target}'. Generate or import one.");
        return Ok(ResolveResult {
            ok: false,
            target_domain: target,
            dryrun_domain: resolved_dryrun,
            phishlet: resolved_phishlet,
            upstream_domain: upstream,
            needs_generate: true,
            error: Some(err),
        });
    }

    Ok(ResolveResult {
        ok: true,
        target_domain: target,
        dryrun_domain: resolved_dryrun,
        phishlet: resolved_phishlet,
        upstream_domain: upstream,
        needs_generate: false,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn normalize_strips_scheme_path_and_port() {
        assert_eq!(
            normalize_target_host("https://App.Example.com:8443/login"),
            "app.example.com"
        );
    }

    #[test]
    fn slugify_and_phishlet_naming() {
        assert_eq!(slugify("My App!!"), "my-app");
        assert_eq!(phishlet_name_for_target("app.acme.com"), "app-acme-com");
    }

    #[test]
    fn dryrun_and_landing_sub() {
        assert_eq!(upstream_domain("app.acme.com"), "acme.com");
        assert_eq!(default_dryrun_domain("app.acme.com"), "acme.phishkit");
        assert_eq!(landing_sub("app.acme.com"), "app");
        assert_eq!(landing_sub("acme.com"), "");
    }

    #[test]
    fn upstream_domain_handles_apex_and_subdomains() {
        assert_eq!(upstream_domain("acme.com"), "acme.com");
        assert_eq!(upstream_domain("portal.app.acme.com"), "acme.com");
    }

    #[test]
    fn upstream_domain_handles_single_label_internal_host() {
        assert_eq!(upstream_domain("intranet"), "intranet");
        assert_eq!(upstream_domain("localhost"), "localhost");
        assert_eq!(landing_sub("intranet"), "");
    }

    #[test]
    fn upstream_domain_handles_compound_public_suffixes() {
        assert_eq!(upstream_domain("app.acme.co.uk"), "acme.co.uk");
        assert_eq!(upstream_domain("acme.co.uk"), "acme.co.uk");
        assert_eq!(upstream_domain("portal.login.acme.com.au"), "acme.com.au");
        assert_eq!(upstream_domain("tenant.github.io"), "tenant.github.io");
        assert_eq!(
            upstream_domain("app.tenant.vercel.app"),
            "tenant.vercel.app"
        );
        assert_eq!(landing_sub("app.acme.co.uk"), "app");
    }

    #[test]
    fn combine_host_apex_and_sub() {
        assert_eq!(combine_host("", "acme.com"), "acme.com");
        assert_eq!(combine_host("app", "acme.com"), "app.acme.com");
        assert_eq!(combine_host("  ", "acme.com"), "acme.com");
    }

    #[test]
    fn landing_hosts_parse_inline_proxy_hosts() {
        let yaml = r#"
min_ver: '3.0.0'
proxy_hosts:
  - {phish_sub: '', orig_sub: 'app', domain: 'cookie-demo.test', session: true, is_landing: true, auto_filter: true}
  - {phish_sub: 'api', orig_sub: 'api', domain: 'cookie-demo.test', session: true, is_landing: false}
"#;
        let hosts = landing_hosts_in_yaml(yaml);
        assert_eq!(hosts, vec!["cookie-demo.test".to_string()]);

        let collide = r#"
proxy_hosts:
  - {phish_sub: 'demo-cookie', orig_sub: 'demo-cookie', domain: 'local.phishkit', session: true, is_landing: true}
"#;
        assert_eq!(
            landing_hosts_in_yaml(collide),
            vec!["demo-cookie.local.phishkit".to_string()]
        );
    }

    #[test]
    fn landing_hosts_ignore_comments() {
        let yaml = r#"
# - {phish_sub: 'ghost', domain: 'commented.test', is_landing: true}
proxy_hosts:
  - {phish_sub: '', domain: 'real.test', is_landing: true}
"#;
        assert_eq!(landing_hosts_in_yaml(yaml), vec!["real.test".to_string()]);
    }

    #[test]
    fn find_reusable_phishlet_detects_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(
            dir.join("demo-cookie.yaml"),
            "proxy_hosts:\n  - {phish_sub: '', orig_sub: '', domain: 'demo-cookie.local.phishkit', is_landing: true}\n",
        )
        .unwrap();
        fs::write(
            dir.join("other.yaml"),
            "proxy_hosts:\n  - {phish_sub: '', domain: 'elsewhere.test', is_landing: true}\n",
        )
        .unwrap();

        // A new target whose landing host collides with demo-cookie should reuse it.
        assert_eq!(
            find_reusable_phishlet(
                dir,
                "demo-cookie.local.phishkit",
                "demo-cookie-local-phishkit"
            ),
            Some("demo-cookie".to_string())
        );
        // The phishlet we are (re)generating itself is skipped.
        assert_eq!(
            find_reusable_phishlet(dir, "demo-cookie.local.phishkit", "demo-cookie"),
            None
        );
        // A target with no collision gets no reuse.
        assert_eq!(
            find_reusable_phishlet(dir, "fresh.example.com", "fresh-example-com"),
            None
        );
    }
}
