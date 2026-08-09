use std::process::{Command, Stdio};

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::kit::kit_root;

#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    pub evilginx_running: bool,
    pub evilginx_pid: Option<u32>,
}

fn port_pids(port: u16) -> Vec<u32> {
    let out = Command::new("lsof")
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-t"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let Ok(out) = out else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect()
}

fn pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn read_pid_file(path: &std::path::Path) -> Option<u32> {
    let s = std::fs::read_to_string(path).ok()?;
    s.trim().parse().ok()
}

fn screen_session_up(name: &str) -> bool {
    let out = Command::new("screen")
        .args(["-ls"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let Ok(out) = out else {
        return false;
    };
    String::from_utf8_lossy(&out.stdout).contains(&format!(".{name}"))
}

fn pgrep_evilginx(data_dir: &std::path::Path) -> Option<u32> {
    let needle = format!("evilginx.*{}", data_dir.display());
    let out = Command::new("pgrep")
        .args(["-f", &needle])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .next()
}

/// Detect background evilginx (screen session, process, or HTTPS listen).
pub fn evilginx_is_running(root: &std::path::Path) -> (bool, Option<u32>) {
    let data_dir =
        crate::setup::evilginx_data_dir().unwrap_or_else(|_| root.join("kit/evilginx/run/data"));
    let pid_file = crate::setup::evilginx_pid_path()
        .unwrap_or_else(|_| root.join("kit/evilginx/run/evilginx.pid"));

    if let Some(pid) = read_pid_file(&pid_file).filter(|p| pid_alive(*p)) {
        return (true, Some(pid));
    }
    if let Some(pid) = pgrep_evilginx(&data_dir) {
        let _ = std::fs::write(&pid_file, format!("{pid}\n"));
        return (true, Some(pid));
    }
    // Legacy kit-tree runtime
    let legacy_data = root.join("kit/evilginx/run/data");
    if legacy_data != data_dir {
        if let Some(pid) = pgrep_evilginx(&legacy_data) {
            return (true, Some(pid));
        }
    }
    if screen_session_up("phishkit-evilginx") {
        return (true, None);
    }
    let port_pid = port_pids(443).into_iter().chain(port_pids(8443)).next();
    if let Some(pid) = port_pid {
        return (true, Some(pid));
    }
    (false, None)
}

pub fn service_status() -> AppResult<ServiceStatus> {
    let root = kit_root()?;
    let (evilginx_running, evilginx_pid) = evilginx_is_running(&root);

    Ok(ServiceStatus {
        evilginx_running,
        evilginx_pid,
    })
}

pub fn stop_evilginx() -> AppResult<String> {
    let root = kit_root()?;
    let script = root.join("kit/evilginx/scripts/stop_evilginx.sh");
    if !script.is_file() {
        return Err(AppError::msg(format!(
            "Script missing: {}",
            script.display()
        )));
    }
    let data = crate::setup::evilginx_data_dir()?;
    let log = crate::setup::evilginx_log_path()?;
    let pid = crate::setup::evilginx_pid_path()?;
    let out = Command::new("bash")
        .arg(&script)
        .current_dir(&root)
        .env("PHISHKIT_ROOT", &root)
        .env("KIT_ROOT", &root)
        .env("EVILGINX_DATA_DIR", &data)
        .env("EVILGINX_LOG", &log)
        .env("EVILGINX_PID", &pid)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(AppError::msg(format!(
            "stop_evilginx.sh failed:\n{stderr}\n{stdout}"
        )));
    }
    Ok(if stdout.is_empty() { stderr } else { stdout })
}

pub fn build_binaries() -> AppResult<String> {
    let root = kit_root()?;
    let out = Command::new("make")
        .arg("build")
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(AppError::msg(format!(
            "make build failed:\n{stderr}\n{stdout}"
        )));
    }
    Ok(stdout)
}
