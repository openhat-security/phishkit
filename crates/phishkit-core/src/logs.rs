use std::fs;
use std::path::PathBuf;

use crate::error::AppResult;
use crate::setup;

fn resolve_log_path() -> AppResult<PathBuf> {
    let primary = setup::evilginx_log_path()?;
    if primary.is_file() {
        return Ok(primary);
    }
    if let Ok(root) = crate::kit::kit_root() {
        let legacy = root.join("kit/evilginx/run/evilginx.log");
        if legacy.is_file() {
            return Ok(legacy);
        }
    }
    Ok(primary)
}

pub fn tail_evilginx_log(lines: Option<usize>) -> AppResult<String> {
    let path = resolve_log_path()?;
    if !path.is_file() {
        return Ok("(no evilginx.log yet)".into());
    }
    let text = fs::read_to_string(path)?;
    let n = lines.unwrap_or(80);
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(n);
    Ok(all[start..].join("\n"))
}
