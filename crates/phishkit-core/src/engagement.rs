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

pub fn upstream_domain(target_host: &str) -> String {
    let host = normalize_target_host(target_host);
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 3 {
        parts[parts.len() - 2..].join(".")
    } else {
        host
    }
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
}
