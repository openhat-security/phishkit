use serde::Serialize;

use crate::db;
use crate::engagement;
use crate::error::{AppError, AppResult};
use crate::hosts;
use crate::kit::{kit_info, kit_root};
use crate::lure_ops::{self, CaTrustInfo};
use crate::services;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessCheck {
    pub id: String,
    pub ok: bool,
    pub label: String,
    pub detail: String,
    pub fix_hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetReadiness {
    pub profile_id: String,
    pub binary_ok: bool,
    pub phishlet_ok: bool,
    pub hosts_ok: bool,
    pub ca_ok: bool,
    pub proxy_running: bool,
    pub runtime_is_this_profile: bool,
    pub default_lure_url: String,
    pub notes: Vec<String>,
    pub checks: Vec<ReadinessCheck>,
}

fn push_check(
    checks: &mut Vec<ReadinessCheck>,
    id: &str,
    ok: bool,
    label: &str,
    detail: &str,
    fix_hint: &str,
) {
    checks.push(ReadinessCheck {
        id: id.into(),
        ok,
        label: label.into(),
        detail: detail.into(),
        fix_hint: fix_hint.into(),
    });
}

pub fn target_readiness(profile_id: String) -> AppResult<TargetReadiness> {
    let profile =
        db::get_profile(&profile_id)?.ok_or_else(|| AppError::msg("profile not found"))?;

    let kit = kit_info()?;
    let binary_ok = kit.evilginx_bin;

    let root = kit_root()?;
    let phishlet_path = if profile.phishlet.is_empty() {
        std::path::PathBuf::new()
    } else {
        root.join(format!("kit/evilginx/phishlets/{}.yaml", profile.phishlet))
    };
    let phishlet_ok = engagement::phishlet_is_valid(&phishlet_path);

    let hosts = hosts::hosts_status(
        profile.dryrun_domain.clone(),
        Some(profile.phishlet.clone()),
    )?;
    let hosts_ok = hosts.hosts_ok;

    let ca: CaTrustInfo = lure_ops::ca_trust_info()?;
    let ca_ok = ca.exists;

    let service = services::service_status()?;
    let proxy_running = service.evilginx_running;
    let runtime_is_this_profile = db::get_runtime_profile_id()?
        .map(|id| id == profile_id)
        .unwrap_or(false);

    let default_lure_url = lure_ops::get_default_lure(&profile_id)?
        .map(|l| l.lure_url)
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| profile.lure_url.clone());

    let mut checks = Vec::new();
    push_check(
        &mut checks,
        "binary",
        binary_ok,
        "Evilginx binary",
        if binary_ok {
            "Built and present under kit/evilginx/run/"
        } else {
            "evilginx binary missing"
        },
        "Advanced → Build binaries (or make build-evilginx)",
    );
    let phishlet_detail = if phishlet_ok {
        format!("Valid phishlet: {}", profile.phishlet)
    } else if profile.phishlet.is_empty() {
        "No phishlet assigned to this target".to_string()
    } else {
        format!("Missing or invalid: {}", phishlet_path.display())
    };
    push_check(
        &mut checks,
        "phishlet",
        phishlet_ok,
        "Phishlet YAML",
        &phishlet_detail,
        "Ensure destination / scaffold phishlet for this target",
    );
    let hosts_detail = if hosts_ok {
        "All required dry-run hostnames resolve locally".to_string()
    } else {
        format!("Missing {} host line(s)", hosts.missing_lines.len())
    };
    push_check(
        &mut checks,
        "hosts",
        hosts_ok,
        "/etc/hosts entries",
        &hosts_detail,
        "Use Fix hosts in the target workflow",
    );
    push_check(
        &mut checks,
        "ca",
        ca_ok,
        "Local CA certificate",
        if ca_ok {
            "CA cert file exists on disk"
        } else {
            "CA not minted yet — start proxy once"
        },
        "Start lure/proxy, then trust the CA (open_ca_cert)",
    );
    push_check(
        &mut checks,
        "proxy",
        proxy_running,
        "Proxy process",
        if proxy_running {
            "evilginx appears to be running"
        } else {
            "No evilginx process detected on port 443"
        },
        "Start lure for this target",
    );
    push_check(
        &mut checks,
        "runtime",
        runtime_is_this_profile,
        "Runtime profile binding",
        if runtime_is_this_profile {
            "This target owns the active proxy session"
        } else if proxy_running {
            "Another target may own the running proxy"
        } else {
            "Proxy not running for this target"
        },
        "Stop other targets and Start lure here",
    );

    let mut notes = Vec::new();
    if !ca.already_installed && ca_ok {
        notes.push("Trust the evilginx CA in your browser keychain before testing.".into());
    }
    if default_lure_url.is_empty() {
        notes.push("No default lure URL yet — start the proxy to generate one.".into());
    }
    if !hosts_ok {
        notes.push(format!(
            "Add missing hosts: {}",
            hosts.missing_lines.join("; ")
        ));
    }

    Ok(TargetReadiness {
        profile_id,
        binary_ok,
        phishlet_ok,
        hosts_ok,
        ca_ok,
        proxy_running,
        runtime_is_this_profile,
        default_lure_url,
        notes,
        checks,
    })
}
