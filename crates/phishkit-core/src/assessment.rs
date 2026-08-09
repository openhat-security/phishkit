use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::{self, now_iso, Profile};
use crate::engagement::upstream_domain;
use crate::error::{AppError, AppResult};

pub const LEGACY_UNASSIGNED_ID: &str = "legacy-unassigned";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentScope {
    pub id: String,
    pub assessment_id: String,
    pub scope_type: String,
    pub value: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    pub id: String,
    pub name: String,
    pub primary_domain: String,
    pub authorization_ref: String,
    pub authorized_by: String,
    pub authorized_at: Option<String>,
    pub notes: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub scopes: Vec<AssessmentScope>,
    pub target_count: i64,
    pub campaign_count: i64,
    pub session_count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssessment {
    pub name: String,
    pub primary_domain: String,
    pub authorization_ref: Option<String>,
    pub authorized_by: Option<String>,
    pub authorized_at: Option<String>,
    pub notes: Option<String>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssessment {
    pub id: String,
    pub name: Option<String>,
    pub primary_domain: Option<String>,
    pub authorization_ref: Option<String>,
    pub authorized_by: Option<String>,
    pub authorized_at: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub scopes: Option<Vec<String>>,
}

fn row_to_scope(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssessmentScope> {
    Ok(AssessmentScope {
        id: row.get("id")?,
        assessment_id: row.get("assessment_id")?,
        scope_type: row.get("scope_type")?,
        value: row.get("value")?,
        created_at: row.get("created_at")?,
    })
}

fn load_scopes(conn: &Connection, assessment_id: &str) -> AppResult<Vec<AssessmentScope>> {
    let mut stmt = conn.prepare(
        "SELECT id, assessment_id, scope_type, value, created_at
         FROM assessment_scopes WHERE assessment_id = ?1 ORDER BY created_at",
    )?;
    let rows = stmt.query_map(params![assessment_id], row_to_scope)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn target_count(conn: &Connection, assessment_id: &str) -> AppResult<i64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM profiles WHERE assessment_id = ?1",
        params![assessment_id],
        |r| r.get(0),
    )?;
    Ok(n)
}

fn campaign_count(conn: &Connection, assessment_id: &str) -> AppResult<i64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM campaigns WHERE assessment_id = ?1",
        params![assessment_id],
        |r| r.get(0),
    )?;
    Ok(n)
}

fn session_count(conn: &Connection, assessment_id: &str) -> AppResult<i64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM captured_sessions cs
         INNER JOIN profiles p ON p.id = cs.profile_id
         WHERE p.assessment_id = ?1",
        params![assessment_id],
        |r| r.get(0),
    )?;
    Ok(n)
}

fn assessment_from_row(conn: &Connection, row: &rusqlite::Row<'_>) -> rusqlite::Result<Assessment> {
    let id: String = row.get("id")?;
    let scopes = load_scopes(conn, &id).unwrap_or_default();
    let targets = target_count(conn, &id).unwrap_or(0);
    let campaigns = campaign_count(conn, &id).unwrap_or(0);
    let sessions = session_count(conn, &id).unwrap_or(0);
    Ok(Assessment {
        id,
        name: row.get("name")?,
        primary_domain: row.get("primary_domain")?,
        authorization_ref: row.get("authorization_ref")?,
        authorized_by: row.get("authorized_by")?,
        authorized_at: row.get("authorized_at")?,
        notes: row.get("notes")?,
        status: row.get("status")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        scopes,
        target_count: targets,
        campaign_count: campaigns,
        session_count: sessions,
    })
}

pub fn list_assessments(include_archived: bool) -> AppResult<Vec<Assessment>> {
    db::with_db(|conn| {
        let sql = if include_archived {
            "SELECT id, name, primary_domain, authorization_ref, authorized_by, authorized_at,
                    notes, status, created_at, updated_at
             FROM assessments ORDER BY updated_at DESC"
        } else {
            "SELECT id, name, primary_domain, authorization_ref, authorized_by, authorized_at,
                    notes, status, created_at, updated_at
             FROM assessments WHERE status = 'active' ORDER BY updated_at DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| assessment_from_row(conn, row))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    })
}

pub fn get_assessment(id: &str) -> AppResult<Option<Assessment>> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, primary_domain, authorization_ref, authorized_by, authorized_at,
                    notes, status, created_at, updated_at
             FROM assessments WHERE id = ?1",
        )?;
        let a = stmt
            .query_row(params![id], |row| assessment_from_row(conn, row))
            .optional()?;
        Ok(a)
    })
}

fn insert_scope(conn: &Connection, assessment_id: &str, value: &str) -> AppResult<()> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(());
    }
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM assessment_scopes
             WHERE assessment_id = ?1 AND scope_type = 'domain' AND value = ?2",
            params![assessment_id, value],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if exists {
        return Ok(());
    }
    let now = now_iso();
    conn.execute(
        "INSERT INTO assessment_scopes(id, assessment_id, scope_type, value, created_at)
         VALUES(?1, ?2, 'domain', ?3, ?4)",
        params![Uuid::new_v4().to_string(), assessment_id, value, now],
    )?;
    Ok(())
}

pub fn create_assessment(req: CreateAssessment) -> AppResult<Assessment> {
    let id = Uuid::new_v4().to_string();
    let now = now_iso();
    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO assessments(id, name, primary_domain, authorization_ref, authorized_by,
                                     authorized_at, notes, status, created_at, updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,'active',?8,?9)",
            params![
                id,
                req.name.trim(),
                req.primary_domain.trim(),
                req.authorization_ref.unwrap_or_default(),
                req.authorized_by.unwrap_or_default(),
                req.authorized_at,
                req.notes.unwrap_or_default(),
                now,
                now,
            ],
        )?;
        insert_scope(conn, &id, &req.primary_domain)?;
        if let Some(scopes) = req.scopes {
            for s in scopes {
                insert_scope(conn, &id, &s)?;
            }
        }
        Ok(())
    })?;
    get_assessment(&id)?.ok_or_else(|| AppError::msg("assessment missing after create"))
}

pub fn update_assessment(req: UpdateAssessment) -> AppResult<Assessment> {
    let now = now_iso();
    db::with_db(|conn| {
        if let Some(name) = &req.name {
            conn.execute(
                "UPDATE assessments SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name.trim(), now, req.id],
            )?;
        }
        if let Some(domain) = &req.primary_domain {
            conn.execute(
                "UPDATE assessments SET primary_domain = ?1, updated_at = ?2 WHERE id = ?3",
                params![domain.trim(), now, req.id],
            )?;
            insert_scope(conn, &req.id, domain)?;
        }
        if let Some(v) = &req.authorization_ref {
            conn.execute(
                "UPDATE assessments SET authorization_ref = ?1, updated_at = ?2 WHERE id = ?3",
                params![v, now, req.id],
            )?;
        }
        if let Some(v) = &req.authorized_by {
            conn.execute(
                "UPDATE assessments SET authorized_by = ?1, updated_at = ?2 WHERE id = ?3",
                params![v, now, req.id],
            )?;
        }
        if let Some(v) = &req.authorized_at {
            conn.execute(
                "UPDATE assessments SET authorized_at = ?1, updated_at = ?2 WHERE id = ?3",
                params![v, now, req.id],
            )?;
        }
        if let Some(v) = &req.notes {
            conn.execute(
                "UPDATE assessments SET notes = ?1, updated_at = ?2 WHERE id = ?3",
                params![v, now, req.id],
            )?;
        }
        if let Some(v) = &req.status {
            conn.execute(
                "UPDATE assessments SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![v, now, req.id],
            )?;
        }
        if let Some(scopes) = &req.scopes {
            for s in scopes {
                insert_scope(conn, &req.id, s)?;
            }
        }
        Ok(())
    })?;
    get_assessment(&req.id)?.ok_or_else(|| AppError::msg("assessment not found"))
}

pub fn archive_assessment(id: &str) -> AppResult<Assessment> {
    let now = now_iso();
    db::with_db(|conn| {
        let n = conn.execute(
            "UPDATE assessments SET status = 'archived', updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        if n == 0 {
            return Err(AppError::msg("assessment not found"));
        }
        if let Some(active) = db::get_meta(conn, "active_assessment")? {
            if active == id {
                db::clear_active_assessment(conn)?;
            }
        }
        Ok(())
    })?;
    get_assessment(id)?.ok_or_else(|| AppError::msg("assessment not found"))
}

pub fn unarchive_assessment(id: &str) -> AppResult<Assessment> {
    let now = now_iso();
    db::with_db(|conn| {
        let n = conn.execute(
            "UPDATE assessments SET status = 'active', updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        if n == 0 {
            return Err(AppError::msg("assessment not found"));
        }
        Ok(())
    })?;
    get_assessment(id)?.ok_or_else(|| AppError::msg("assessment not found"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAssessmentResult {
    pub id: String,
    pub name: String,
    pub profiles_deleted: usize,
    pub campaigns_deleted: usize,
    pub templates_deleted: usize,
    pub lists_deleted: usize,
}

/// Permanently erase an assessment and all engagement-owned rows from the app
/// database (targets/profiles, lures, captures, campaigns, attempts, recipient
/// lists, scoped templates, scopes). Shared kit files under
/// `kit/evilginx/phishlets/` are left alone — they are not assessment-scoped.
/// Callers should run hosts cleanup separately when desired.
pub fn delete_assessment(id: &str) -> AppResult<DeleteAssessmentResult> {
    let existing = get_assessment(id)?.ok_or_else(|| AppError::msg("assessment not found"))?;
    let name = existing.name.clone();

    let result = db::with_db(|conn| {
        let pids: Vec<String> = {
            let mut stmt = conn.prepare("SELECT id FROM profiles WHERE assessment_id = ?1")?;
            let rows = stmt.query_map(params![id], |r| r.get::<_, String>(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let cids: Vec<String> = {
            let mut stmt = conn.prepare("SELECT id FROM campaigns WHERE assessment_id = ?1")?;
            let rows = stmt.query_map(params![id], |r| r.get::<_, String>(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let lids: Vec<String> = {
            let mut stmt =
                conn.prepare("SELECT id FROM recipient_lists WHERE assessment_id = ?1")?;
            let rows = stmt.query_map(params![id], |r| r.get::<_, String>(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let mut campaigns_deleted = 0usize;
        for cid in &cids {
            conn.execute(
                "DELETE FROM campaign_attempts WHERE campaign_id = ?1",
                params![cid],
            )?;
            campaigns_deleted +=
                conn.execute("DELETE FROM campaigns WHERE id = ?1", params![cid])?;
        }

        let mut lists_deleted = 0usize;
        for lid in &lids {
            conn.execute("DELETE FROM recipients WHERE list_id = ?1", params![lid])?;
            lists_deleted +=
                conn.execute("DELETE FROM recipient_lists WHERE id = ?1", params![lid])?;
        }

        let templates_deleted = conn.execute(
            "DELETE FROM email_templates WHERE assessment_id = ?1",
            params![id],
        )?;

        let mut profiles_deleted = 0usize;
        for pid in &pids {
            // Sessions / ignored / lures / proxy_runs cascade from profiles when FKs apply;
            // delete children explicitly for older DBs without FK enforcement.
            conn.execute(
                "DELETE FROM captured_sessions WHERE profile_id = ?1",
                params![pid],
            )?;
            conn.execute(
                "DELETE FROM ignored_sessions WHERE profile_id = ?1",
                params![pid],
            )?;
            conn.execute("DELETE FROM lures WHERE profile_id = ?1", params![pid])?;
            conn.execute("DELETE FROM proxy_runs WHERE profile_id = ?1", params![pid])?;
            profiles_deleted += conn.execute("DELETE FROM profiles WHERE id = ?1", params![pid])?;
        }

        conn.execute(
            "DELETE FROM assessment_scopes WHERE assessment_id = ?1",
            params![id],
        )?;
        let n = conn.execute("DELETE FROM assessments WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(AppError::msg("assessment not found"));
        }

        if let Some(active) = db::get_meta(conn, "active_assessment")? {
            if active == id {
                db::clear_active_assessment(conn)?;
            }
        }

        Ok(DeleteAssessmentResult {
            id: id.to_string(),
            name,
            profiles_deleted,
            campaigns_deleted,
            templates_deleted,
            lists_deleted,
        })
    })?;
    Ok(result)
}

/// Create a new active assessment from an archived (or active) one: copies
/// metadata and Targets/Lures, not campaigns, sessions, or recipient PII.
pub fn clone_assessment(id: &str) -> AppResult<Assessment> {
    let src = get_assessment(id)?.ok_or_else(|| AppError::msg("assessment not found"))?;
    let now = now_iso();
    let new_id = Uuid::new_v4().to_string();
    let new_name = {
        let base = src.name.trim();
        if base.is_empty() {
            "Assessment copy".into()
        } else if base.contains("(copy)") {
            format!("{base}")
        } else {
            format!("{base} (copy)")
        }
    };

    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO assessments(id, name, primary_domain, authorization_ref, authorized_by,
                                     authorized_at, notes, status, created_at, updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,'active',?8,?9)",
            params![
                new_id,
                new_name,
                src.primary_domain,
                src.authorization_ref,
                src.authorized_by,
                src.authorized_at,
                src.notes,
                now,
                now,
            ],
        )?;

        let mut scope_stmt = conn
            .prepare("SELECT scope_type, value FROM assessment_scopes WHERE assessment_id = ?1")?;
        let scopes: Vec<(String, String)> = scope_stmt
            .query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        drop(scope_stmt);
        for (stype, value) in scopes {
            conn.execute(
                "INSERT INTO assessment_scopes(id, assessment_id, scope_type, value, created_at)
                 VALUES(?1, ?2, ?3, ?4, ?5)",
                params![Uuid::new_v4().to_string(), new_id, stype, value, now],
            )?;
        }

        let mut pstmt = conn.prepare(
            "SELECT id, name, phishlet, dryrun_domain, target_domain, lure_url,
                    auth_meta, stack_info, notes
             FROM profiles WHERE assessment_id = ?1",
        )?;
        let profiles: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            String,
        )> = pstmt
            .query_map(params![id], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();
        drop(pstmt);

        for (old_pid, name, phishlet, dryrun, target, lure_url, auth_meta, stack, notes) in profiles
        {
            let new_pid = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO profiles(id, name, phishlet, dryrun_domain, target_domain, lure_url,
                                      auth_meta, stack_info, notes, assessment_id, created_at, updated_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![
                    new_pid,
                    name,
                    phishlet,
                    dryrun,
                    target,
                    lure_url,
                    auth_meta,
                    stack,
                    notes,
                    new_id,
                    now,
                    now,
                ],
            )?;

            let mut lstmt = conn.prepare(
                "SELECT name, path, lure_url, redirect_url, redirector, ua_filter,
                        og_title, og_desc, og_image, og_url, paused, is_default
                 FROM lures WHERE profile_id = ?1",
            )?;
            let lures: Vec<(
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                i64,
                i64,
            )> = lstmt
                .query_map(params![old_pid], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                        r.get(9)?,
                        r.get(10)?,
                        r.get(11)?,
                    ))
                })?
                .filter_map(|r| r.ok())
                .collect();
            drop(lstmt);
            for (
                lname,
                path,
                lure_url,
                redirect_url,
                redirector,
                ua_filter,
                og_title,
                og_desc,
                og_image,
                og_url,
                paused,
                is_default,
            ) in lures
            {
                conn.execute(
                    "INSERT INTO lures(id, profile_id, name, path, lure_url, redirect_url, redirector,
                                       ua_filter, og_title, og_desc, og_image, og_url, paused,
                                       is_default, created_at, updated_at)
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
                    params![
                        Uuid::new_v4().to_string(),
                        new_pid,
                        lname,
                        path,
                        lure_url,
                        redirect_url,
                        redirector,
                        ua_filter,
                        og_title,
                        og_desc,
                        og_image,
                        og_url,
                        paused,
                        is_default,
                        now,
                        now,
                    ],
                )?;
            }
        }
        Ok(())
    })?;

    get_assessment(&new_id)?.ok_or_else(|| AppError::msg("assessment missing after clone"))
}

pub fn set_active_assessment(id: &str) -> AppResult<()> {
    db::with_db(|conn| db::set_active_assessment(conn, id))
}

pub fn get_active_assessment() -> AppResult<Option<Assessment>> {
    let id = db::get_active_assessment_id()?;
    match id {
        Some(id) => get_assessment(&id),
        None => Ok(None),
    }
}

pub fn list_targets(assessment_id: &str) -> AppResult<Vec<Profile>> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, phishlet, dryrun_domain, target_domain, lure_url,
                    auth_meta, stack_info, notes, assessment_id, created_at, updated_at
             FROM profiles WHERE assessment_id = ?1 ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![assessment_id], db::row_to_profile)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    })
}

fn ensure_legacy_unassigned_conn(conn: &Connection) -> AppResult<String> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM assessments WHERE id = ?1",
            params![LEGACY_UNASSIGNED_ID],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists {
        let now = now_iso();
        conn.execute(
            "INSERT INTO assessments(id, name, primary_domain, authorization_ref, authorized_by,
                                     notes, status, created_at, updated_at)
             VALUES(?1, 'Legacy / Unassigned', '', '', '', '', 'active', ?2, ?3)",
            params![LEGACY_UNASSIGNED_ID, now, now],
        )?;
    }
    Ok(LEGACY_UNASSIGNED_ID.to_string())
}

/// Find or create an assessment for a primary domain (migration / ensure_destination).
pub fn find_or_create_for_domain(conn: &Connection, domain: &str) -> AppResult<String> {
    let domain = domain.trim();
    let (name, primary) = if domain.is_empty() {
        ("Legacy".to_string(), String::new())
    } else {
        (domain.to_string(), domain.to_string())
    };

    if !primary.is_empty() {
        if let Some(id) = conn
            .query_row(
                "SELECT id FROM assessments WHERE primary_domain = ?1 AND status = 'active' LIMIT 1",
                params![primary],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            insert_scope(conn, &id, &primary)?;
            return Ok(id);
        }
    }

    let id = Uuid::new_v4().to_string();
    let now = now_iso();
    conn.execute(
        "INSERT INTO assessments(id, name, primary_domain, authorization_ref, authorized_by,
                                 notes, status, created_at, updated_at)
         VALUES(?1,?2,?3,'','','','active',?4,?5)",
        params![id, name, primary, now, now],
    )?;
    if !primary.is_empty() {
        insert_scope(conn, &id, &primary)?;
    }
    Ok(id)
}

pub(crate) fn migrate_v8_assessments(conn: &Connection) -> AppResult<()> {
    let mut stmt = conn.prepare("SELECT id, target_domain, assessment_id FROM profiles")?;
    let profiles: Vec<(String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (pid, target_domain, assessment_id) in profiles {
        if !assessment_id.is_empty() {
            continue;
        }
        let domain = upstream_domain(&target_domain);
        let aid = find_or_create_for_domain(conn, &domain)?;
        conn.execute(
            "UPDATE profiles SET assessment_id = ?1 WHERE id = ?2",
            params![aid, pid],
        )?;
    }

    conn.execute(
        "UPDATE campaigns SET assessment_id = (
            SELECT assessment_id FROM profiles WHERE profiles.id = campaigns.profile_id
         )
         WHERE profile_id != '' AND (assessment_id IS NULL OR assessment_id = '')",
        [],
    )?;

    let legacy_id = ensure_legacy_unassigned_conn(conn)?;

    let mut stmt = conn.prepare("SELECT id FROM recipient_lists")?;
    let list_ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    for list_id in list_ids {
        let linked: Option<String> = conn
            .query_row(
                "SELECT assessment_id FROM campaigns WHERE list_id = ?1 AND assessment_id != '' LIMIT 1",
                params![list_id],
                |r| r.get(0),
            )
            .optional()?;
        let aid = linked.unwrap_or_else(|| legacy_id.clone());
        conn.execute(
            "UPDATE recipient_lists SET assessment_id = ?1 WHERE id = ?2 AND assessment_id = ''",
            params![aid, list_id],
        )?;
    }

    Ok(())
}

/// Partially mask an email for redacted exports: keep first char + domain.
fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((user, dom)) => {
            let first = user
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default();
            format!("{first}***@{dom}")
        }
        None => "***".to_string(),
    }
}

/// Redact secret values in a capture `data` object while preserving structure
/// (token/cookie *names* survive; values become "REDACTED").
fn redact_capture_data(data: &Value) -> Value {
    let mut out = data.clone();
    if let Some(obj) = out.as_object_mut() {
        if obj
            .get("password")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false)
        {
            obj.insert("password".into(), json!("REDACTED"));
        }
        for key in ["custom", "body_tokens"] {
            if let Some(Value::Object(m)) = obj.get_mut(key) {
                for (_, v) in m.iter_mut() {
                    *v = json!("REDACTED");
                }
            }
        }
        if let Some(Value::Object(domains)) = obj.get_mut("tokens") {
            for (_, v) in domains.iter_mut() {
                if let Value::Object(names) = v {
                    let keys: Vec<Value> = names.keys().map(|k| json!(k)).collect();
                    *v = Value::Array(keys);
                }
            }
        }
    }
    out
}

/// Build a portable JSON evidence bundle for an assessment. With `redact`,
/// recipient emails are masked and captured secrets are stripped, so the bundle
/// is safe to attach to a report. Reusable assets (phishlets/templates/lures)
/// are included by reference, never mutated.
pub fn export_bundle(id: &str, redact: bool) -> AppResult<Value> {
    let assessment = get_assessment(id)?.ok_or_else(|| AppError::msg("assessment not found"))?;
    let targets = list_targets(id)?;

    let mut targets_json = Vec::new();
    let mut sessions_json = Vec::new();
    for t in &targets {
        let lures = crate::lure_ops::list_lures(&t.id).unwrap_or_default();
        targets_json.push(json!({
            "id": t.id,
            "name": t.name,
            "targetDomain": t.target_domain,
            "phishlet": t.phishlet,
            "dryrunDomain": t.dryrun_domain,
            "lureUrl": t.lure_url,
            "lures": lures,
        }));
        for cap in db::list_captures(&t.id).unwrap_or_default() {
            let data = if redact {
                redact_capture_data(&cap.data)
            } else {
                cap.data.clone()
            };
            sessions_json.push(json!({
                "profileId": cap.profile_id,
                "evilginxSessionId": cap.evilginx_session_id,
                "createTime": cap.evilginx_create_time,
                "updateTime": cap.evilginx_update_time,
                "data": data,
            }));
        }
    }

    let templates = crate::mail::list_templates(Some(id.to_string())).unwrap_or_default();

    let lists = crate::mail::list_recipient_lists(Some(id.to_string())).unwrap_or_default();
    let mut lists_json = Vec::new();
    for l in &lists {
        let recips = crate::mail::list_recipients(l.id.clone()).unwrap_or_default();
        let recips_json: Vec<Value> = recips
            .iter()
            .map(|r| {
                json!({
                    "email": if redact { mask_email(&r.email) } else { r.email.clone() },
                    "firstName": r.first_name,
                    "lastName": r.last_name,
                })
            })
            .collect();
        lists_json.push(json!({
            "id": l.id,
            "name": l.name,
            "recipients": recips_json,
        }));
    }

    let campaigns = crate::campaign::list_campaigns(Some(id.to_string())).unwrap_or_default();
    let mut campaigns_json = Vec::new();
    for c in &campaigns {
        let attempts = crate::campaign::list_attempts(c.id.clone()).unwrap_or_default();
        let attempts_json: Vec<Value> = attempts
            .iter()
            .map(|a| {
                json!({
                    "email": if redact { mask_email(&a.email) } else { a.email.clone() },
                    "status": a.status,
                    "sentAt": a.sent_at,
                    "trackingToken": a.tracking_token,
                    "delivered": a.delivered_at.is_some(),
                    "opened": a.opened_at.is_some(),
                    "clicked": a.clicked_at.is_some(),
                    "bounced": a.bounced_at.is_some(),
                    "complained": a.complained_at.is_some(),
                })
            })
            .collect();
        campaigns_json.push(json!({
            "id": c.id,
            "name": c.name,
            "status": c.status,
            "mode": c.mode,
            "sent": c.sent,
            "failed": c.failed,
            "pending": c.pending,
            "total": c.total,
            "attempts": attempts_json,
        }));
    }

    Ok(json!({
        "kind": "phishkit.assessment-export",
        "version": 1,
        "generatedAt": now_iso(),
        "redacted": redact,
        "assessment": assessment,
        "targets": targets_json,
        "templates": templates,
        "recipientLists": lists_json,
        "campaigns": campaigns_json,
        "sessions": sessions_json,
        "counts": {
            "targets": targets.len(),
            "campaigns": campaigns.len(),
            "sessions": sessions_json.len(),
        },
    }))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurgeResult {
    pub sessions_deleted: usize,
    pub attempts_deleted: usize,
    pub recipients_deleted: usize,
    pub lists_deleted: usize,
    pub kept: Vec<String>,
}

/// Selectively purge sensitive assessment data (captured sessions, send
/// attempts, recipient PII) while preserving reusable assets: profiles,
/// phishlets, named lures, and email templates. Intended for post-engagement
/// cleanup after an export bundle has been saved.
pub fn purge_assessment_data(
    id: &str,
    sessions: bool,
    attempts: bool,
    pii: bool,
) -> AppResult<PurgeResult> {
    db::with_db(|conn| {
        let pids: Vec<String> = {
            let mut stmt = conn.prepare("SELECT id FROM profiles WHERE assessment_id = ?1")?;
            let rows = stmt.query_map(params![id], |r| r.get::<_, String>(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let cids: Vec<String> = {
            let mut stmt = conn.prepare("SELECT id FROM campaigns WHERE assessment_id = ?1")?;
            let rows = stmt.query_map(params![id], |r| r.get::<_, String>(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let mut sessions_deleted = 0;
        let mut attempts_deleted = 0;
        let mut recipients_deleted = 0;
        let mut lists_deleted = 0;

        if sessions {
            for pid in &pids {
                sessions_deleted += conn.execute(
                    "DELETE FROM captured_sessions WHERE profile_id = ?1",
                    params![pid],
                )?;
                conn.execute(
                    "DELETE FROM ignored_sessions WHERE profile_id = ?1",
                    params![pid],
                )?;
            }
        }
        if attempts {
            for cid in &cids {
                attempts_deleted += conn.execute(
                    "DELETE FROM campaign_attempts WHERE campaign_id = ?1",
                    params![cid],
                )?;
            }
        }
        if pii {
            let lids: Vec<String> = {
                let mut stmt =
                    conn.prepare("SELECT id FROM recipient_lists WHERE assessment_id = ?1")?;
                let rows = stmt.query_map(params![id], |r| r.get::<_, String>(0))?;
                rows.filter_map(|r| r.ok()).collect()
            };
            for lid in &lids {
                recipients_deleted +=
                    conn.execute("DELETE FROM recipients WHERE list_id = ?1", params![lid])?;
                lists_deleted +=
                    conn.execute("DELETE FROM recipient_lists WHERE id = ?1", params![lid])?;
            }
        }

        Ok(PurgeResult {
            sessions_deleted,
            attempts_deleted,
            recipients_deleted,
            lists_deleted,
            kept: vec![
                "profiles".into(),
                "phishlets".into(),
                "lures".into(),
                "email_templates".into(),
            ],
        })
    })
}

/// Remove the /etc/hosts entries phishkit added for every target in an
/// assessment, in a single privileged operation (one prompt on macOS).
pub fn hosts_cleanup(id: &str) -> AppResult<Value> {
    let targets = list_targets(id)?;
    let mut fqdns: Vec<String> = Vec::new();
    for t in &targets {
        if t.dryrun_domain.trim().is_empty() {
            continue;
        }
        for f in crate::hosts::required_fqdns(&t.dryrun_domain, &t.phishlet) {
            fqdns.push(f);
        }
    }
    fqdns.sort();
    fqdns.dedup();
    crate::hosts::remove_fqdns(fqdns)
}

/// Resolve assessment for ensure_destination when none specified.
pub fn resolve_assessment_for_target(
    target_domain: &str,
    assessment_id: Option<String>,
) -> AppResult<String> {
    if let Some(id) = assessment_id.filter(|s| !s.is_empty()) {
        return Ok(id);
    }
    if let Some(active) = db::get_active_assessment_id()? {
        return Ok(active);
    }
    db::with_db(|conn| {
        let domain = upstream_domain(target_domain);
        find_or_create_for_domain(conn, &domain)
    })
}
