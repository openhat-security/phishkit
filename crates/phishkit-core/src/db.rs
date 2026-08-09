use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Utc;
use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::setup;

static DB_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static PROFILE_ID_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-z0-9]([a-z0-9-]*[a-z0-9])?$").unwrap());

const SCHEMA_VERSION: i32 = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub phishlet: String,
    pub dryrun_domain: String,
    pub target_domain: String,
    pub lure_url: String,
    pub auth_meta: Value,
    pub stack_info: Option<Value>,
    pub notes: String,
    pub assessment_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRow {
    pub id: i64,
    pub profile_id: String,
    pub evilginx_session_id: i64,
    pub data: Value,
    pub evilginx_create_time: Option<i64>,
    pub evilginx_update_time: Option<i64>,
    pub synced_at: String,
}

fn db_path() -> AppResult<PathBuf> {
    let preferred = setup::db_file_path()?;
    setup::migrate_legacy_db_if_needed(&preferred)?;
    if let Some(parent) = preferred.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(preferred)
}

fn connect() -> AppResult<Connection> {
    let path = db_path()?;
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

fn slug(name: &str) -> String {
    let s: String = name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').chars().take(64).collect::<String>();
    if s.is_empty() {
        "profile".into()
    } else {
        s
    }
}

pub fn normalize_profile_id(profile_id: Option<&str>, name: Option<&str>) -> AppResult<String> {
    let raw = profile_id.unwrap_or("").trim();
    let raw = if raw.is_empty() {
        name.unwrap_or("profile")
    } else {
        raw
    };
    let pid = slug(raw);
    if !PROFILE_ID_RE.is_match(&pid) {
        return Err(AppError::msg(format!("invalid profile id: {pid}")));
    }
    Ok(pid)
}

fn init_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS app_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            phishlet TEXT NOT NULL DEFAULT '',
            dryrun_domain TEXT NOT NULL DEFAULT 'test-phish.local.phishkit',
            target_domain TEXT NOT NULL DEFAULT '',
            lure_url TEXT NOT NULL DEFAULT '',
            auth_meta TEXT NOT NULL DEFAULT '{}',
            stack_info TEXT,
            notes TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS captured_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            profile_id TEXT NOT NULL,
            evilginx_session_id INTEGER NOT NULL UNIQUE,
            data TEXT NOT NULL,
            evilginx_create_time INTEGER,
            evilginx_update_time INTEGER,
            synced_at TEXT NOT NULL,
            FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_captured_sessions_profile
            ON captured_sessions(profile_id);
        CREATE TABLE IF NOT EXISTS ignored_sessions (
            profile_id TEXT NOT NULL,
            evilginx_session_id INTEGER NOT NULL,
            ignored_at TEXT NOT NULL,
            PRIMARY KEY (profile_id, evilginx_session_id),
            FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS smtp_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            host TEXT NOT NULL DEFAULT '',
            port INTEGER NOT NULL DEFAULT 587,
            username TEXT NOT NULL DEFAULT '',
            password TEXT NOT NULL DEFAULT '',
            from_email TEXT NOT NULL DEFAULT '',
            from_name TEXT NOT NULL DEFAULT '',
            use_starttls INTEGER NOT NULL DEFAULT 1,
            provider TEXT NOT NULL DEFAULT 'smtp',
            api_key TEXT NOT NULL DEFAULT '',
            region TEXT NOT NULL DEFAULT '',
            domain TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT ''
        );
        INSERT OR IGNORE INTO smtp_settings(id, updated_at) VALUES(1, '');
        CREATE TABLE IF NOT EXISTS mail_accounts (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            provider TEXT NOT NULL DEFAULT 'smtp',
            host TEXT NOT NULL DEFAULT '',
            port INTEGER NOT NULL DEFAULT 587,
            username TEXT NOT NULL DEFAULT '',
            password TEXT NOT NULL DEFAULT '',
            from_email TEXT NOT NULL DEFAULT '',
            from_name TEXT NOT NULL DEFAULT '',
            use_starttls INTEGER NOT NULL DEFAULT 1,
            api_key TEXT NOT NULL DEFAULT '',
            region TEXT NOT NULL DEFAULT '',
            domain TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS email_templates (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            subject TEXT NOT NULL DEFAULT '',
            html_body TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS recipient_lists (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS recipients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            list_id TEXT NOT NULL,
            email TEXT NOT NULL,
            first_name TEXT NOT NULL DEFAULT '',
            last_name TEXT NOT NULL DEFAULT '',
            extras TEXT NOT NULL DEFAULT '{}',
            suppressed INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (list_id) REFERENCES recipient_lists(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_recipients_list ON recipients(list_id);
        CREATE TABLE IF NOT EXISTS campaigns (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            template_id TEXT NOT NULL,
            list_id TEXT NOT NULL,
            link_url TEXT NOT NULL DEFAULT '',
            profile_id TEXT NOT NULL DEFAULT '',
            rate_per_minute INTEGER NOT NULL DEFAULT 30,
            status TEXT NOT NULL DEFAULT 'draft',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            started_at TEXT,
            finished_at TEXT
        );
        CREATE TABLE IF NOT EXISTS campaign_attempts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            campaign_id TEXT NOT NULL,
            recipient_id INTEGER NOT NULL,
            email TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            error TEXT NOT NULL DEFAULT '',
            sent_at TEXT,
            FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_campaign_attempts_campaign
            ON campaign_attempts(campaign_id);
        "#,
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO app_meta(key, value) VALUES('schema_version', ?1)",
        params![SCHEMA_VERSION.to_string()],
    )?;
    migrate(conn)?;
    Ok(())
}

pub(crate) fn table_columns(conn: &Connection, table: &str) -> AppResult<Vec<String>> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub(crate) fn ensure_column(conn: &Connection, table: &str, col: &str, ddl: &str) -> AppResult<()> {
    let cols = table_columns(conn, table)?;
    if cols.is_empty() || cols.iter().any(|c| c == col) {
        return Ok(());
    }
    conn.execute(ddl, [])?;
    Ok(())
}

pub(crate) fn get_meta(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    conn.query_row(
        "SELECT value FROM app_meta WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn set_meta(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO app_meta(key, value) VALUES(?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_active_assessment_id() -> AppResult<Option<String>> {
    with_db(|conn| get_meta(conn, "active_assessment").map(|v| v.filter(|s| !s.is_empty())))
}

pub(crate) fn set_active_assessment(conn: &Connection, id: &str) -> AppResult<()> {
    set_meta(conn, "active_assessment", id)
}

pub(crate) fn clear_active_assessment(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM app_meta WHERE key = 'active_assessment'", [])?;
    Ok(())
}

pub fn get_runtime_profile_id() -> AppResult<Option<String>> {
    with_db(|conn| get_meta(conn, "runtime_profile").map(|v| v.filter(|s| !s.is_empty())))
}

pub fn set_runtime_profile(id: &str) -> AppResult<()> {
    with_db(|conn| set_meta(conn, "runtime_profile", id))
}

pub fn clear_runtime_profile() -> AppResult<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM app_meta WHERE key = 'runtime_profile'", [])?;
        Ok(())
    })
}

pub fn insert_proxy_run(
    profile_id: &str,
    phishlet: &str,
    dryrun_domain: &str,
    lure_config_json: &str,
) -> AppResult<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    with_db(|conn| {
        conn.execute(
            "INSERT INTO proxy_runs(id, profile_id, phishlet, dryrun_domain, started_at, lure_config_json)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![id, profile_id, phishlet, dryrun_domain, now, lure_config_json],
        )?;
        Ok(())
    })?;
    Ok(id)
}

pub fn stop_open_proxy_runs(profile_id: &str) -> AppResult<()> {
    if profile_id.trim().is_empty() {
        return Ok(());
    }
    let now = now_iso();
    with_db(|conn| {
        conn.execute(
            "UPDATE proxy_runs SET stopped_at = ?1 WHERE profile_id = ?2 AND stopped_at IS NULL",
            params![now, profile_id],
        )?;
        Ok(())
    })
}

fn migrate_v8_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS assessments (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          primary_domain TEXT NOT NULL DEFAULT '',
          authorization_ref TEXT NOT NULL DEFAULT '',
          authorized_by TEXT NOT NULL DEFAULT '',
          authorized_at TEXT,
          notes TEXT NOT NULL DEFAULT '',
          status TEXT NOT NULL DEFAULT 'active',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS assessment_scopes (
          id TEXT PRIMARY KEY,
          assessment_id TEXT NOT NULL,
          scope_type TEXT NOT NULL DEFAULT 'domain',
          value TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY (assessment_id) REFERENCES assessments(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS lures (
          id TEXT PRIMARY KEY,
          profile_id TEXT NOT NULL,
          name TEXT NOT NULL,
          path TEXT NOT NULL DEFAULT '',
          lure_url TEXT NOT NULL DEFAULT '',
          redirect_url TEXT NOT NULL DEFAULT '',
          redirector TEXT NOT NULL DEFAULT '',
          ua_filter TEXT NOT NULL DEFAULT '',
          og_title TEXT NOT NULL DEFAULT '',
          og_desc TEXT NOT NULL DEFAULT '',
          og_image TEXT NOT NULL DEFAULT '',
          og_url TEXT NOT NULL DEFAULT '',
          paused INTEGER NOT NULL DEFAULT 0,
          is_default INTEGER NOT NULL DEFAULT 0,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_lures_profile_path ON lures(profile_id, path) WHERE path != '';
        CREATE INDEX IF NOT EXISTS idx_lures_profile ON lures(profile_id);

        CREATE TABLE IF NOT EXISTS proxy_runs (
          id TEXT PRIMARY KEY,
          profile_id TEXT NOT NULL,
          phishlet TEXT NOT NULL DEFAULT '',
          dryrun_domain TEXT NOT NULL DEFAULT '',
          started_at TEXT NOT NULL,
          stopped_at TEXT,
          lure_config_json TEXT NOT NULL DEFAULT '[]',
          FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
        );
        "#,
    )?;

    ensure_column(
        conn,
        "profiles",
        "assessment_id",
        "ALTER TABLE profiles ADD COLUMN assessment_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "email_templates",
        "assessment_id",
        "ALTER TABLE email_templates ADD COLUMN assessment_id TEXT",
    )?;
    ensure_column(
        conn,
        "recipient_lists",
        "assessment_id",
        "ALTER TABLE recipient_lists ADD COLUMN assessment_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "campaigns",
        "assessment_id",
        "ALTER TABLE campaigns ADD COLUMN assessment_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "campaigns",
        "lure_id",
        "ALTER TABLE campaigns ADD COLUMN lure_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "campaigns",
        "sender_account_id",
        "ALTER TABLE campaigns ADD COLUMN sender_account_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "campaigns",
        "snapshot_json",
        "ALTER TABLE campaigns ADD COLUMN snapshot_json TEXT NOT NULL DEFAULT '{}'",
    )?;
    ensure_column(
        conn,
        "campaign_attempts",
        "tracking_token",
        "ALTER TABLE campaign_attempts ADD COLUMN tracking_token TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "campaign_attempts",
        "tracked_url",
        "ALTER TABLE campaign_attempts ADD COLUMN tracked_url TEXT NOT NULL DEFAULT ''",
    )?;
    // Campaign engine: scheduling + delivery/open/click/bounce events.
    ensure_column(
        conn,
        "campaigns",
        "mode",
        "ALTER TABLE campaigns ADD COLUMN mode TEXT NOT NULL DEFAULT 'aitm'",
    )?;
    ensure_column(
        conn,
        "campaigns",
        "scheduled_at",
        "ALTER TABLE campaigns ADD COLUMN scheduled_at TEXT",
    )?;
    ensure_column(
        conn,
        "campaigns",
        "send_window_start",
        "ALTER TABLE campaigns ADD COLUMN send_window_start TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "campaigns",
        "send_window_end",
        "ALTER TABLE campaigns ADD COLUMN send_window_end TEXT NOT NULL DEFAULT ''",
    )?;
    for (col, ddl) in [
        (
            "provider_message_id",
            "ALTER TABLE campaign_attempts ADD COLUMN provider_message_id TEXT NOT NULL DEFAULT ''",
        ),
        (
            "delivered_at",
            "ALTER TABLE campaign_attempts ADD COLUMN delivered_at TEXT",
        ),
        (
            "opened_at",
            "ALTER TABLE campaign_attempts ADD COLUMN opened_at TEXT",
        ),
        (
            "clicked_at",
            "ALTER TABLE campaign_attempts ADD COLUMN clicked_at TEXT",
        ),
        (
            "bounced_at",
            "ALTER TABLE campaign_attempts ADD COLUMN bounced_at TEXT",
        ),
        (
            "complained_at",
            "ALTER TABLE campaign_attempts ADD COLUMN complained_at TEXT",
        ),
        (
            "bounce_reason",
            "ALTER TABLE campaign_attempts ADD COLUMN bounce_reason TEXT NOT NULL DEFAULT ''",
        ),
    ] {
        ensure_column(conn, "campaign_attempts", col, ddl)?;
    }
    ensure_column(
        conn,
        "captured_sessions",
        "lure_id",
        "ALTER TABLE captured_sessions ADD COLUMN lure_id TEXT",
    )?;
    ensure_column(
        conn,
        "captured_sessions",
        "campaign_attempt_id",
        "ALTER TABLE captured_sessions ADD COLUMN campaign_attempt_id INTEGER",
    )?;

    Ok(())
}

fn migrate(conn: &Connection) -> AppResult<()> {
    // Ensure columns exist if DB was created by older Python schema
    let cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(profiles)")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        rows.filter_map(|r| r.ok()).collect()
    };
    if !cols.is_empty() {
        if !cols.iter().any(|c| c == "lure_url") {
            let _ = conn.execute(
                "ALTER TABLE profiles ADD COLUMN lure_url TEXT NOT NULL DEFAULT ''",
                [],
            );
        }
        if !cols.iter().any(|c| c == "auth_meta") {
            let _ = conn.execute(
                "ALTER TABLE profiles ADD COLUMN auth_meta TEXT NOT NULL DEFAULT '{}'",
                [],
            );
            // Best-effort migrate firebase_* into auth_meta
            if cols.iter().any(|c| c == "firebase_api_key") {
                let mut stmt = conn.prepare(
                    "SELECT id, firebase_api_key, COALESCE(firebase_key_source,'') FROM profiles",
                )?;
                let rows: Vec<(String, String, String)> = stmt
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                    .filter_map(|r| r.ok())
                    .collect();
                for (id, key, src) in rows {
                    if key.is_empty() {
                        continue;
                    }
                    let meta = serde_json::json!({
                        "firebase_api_key": key,
                        "firebase_key_source": src,
                    });
                    conn.execute(
                        "UPDATE profiles SET auth_meta = ?1 WHERE id = ?2",
                        params![meta.to_string(), id],
                    )?;
                }
            }
        }
        if !cols.iter().any(|c| c == "stack_info") {
            let _ = conn.execute("ALTER TABLE profiles ADD COLUMN stack_info TEXT", []);
        }
    }
    // Mail settings columns (schema v6)
    let smtp_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(smtp_settings)")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        rows.filter_map(|r| r.ok()).collect()
    };
    if !smtp_cols.is_empty() {
        for (col, ddl) in [
            (
                "provider",
                "ALTER TABLE smtp_settings ADD COLUMN provider TEXT NOT NULL DEFAULT 'smtp'",
            ),
            (
                "api_key",
                "ALTER TABLE smtp_settings ADD COLUMN api_key TEXT NOT NULL DEFAULT ''",
            ),
            (
                "region",
                "ALTER TABLE smtp_settings ADD COLUMN region TEXT NOT NULL DEFAULT ''",
            ),
            (
                "domain",
                "ALTER TABLE smtp_settings ADD COLUMN domain TEXT NOT NULL DEFAULT ''",
            ),
        ] {
            if !smtp_cols.iter().any(|c| c == col) {
                let _ = conn.execute(ddl, []);
            }
        }
    }
    // Ensure mail_accounts exists (older DBs)
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS mail_accounts (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            provider TEXT NOT NULL DEFAULT 'smtp',
            host TEXT NOT NULL DEFAULT '',
            port INTEGER NOT NULL DEFAULT 587,
            username TEXT NOT NULL DEFAULT '',
            password TEXT NOT NULL DEFAULT '',
            from_email TEXT NOT NULL DEFAULT '',
            from_name TEXT NOT NULL DEFAULT '',
            use_starttls INTEGER NOT NULL DEFAULT 1,
            api_key TEXT NOT NULL DEFAULT '',
            region TEXT NOT NULL DEFAULT '',
            domain TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    migrate_v8_schema(conn)?;

    let desktop_ver: i32 = get_meta(conn, "desktop_schema_version")?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if desktop_ver < 8 {
        crate::assessment::migrate_v8_assessments(conn)?;
        crate::lure_ops::migrate_v8_lures(conn)?;
        set_meta(conn, "desktop_schema_version", "8")?;
    }

    set_meta(conn, "schema_version", &SCHEMA_VERSION.to_string())?;
    Ok(())
}

pub(crate) fn with_db<T>(f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
    let _g = DB_LOCK.lock().unwrap();
    let conn = connect()?;
    init_schema(&conn)?;
    f(&conn)
}

pub(crate) fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub(crate) fn row_to_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<Profile> {
    let auth_meta_s: String = row.get("auth_meta")?;
    let stack_s: Option<String> = row.get("stack_info")?;
    let assessment_id: String = row.get::<_, String>("assessment_id").unwrap_or_default();
    Ok(Profile {
        id: row.get("id")?,
        name: row.get("name")?,
        phishlet: row.get("phishlet")?,
        dryrun_domain: row.get("dryrun_domain")?,
        target_domain: row.get("target_domain")?,
        lure_url: row.get("lure_url")?,
        auth_meta: serde_json::from_str(&auth_meta_s).unwrap_or(Value::Object(Default::default())),
        stack_info: stack_s.and_then(|s| serde_json::from_str(&s).ok()),
        notes: row.get("notes")?,
        assessment_id,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn list_profiles() -> AppResult<Vec<Profile>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, phishlet, dryrun_domain, target_domain, lure_url,
                    auth_meta, stack_info, notes, assessment_id, created_at, updated_at
             FROM profiles ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_profile)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    })
}

pub fn get_profile(id: &str) -> AppResult<Option<Profile>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, phishlet, dryrun_domain, target_domain, lure_url,
                    auth_meta, stack_info, notes, assessment_id, created_at, updated_at
             FROM profiles WHERE id = ?1",
        )?;
        let p = stmt.query_row(params![id], row_to_profile).optional()?;
        Ok(p)
    })
}

pub fn get_active_profile_id() -> AppResult<Option<String>> {
    with_db(|conn| {
        let v: Option<String> = conn
            .query_row(
                "SELECT value FROM app_meta WHERE key = 'active_profile'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        Ok(v)
    })
}

pub fn set_active_profile(id: &str) -> AppResult<()> {
    with_db(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO app_meta(key, value) VALUES('active_profile', ?1)",
            params![id],
        )?;
        Ok(())
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertProfile {
    pub id: Option<String>,
    pub name: String,
    pub phishlet: Option<String>,
    pub dryrun_domain: Option<String>,
    pub target_domain: Option<String>,
    pub lure_url: Option<String>,
    pub auth_meta: Option<Value>,
    pub stack_info: Option<Value>,
    pub notes: Option<String>,
    pub assessment_id: Option<String>,
}

pub fn upsert_profile(req: UpsertProfile) -> AppResult<Profile> {
    let id = normalize_profile_id(req.id.as_deref(), Some(&req.name))?;
    let now = now_iso();
    with_db(|conn| {
        let existing = conn
            .query_row(
                "SELECT created_at FROM profiles WHERE id = ?1",
                params![id],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        let created = existing.unwrap_or_else(|| now.clone());
        let auth_meta = req
            .auth_meta
            .unwrap_or(Value::Object(Default::default()))
            .to_string();
        let stack = req.stack_info.as_ref().map(|v| v.to_string());
        let existing_assessment: Option<String> = conn
            .query_row(
                "SELECT assessment_id FROM profiles WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?;
        let assessment_id = req
            .assessment_id
            .filter(|s| !s.is_empty())
            .or(existing_assessment)
            .unwrap_or_default();

        conn.execute(
            "INSERT INTO profiles(id, name, phishlet, dryrun_domain, target_domain, lure_url,
                                  auth_meta, stack_info, notes, assessment_id, created_at, updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name,
               phishlet=excluded.phishlet,
               dryrun_domain=excluded.dryrun_domain,
               target_domain=excluded.target_domain,
               lure_url=excluded.lure_url,
               auth_meta=excluded.auth_meta,
               stack_info=excluded.stack_info,
               notes=excluded.notes,
               assessment_id=CASE WHEN excluded.assessment_id != '' THEN excluded.assessment_id ELSE profiles.assessment_id END,
               updated_at=excluded.updated_at",
            params![
                id,
                req.name,
                req.phishlet.unwrap_or_default(),
                req.dryrun_domain
                    .unwrap_or_else(|| "test-phish.local.phishkit".into()),
                req.target_domain.unwrap_or_default(),
                req.lure_url.unwrap_or_default(),
                auth_meta,
                stack,
                req.notes.unwrap_or_default(),
                assessment_id,
                created,
                now,
            ],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO app_meta(key, value) VALUES('active_profile', ?1)",
            params![id],
        )?;
        Ok(())
    })?;
    get_profile(&id)?.ok_or_else(|| AppError::msg("profile missing after upsert"))
}

pub fn delete_profile(id: &str) -> AppResult<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM profiles WHERE id = ?1", params![id])?;
        let active: Option<String> = conn
            .query_row(
                "SELECT value FROM app_meta WHERE key = 'active_profile'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if active.as_deref() == Some(id) {
            conn.execute("DELETE FROM app_meta WHERE key = 'active_profile'", [])?;
        }
        Ok(())
    })
}

pub fn update_profile_fields(
    id: &str,
    phishlet: Option<&str>,
    dryrun: Option<&str>,
    target: Option<&str>,
    lure: Option<&str>,
    stack: Option<&Value>,
    auth_meta: Option<&Value>,
) -> AppResult<()> {
    with_db(|conn| {
        let now = now_iso();
        if let Some(p) = phishlet {
            conn.execute(
                "UPDATE profiles SET phishlet = ?1, updated_at = ?2 WHERE id = ?3",
                params![p, now, id],
            )?;
        }
        if let Some(d) = dryrun {
            conn.execute(
                "UPDATE profiles SET dryrun_domain = ?1, updated_at = ?2 WHERE id = ?3",
                params![d, now, id],
            )?;
        }
        if let Some(t) = target {
            conn.execute(
                "UPDATE profiles SET target_domain = ?1, updated_at = ?2 WHERE id = ?3",
                params![t, now, id],
            )?;
        }
        if let Some(l) = lure {
            conn.execute(
                "UPDATE profiles SET lure_url = ?1, updated_at = ?2 WHERE id = ?3",
                params![l, now, id],
            )?;
        }
        if let Some(s) = stack {
            conn.execute(
                "UPDATE profiles SET stack_info = ?1, updated_at = ?2 WHERE id = ?3",
                params![s.to_string(), now, id],
            )?;
        }
        if let Some(a) = auth_meta {
            conn.execute(
                "UPDATE profiles SET auth_meta = ?1, updated_at = ?2 WHERE id = ?3",
                params![a.to_string(), now, id],
            )?;
        }
        Ok(())
    })
}

pub fn list_captures(profile_id: &str) -> AppResult<Vec<CaptureRow>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, evilginx_session_id, data, evilginx_create_time,
                    evilginx_update_time, synced_at
             FROM captured_sessions WHERE profile_id = ?1
             ORDER BY evilginx_update_time DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![profile_id], |r| {
            let data_s: String = r.get("data")?;
            Ok(CaptureRow {
                id: r.get("id")?,
                profile_id: r.get("profile_id")?,
                evilginx_session_id: r.get("evilginx_session_id")?,
                data: serde_json::from_str(&data_s).unwrap_or(Value::Null),
                evilginx_create_time: r.get("evilginx_create_time")?,
                evilginx_update_time: r.get("evilginx_update_time")?,
                synced_at: r.get("synced_at")?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    })
}

pub fn upsert_capture(
    profile_id: &str,
    evilginx_session_id: i64,
    data: &Value,
    create_time: Option<i64>,
    update_time: Option<i64>,
) -> AppResult<()> {
    with_db(|conn| {
        let ignored: bool = conn
            .query_row(
                "SELECT 1 FROM ignored_sessions WHERE profile_id = ?1 AND evilginx_session_id = ?2",
                params![profile_id, evilginx_session_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if ignored {
            return Ok(());
        }
        let now = now_iso();
        conn.execute(
            "INSERT INTO captured_sessions(profile_id, evilginx_session_id, data,
                 evilginx_create_time, evilginx_update_time, synced_at)
             VALUES(?1,?2,?3,?4,?5,?6)
             ON CONFLICT(evilginx_session_id) DO UPDATE SET
               data=excluded.data,
               evilginx_create_time=excluded.evilginx_create_time,
               evilginx_update_time=excluded.evilginx_update_time,
               synced_at=excluded.synced_at",
            params![
                profile_id,
                evilginx_session_id,
                data.to_string(),
                create_time,
                update_time,
                now
            ],
        )?;
        Ok(())
    })
}

pub fn ignore_and_delete_capture(profile_id: &str, evilginx_session_id: i64) -> AppResult<()> {
    with_db(|conn| {
        let now = now_iso();
        conn.execute(
            "INSERT OR REPLACE INTO ignored_sessions(profile_id, evilginx_session_id, ignored_at)
             VALUES(?1,?2,?3)",
            params![profile_id, evilginx_session_id, now],
        )?;
        conn.execute(
            "DELETE FROM captured_sessions WHERE evilginx_session_id = ?1",
            params![evilginx_session_id],
        )?;
        Ok(())
    })
}

pub fn prune_empty_captures(profile_id: &str) -> AppResult<usize> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, evilginx_session_id, data FROM captured_sessions WHERE profile_id = ?1",
        )?;
        let rows: Vec<(i64, i64, String)> = stmt
            .query_map(params![profile_id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        let mut n = 0usize;
        let now = now_iso();
        for (_id, sid, data_s) in rows {
            let data: Value = serde_json::from_str(&data_s).unwrap_or(Value::Null);
            let user = data
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let pass = data
                .get("password")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let custom = data.get("custom").and_then(|v| v.as_object());
            let tokens = data.get("body_tokens").and_then(|v| v.as_object());
            let empty = user.is_empty()
                && pass.is_empty()
                && custom.map(|c| c.is_empty()).unwrap_or(true)
                && tokens.map(|t| t.is_empty()).unwrap_or(true);
            if empty {
                conn.execute(
                    "INSERT OR REPLACE INTO ignored_sessions(profile_id, evilginx_session_id, ignored_at)
                     VALUES(?1,?2,?3)",
                    params![profile_id, sid, now],
                )?;
                conn.execute(
                    "DELETE FROM captured_sessions WHERE evilginx_session_id = ?1",
                    params![sid],
                )?;
                n += 1;
            }
        }
        Ok(n)
    })
}
