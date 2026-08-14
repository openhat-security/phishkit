use std::fs;
#[cfg(target_os = "macos")]
use std::process::Command;

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::kit::kit_root;

#[derive(Debug, Serialize)]
pub struct HostEntry {
    pub line: String,
    pub fqdn: String,
    pub present: bool,
}

#[derive(Debug, Serialize)]
pub struct HostsStatus {
    pub hosts_ok: bool,
    pub dryrun_domain: String,
    pub landing_host: String,
    pub entries: Vec<HostEntry>,
    pub missing_lines: Vec<String>,
    pub uses_native_prompt: bool,
    pub platform: String,
}

/// Landing phish_sub from phishlet YAML ('' for apex sites). No portal default.
fn landing_sub(phishlet: &str) -> String {
    if phishlet.is_empty() {
        return String::new();
    }
    let root = kit_root().ok();
    if let Some(root) = root {
        let path = root.join(format!("kit/evilginx/phishlets/{phishlet}.yaml"));
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                if line.contains("is_landing: true") || line.contains("is_landing:true") {
                    if let Some(idx) = line.find("phish_sub:") {
                        let rest = &line[idx + "phish_sub:".len()..];
                        let sub = rest
                            .trim()
                            .trim_matches(|c| c == '\'' || c == '"' || c == ',' || c == ' ');
                        let sub = sub.split(',').next().unwrap_or(sub).trim_matches(|c| {
                            c == '\'' || c == '"' || c == ' ' || c == '{' || c == '}'
                        });
                        return sub.to_string();
                    }
                }
            }
            let re = regex::Regex::new(r"phish_sub:\s*'([^']*)'[^\}]*is_landing:\s*true").ok();
            if let Some(re) = re {
                if let Some(c) = re.captures(&text) {
                    return c[1].to_string();
                }
            }
        }
    }
    String::new()
}

/// All phish_sub values from the phishlet (apex '', www, api, services, …).
fn phish_subs(phishlet: &str) -> Vec<String> {
    let mut out = Vec::new();
    if phishlet.is_empty() {
        return out;
    }
    let Ok(root) = kit_root() else {
        return out;
    };
    let path = root.join(format!("kit/evilginx/phishlets/{phishlet}.yaml"));
    let Ok(text) = fs::read_to_string(path) else {
        return out;
    };
    let re = match regex::Regex::new(r"phish_sub:\s*'([^']*)'") {
        Ok(r) => r,
        Err(_) => return out,
    };
    for cap in re.captures_iter(&text) {
        out.push(cap[1].to_string());
    }
    out.sort();
    out.dedup();
    out
}

fn required_hosts(dryrun: &str, phishlet: &str) -> Vec<(String, String)> {
    let mut hosts = vec![(format!("127.0.0.1   {dryrun}"), dryrun.to_string())];
    let subs = phish_subs(phishlet);
    if subs.is_empty() {
        let landing = landing_sub(phishlet);
        if !landing.is_empty() {
            let fqdn = format!("{landing}.{dryrun}");
            hosts.push((format!("127.0.0.1   {fqdn}"), fqdn));
        }
        hosts.push((format!("127.0.0.1   api.{dryrun}"), format!("api.{dryrun}")));
    } else {
        for sub in subs {
            let fqdn = if sub.is_empty() {
                dryrun.to_string()
            } else {
                format!("{sub}.{dryrun}")
            };
            hosts.push((format!("127.0.0.1   {fqdn}"), fqdn));
        }
    }
    hosts.sort_by(|a, b| a.1.cmp(&b.1));
    hosts.dedup_by(|a, b| a.1 == b.1);
    hosts
}

fn etc_hosts() -> AppResult<String> {
    fs::read_to_string("/etc/hosts").map_err(|e| AppError::msg(format!("read /etc/hosts: {e}")))
}

/// FQDNs a target's dryrun+phishlet would have added to /etc/hosts.
pub fn required_fqdns(dryrun: &str, phishlet: &str) -> Vec<String> {
    required_hosts(dryrun, phishlet)
        .into_iter()
        .map(|(_, fqdn)| fqdn)
        .collect()
}

/// Remove the 127.0.0.1 lines phishkit added for the given FQDNs. Only deletes
/// lines of the exact form we write (`127.0.0.1<ws>FQDN`), so user-authored
/// multi-host entries are never clobbered. Uses an admin prompt on macOS.
pub fn remove_fqdns(fqdns: Vec<String>) -> AppResult<serde_json::Value> {
    let text = etc_hosts().unwrap_or_default();
    let present: Vec<String> = fqdns
        .into_iter()
        .filter(|f| !f.trim().is_empty() && fqdn_present(&text, f))
        .collect();
    if present.is_empty() {
        return Ok(serde_json::json!({"ok": true, "removed": 0, "already": true}));
    }

    #[cfg(target_os = "macos")]
    {
        let mut cmd = String::from("/usr/bin/sed -i '' ");
        for f in &present {
            let esc = f.replace('.', "\\.");
            cmd.push_str(&format!(
                "-e '/^127\\.0\\.0\\.1[[:space:]][[:space:]]*{esc}[[:space:]]*$/d' "
            ));
        }
        cmd.push_str("/etc/hosts");
        let script = format!(
            "do shell script {} with administrator privileges",
            serde_json::to_string(&cmd).unwrap()
        );
        let out = Command::new("osascript").args(["-e", &script]).output()?;
        if out.status.success() {
            return Ok(serde_json::json!({
                "ok": true,
                "removed": present.len(),
                "fqdns": present,
            }));
        }
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        Ok(serde_json::json!({
            "ok": false,
            "need_password": true,
            "stderr": err,
            "fqdns": present,
        }))
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(serde_json::json!({
            "ok": false,
            "manual": true,
            "message": "Remove these 127.0.0.1 FQDN lines from /etc/hosts with sudo",
            "fqdns": present,
        }))
    }
}

fn fqdn_present(hosts_text: &str, fqdn: &str) -> bool {
    for line in hosts_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1..].contains(&fqdn) {
            return true;
        }
    }
    false
}

pub fn hosts_status(dryrun_domain: String, phishlet: Option<String>) -> AppResult<HostsStatus> {
    let dryrun = dryrun_domain.trim().to_string();
    if dryrun.is_empty() {
        return Err(AppError::msg("dryrun_domain required"));
    }
    let phishlet = phishlet.unwrap_or_default();
    let text = etc_hosts().unwrap_or_default();
    let req = required_hosts(&dryrun, &phishlet);
    let landing = landing_sub(&phishlet);
    let landing_host = if landing.is_empty() {
        dryrun.clone()
    } else {
        format!("{landing}.{dryrun}")
    };
    let mut entries = Vec::new();
    let mut missing = Vec::new();
    for (line, fqdn) in req {
        let present = fqdn_present(&text, &fqdn);
        if !present {
            missing.push(line.clone());
        }
        entries.push(HostEntry {
            line,
            fqdn,
            present,
        });
    }
    Ok(HostsStatus {
        hosts_ok: missing.is_empty(),
        dryrun_domain: dryrun,
        landing_host,
        entries,
        missing_lines: missing,
        uses_native_prompt: cfg!(target_os = "macos"),
        platform: std::env::consts::OS.to_string(),
    })
}

pub fn hosts_fix(dryrun_domain: String, phishlet: Option<String>) -> AppResult<serde_json::Value> {
    let status = hosts_status(dryrun_domain.clone(), phishlet.clone())?;
    if status.hosts_ok {
        return Ok(serde_json::json!({"ok": true, "already": true, "hosts_ok": true}));
    }
    let missing = status.missing_lines;

    #[cfg(target_os = "macos")]
    {
        let mut parts = Vec::new();
        for line in &missing {
            let fqdn = line.split_whitespace().last().unwrap_or("");
            parts.push(format!(
                "grep -E '(^|[[:space:]]){}([[:space:]]|$)' /etc/hosts >/dev/null || printf '%s\\n' '{}' >> /etc/hosts",
                fqdn.replace('.', r"\."),
                line.replace('\'', r"'\''")
            ));
        }
        let shell_cmd = parts.join(" && ");
        let script = format!(
            "do shell script {} with administrator privileges",
            serde_json::to_string(&shell_cmd).unwrap()
        );
        let out = Command::new("osascript").args(["-e", &script]).output()?;
        if out.status.success() {
            let again = hosts_status(dryrun_domain, phishlet)?;
            if again.hosts_ok {
                return Ok(
                    serde_json::json!({"ok": true, "method": "osascript", "hosts_ok": true}),
                );
            }
        }
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        Ok(serde_json::json!({
            "ok": false,
            "need_password": true,
            "stderr": err,
            "manual_lines": missing,
        }))
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(serde_json::json!({
            "ok": false,
            "need_password": true,
            "stderr": "Add these lines to /etc/hosts with sudo",
            "manual_lines": missing,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_fqdns_always_includes_apex() {
        let fqdns = required_fqdns("acme.phishkit", "");
        assert!(fqdns.iter().any(|f| f == "acme.phishkit"));
    }
}
