use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::{Timelike, Utc};
use once_cell::sync::Lazy;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::{self, now_iso, with_db};
use crate::error::{AppError, AppResult};
use crate::lure_ops;
use crate::mail::{self, EmailTemplate, Recipient, SendReceipt};

static RUNNER: Lazy<Mutex<Option<RunningCampaign>>> = Lazy::new(|| Mutex::new(None));

struct RunningCampaign {
    id: String,
    cancel: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub template_id: String,
    pub list_id: String,
    pub link_url: String,
    pub profile_id: String,
    pub assessment_id: String,
    pub lure_id: String,
    pub sender_account_id: String,
    pub rate_per_minute: i64,
    pub status: String,
    /// aitm (credential/token capture via evilginx) | awareness (click-only training)
    pub mode: String,
    pub scheduled_at: Option<String>,
    pub send_window_start: String,
    pub send_window_end: String,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub pending: i64,
    pub sent: i64,
    pub failed: i64,
    pub total: i64,
    /// 0–100
    pub progress_pct: f64,
    /// Rough ETA for remaining pending at current rate (seconds)
    pub eta_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignAttempt {
    pub id: i64,
    pub campaign_id: String,
    pub recipient_id: i64,
    pub email: String,
    pub status: String,
    pub error: String,
    pub sent_at: Option<String>,
    pub tracking_token: String,
    pub tracked_url: String,
    pub provider_message_id: String,
    pub delivered_at: Option<String>,
    pub opened_at: Option<String>,
    pub clicked_at: Option<String>,
    pub bounced_at: Option<String>,
    pub complained_at: Option<String>,
    pub bounce_reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCampaign {
    pub name: String,
    pub template_id: String,
    pub list_id: String,
    pub link_url: String,
    pub profile_id: Option<String>,
    pub assessment_id: Option<String>,
    pub lure_id: Option<String>,
    pub sender_account_id: Option<String>,
    pub rate_per_minute: Option<i64>,
    pub mode: Option<String>,
    pub scheduled_at: Option<String>,
    pub send_window_start: Option<String>,
    pub send_window_end: Option<String>,
}

/// Immutable point-in-time copy of sender identity + rendered content, kept so
/// later template/sender edits never rewrite a launched campaign's audit trail.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignSnapshot {
    pub sender_account_id: String,
    pub sender_label: String,
    pub from_email: String,
    pub provider: String,
    pub subject: String,
    pub html_body: String,
    pub link_url: String,
    pub lure_id: String,
    pub lure_url: String,
    pub captured_at: String,
}

const CAMPAIGN_SELECT: &str = "SELECT id, name, template_id, list_id, link_url, profile_id,
    assessment_id, lure_id, sender_account_id, rate_per_minute, status, mode,
    scheduled_at, send_window_start, send_window_end, created_at, updated_at,
    started_at, finished_at";

fn new_tracking_token() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}

fn append_tracking_param(base_url: &str, token: &str) -> AppResult<String> {
    let mut url = url::Url::parse(base_url)
        .map_err(|e| AppError::msg(format!("invalid campaign link URL: {e}")))?;
    url.query_pairs_mut().append_pair("pk", token);
    Ok(url.to_string())
}

fn pk_from_landing_url(landing: &str) -> Option<String> {
    url::Url::parse(landing)
        .ok()
        .and_then(|u| {
            u.query_pairs()
                .find(|(k, _)| k == "pk")
                .map(|(_, v)| v.into_owned())
        })
        .filter(|s| !s.is_empty())
}

fn parse_iso(s: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ").ok()
}

fn parse_hhmm(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut parts = s.split(':');
    let h: i64 = parts.next()?.trim().parse().ok()?;
    let m: i64 = parts.next().unwrap_or("0").trim().parse().ok()?;
    if !(0..=23).contains(&h) || !(0..=59).contains(&m) {
        return None;
    }
    Some(h * 60 + m)
}

/// Whether the current UTC time-of-day falls inside the (optional) send window.
/// Empty/invalid window = always allowed.
fn in_send_window(start: &str, end: &str) -> bool {
    match (parse_hhmm(start), parse_hhmm(end)) {
        (Some(a), Some(b)) => {
            let now = Utc::now();
            let n = now.hour() as i64 * 60 + now.minute() as i64;
            if a <= b {
                n >= a && n <= b
            } else {
                n >= a || n <= b
            }
        }
        _ => true,
    }
}

fn attempt_counts(conn: &rusqlite::Connection, campaign_id: &str) -> AppResult<(i64, i64, i64)> {
    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM campaign_attempts WHERE campaign_id = ?1 AND status = 'pending'",
        params![campaign_id],
        |r| r.get(0),
    )?;
    let sent: i64 = conn.query_row(
        "SELECT COUNT(*) FROM campaign_attempts WHERE campaign_id = ?1 AND status = 'sent'",
        params![campaign_id],
        |r| r.get(0),
    )?;
    let failed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM campaign_attempts WHERE campaign_id = ?1 AND status = 'failed'",
        params![campaign_id],
        |r| r.get(0),
    )?;
    Ok((pending, sent, failed))
}

fn with_progress(mut c: Campaign, pending: i64, sent: i64, failed: i64) -> Campaign {
    c.pending = pending;
    c.sent = sent;
    c.failed = failed;
    c.total = pending + sent + failed;
    c.progress_pct = if c.total > 0 {
        ((sent + failed) as f64 / c.total as f64) * 100.0
    } else {
        0.0
    };
    let rate = c.rate_per_minute.max(1) as f64;
    c.eta_seconds = if pending > 0 {
        ((pending as f64 / rate) * 60.0).ceil() as i64
    } else {
        0
    };
    c
}

fn row_to_campaign(r: &rusqlite::Row<'_>) -> rusqlite::Result<Campaign> {
    Ok(Campaign {
        id: r.get(0)?,
        name: r.get(1)?,
        template_id: r.get(2)?,
        list_id: r.get(3)?,
        link_url: r.get(4)?,
        profile_id: r.get(5)?,
        assessment_id: r.get(6)?,
        lure_id: r.get(7)?,
        sender_account_id: r.get(8)?,
        rate_per_minute: r.get(9)?,
        status: r.get(10)?,
        mode: r.get(11)?,
        scheduled_at: r.get(12)?,
        send_window_start: r.get(13)?,
        send_window_end: r.get(14)?,
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
        started_at: r.get(17)?,
        finished_at: r.get(18)?,
        pending: 0,
        sent: 0,
        failed: 0,
        total: 0,
        progress_pct: 0.0,
        eta_seconds: 0,
    })
}

pub fn list_campaigns(assessment_id: Option<String>) -> AppResult<Vec<Campaign>> {
    with_db(|conn| {
        let sql = format!("{CAMPAIGN_SELECT} FROM campaigns");
        let campaigns: Vec<Campaign> = if let Some(aid) = assessment_id.filter(|s| !s.is_empty()) {
            let mut stmt = conn.prepare(&format!(
                "{sql} WHERE assessment_id = ?1 ORDER BY updated_at DESC"
            ))?;
            let rows = stmt.query_map(params![aid], row_to_campaign)?;
            rows.filter_map(|r| r.ok()).collect()
        } else {
            let mut stmt = conn.prepare(&format!("{sql} ORDER BY updated_at DESC"))?;
            let rows = stmt.query_map([], row_to_campaign)?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let mut out = Vec::new();
        for c in campaigns {
            let (pending, sent, failed) = attempt_counts(conn, &c.id)?;
            out.push(with_progress(c, pending, sent, failed));
        }
        Ok(out)
    })
}

pub fn get_campaign(id: &str) -> AppResult<Option<Campaign>> {
    with_db(|conn| {
        let sql = format!("{CAMPAIGN_SELECT} FROM campaigns WHERE id = ?1");
        let camp = conn
            .query_row(&sql, params![id], row_to_campaign)
            .optional()?;
        match camp {
            Some(c) => {
                let (pending, sent, failed) = attempt_counts(conn, &c.id)?;
                Ok(Some(with_progress(c, pending, sent, failed)))
            }
            None => Ok(None),
        }
    })
}

/// Delivery settings bound to a campaign's saved sender, falling back to the
/// active account when the campaign has none or its account was removed.
fn settings_for_campaign(camp: &Campaign) -> AppResult<mail::MailSettings> {
    if !camp.sender_account_id.is_empty() {
        if let Some(s) = mail::get_settings_for_account(&camp.sender_account_id)? {
            return Ok(s);
        }
    }
    mail::get_mail_settings()
}

fn load_snapshot(campaign_id: &str) -> AppResult<Option<CampaignSnapshot>> {
    with_db(|conn| {
        let raw: Option<String> = conn
            .query_row(
                "SELECT snapshot_json FROM campaigns WHERE id = ?1",
                params![campaign_id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(raw
            .filter(|s| !s.trim().is_empty() && s.trim() != "{}")
            .and_then(|s| serde_json::from_str::<CampaignSnapshot>(&s).ok()))
    })
}

/// Prefer the frozen snapshot content; fall back to the live template.
fn subject_body_for(camp: &Campaign) -> AppResult<(String, String)> {
    if let Some(s) = load_snapshot(&camp.id)? {
        if !s.subject.is_empty() || !s.html_body.is_empty() {
            return Ok((s.subject, s.html_body));
        }
    }
    let t = load_template(&camp.template_id)?;
    Ok((t.subject, t.html_body))
}

fn build_snapshot(
    template_id: &str,
    sender_account_id: &str,
    link: &str,
    lure_id: &str,
) -> CampaignSnapshot {
    let mut snap = CampaignSnapshot {
        link_url: link.to_string(),
        lure_id: lure_id.to_string(),
        captured_at: now_iso(),
        ..Default::default()
    };
    if let Ok(Some(t)) = mail::get_template(template_id) {
        snap.subject = t.subject;
        snap.html_body = t.html_body;
    }
    if !sender_account_id.is_empty() {
        if let Ok(accounts) = mail::list_mail_accounts() {
            if let Some(a) = accounts.into_iter().find(|a| a.id == sender_account_id) {
                snap.sender_account_id = a.id;
                snap.sender_label = a.label;
                snap.from_email = a.from_email;
                snap.provider = a.provider;
            }
        }
    }
    if snap.from_email.is_empty() {
        if let Ok(s) = mail::get_mail_settings() {
            snap.from_email = s.from_email;
            if snap.provider.is_empty() {
                snap.provider = s.provider;
            }
        }
    }
    if !lure_id.is_empty() {
        if let Ok(Some(l)) = lure_ops::get_lure(lure_id) {
            snap.lure_url = l.lure_url;
        }
    }
    snap
}

pub fn create_campaign(req: CreateCampaign) -> AppResult<Campaign> {
    crate::aup::require_aup()?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    let rate = req.rate_per_minute.unwrap_or(30).clamp(1, 600);
    let template_id = req.template_id.clone();
    let list_id = req.list_id.clone();
    let name = req.name.trim().to_string();
    let lure_id = req.lure_id.clone().unwrap_or_default();
    let mode = match req.mode.clone().unwrap_or_default().trim() {
        "awareness" => "awareness".to_string(),
        _ => "aitm".to_string(),
    };
    let window_start = req.send_window_start.clone().unwrap_or_default();
    let window_end = req.send_window_end.clone().unwrap_or_default();
    let scheduled_at = req
        .scheduled_at
        .clone()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let mut profile_id = req.profile_id.clone().unwrap_or_default();
    let mut link = req.link_url.trim().to_string();

    if !lure_id.is_empty() {
        if let Some(lure) = lure_ops::get_lure(&lure_id)? {
            if profile_id.is_empty() {
                profile_id = lure.profile_id.clone();
            }
            if link.is_empty() && !lure.lure_url.is_empty() {
                link = lure.lure_url.clone();
            }
        }
    }

    if link.is_empty() {
        return Err(AppError::msg(
            "Campaign link URL is required (AiTM tracked link or awareness training URL)",
        ));
    }

    let mut assessment_id = req.assessment_id.clone().unwrap_or_default();
    if assessment_id.is_empty() && !profile_id.is_empty() {
        if let Some(p) = db::get_profile(&profile_id)? {
            assessment_id = p.assessment_id;
        }
    }

    let sender_account_id = req
        .sender_account_id
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| mail::active_mail_account_id().ok().flatten())
        .unwrap_or_default();

    let status = if scheduled_at
        .as_deref()
        .and_then(parse_iso)
        .map(|dt| dt > Utc::now().naive_utc())
        .unwrap_or(false)
    {
        "scheduled"
    } else {
        "draft"
    };

    let link_for_tracking = link.clone();
    let snapshot = build_snapshot(&template_id, &sender_account_id, &link, &lure_id);
    let snapshot_json = serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".into());

    with_db(|conn| {
        let tmpl_ok: bool = conn
            .query_row(
                "SELECT 1 FROM email_templates WHERE id = ?1",
                params![&template_id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !tmpl_ok {
            return Err(AppError::msg("template not found"));
        }
        let list_ok: bool = conn
            .query_row(
                "SELECT 1 FROM recipient_lists WHERE id = ?1",
                params![&list_id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !list_ok {
            return Err(AppError::msg("recipient list not found"));
        }

        conn.execute(
            "INSERT INTO campaigns(id, name, template_id, list_id, link_url, profile_id,
                 assessment_id, lure_id, sender_account_id, rate_per_minute, status, mode,
                 scheduled_at, send_window_start, send_window_end, snapshot_json,
                 created_at, updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?17)",
            params![
                id,
                name,
                template_id,
                &list_id,
                link,
                profile_id,
                assessment_id,
                lure_id,
                sender_account_id,
                rate,
                status,
                mode,
                scheduled_at,
                window_start,
                window_end,
                snapshot_json,
                now
            ],
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, list_id, email, first_name, last_name, extras, suppressed
             FROM recipients WHERE list_id = ?1 AND suppressed = 0",
        )?;
        let recipients: Vec<(i64, String)> = stmt
            .query_map(params![&list_id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        if recipients.is_empty() {
            return Err(AppError::msg("recipient list is empty"));
        }
        for (rid, email) in recipients {
            let token = new_tracking_token();
            let tracked_url = append_tracking_param(&link_for_tracking, &token)?;
            conn.execute(
                "INSERT INTO campaign_attempts(campaign_id, recipient_id, email, status,
                     tracking_token, tracked_url)
                 VALUES(?1,?2,?3,'pending',?4,?5)",
                params![id, rid, email, token, tracked_url],
            )?;
        }
        Ok(())
    })?;

    get_campaign(&id)?.ok_or_else(|| AppError::msg("campaign missing after create"))
}

pub fn list_attempts(campaign_id: String) -> AppResult<Vec<CampaignAttempt>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, campaign_id, recipient_id, email, status, error, sent_at,
                    tracking_token, tracked_url, provider_message_id, delivered_at,
                    opened_at, clicked_at, bounced_at, complained_at, bounce_reason
             FROM campaign_attempts WHERE campaign_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![campaign_id], |r| {
            Ok(CampaignAttempt {
                id: r.get(0)?,
                campaign_id: r.get(1)?,
                recipient_id: r.get(2)?,
                email: r.get(3)?,
                status: r.get(4)?,
                error: r.get(5)?,
                sent_at: r.get(6)?,
                tracking_token: r.get(7)?,
                tracked_url: r.get(8)?,
                provider_message_id: r.get(9)?,
                delivered_at: r.get(10)?,
                opened_at: r.get(11)?,
                clicked_at: r.get(12)?,
                bounced_at: r.get(13)?,
                complained_at: r.get(14)?,
                bounce_reason: r.get(15)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    })
}

pub fn start_campaign(id: String) -> AppResult<Campaign> {
    crate::aup::require_aup()?;
    {
        let guard = RUNNER.lock().unwrap();
        if let Some(r) = guard.as_ref() {
            if r.id != id {
                return Err(AppError::msg(format!(
                    "another campaign is running ({})",
                    r.id
                )));
            }
            return get_campaign(&id)?.ok_or_else(|| AppError::msg("campaign not found"));
        }
    }

    let camp = get_campaign(&id)?.ok_or_else(|| AppError::msg("campaign not found"))?;
    if camp.status == "completed" && camp.pending == 0 {
        return Err(AppError::msg(
            "campaign already completed — use Retry failed to requeue errors",
        ));
    }

    let now = now_iso();
    with_db(|conn| {
        conn.execute(
            "UPDATE campaigns SET status = 'running', started_at = COALESCE(started_at, ?1),
                 updated_at = ?1, finished_at = NULL WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    })?;

    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut guard = RUNNER.lock().unwrap();
        *guard = Some(RunningCampaign {
            id: id.clone(),
            cancel: cancel.clone(),
        });
    }

    let campaign_id = id.clone();
    thread::spawn(move || {
        let _ = run_loop(&campaign_id, cancel);
        let mut guard = RUNNER.lock().unwrap();
        if guard.as_ref().map(|r| r.id == campaign_id).unwrap_or(false) {
            *guard = None;
        }
    });

    get_campaign(&id)?.ok_or_else(|| AppError::msg("campaign not found"))
}

pub fn list_campaigns_for_profile(profile_id: String) -> AppResult<Vec<Campaign>> {
    let all = list_campaigns(None)?;
    Ok(all
        .into_iter()
        .filter(|c| !profile_id.is_empty() && c.profile_id == profile_id)
        .collect())
}

/// Match capture usernames/emails against campaign attempt rows for a profile.
pub fn match_captures_to_sends(profile_id: String) -> AppResult<Vec<CaptureSendMatch>> {
    let camps = list_campaigns_for_profile(profile_id)?;
    let mut out = Vec::new();
    for c in camps {
        let attempts = list_attempts(c.id.clone())?;
        for a in attempts {
            if a.status == "sent" {
                out.push(CaptureSendMatch {
                    campaign_id: c.id.clone(),
                    campaign_name: c.name.clone(),
                    email: a.email.to_ascii_lowercase(),
                    sent_at: a.sent_at.clone(),
                    status: a.status.clone(),
                });
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSendMatch {
    pub campaign_id: String,
    pub campaign_name: String,
    pub email: String,
    pub sent_at: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureAttribution {
    pub evilginx_session_id: i64,
    pub campaign_id: String,
    pub campaign_name: String,
    pub email: String,
    pub tracking_token: String,
    /// "token" (deterministic pk match) or "email" (username fallback)
    pub matched_by: String,
}

/// Deterministically attribute captured sessions to campaigns/recipients using
/// the per-attempt tracking token embedded in the lure link, falling back to a
/// username/email match when no token is present in the landing URL.
pub fn attribute_captures(profile_id: String) -> AppResult<Vec<CaptureAttribution>> {
    let camps = list_campaigns_for_profile(profile_id.clone())?;
    let mut by_token: std::collections::HashMap<String, (String, String, String)> =
        std::collections::HashMap::new();
    let mut by_email: std::collections::HashMap<String, (String, String, String)> =
        std::collections::HashMap::new();
    for c in &camps {
        for a in list_attempts(c.id.clone())? {
            if !a.tracking_token.is_empty() {
                by_token.insert(
                    a.tracking_token.clone(),
                    (c.id.clone(), c.name.clone(), a.email.clone()),
                );
            }
            by_email.entry(a.email.to_ascii_lowercase()).or_insert((
                c.id.clone(),
                c.name.clone(),
                a.email.clone(),
            ));
        }
    }

    let _ = crate::sessions::sync_captures(profile_id.clone());
    let captures = crate::db::list_captures(&profile_id).unwrap_or_default();
    let mut out = Vec::new();
    for cap in captures {
        let landing = cap
            .data
            .get("landing_url")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let pk = pk_from_landing_url(landing);
        let mut hit: Option<(String, String, String)> = None;
        let mut matched_by = "";
        if let Some(tok) = pk.as_ref() {
            if let Some(v) = by_token.get(tok) {
                hit = Some(v.clone());
                matched_by = "token";
            }
        }
        if hit.is_none() {
            let user = cap
                .data
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            if !user.is_empty() {
                if let Some(v) = by_email.get(&user) {
                    hit = Some(v.clone());
                    matched_by = "email";
                }
            }
        }
        if let Some((cid, cname, email)) = hit {
            out.push(CaptureAttribution {
                evilginx_session_id: cap.evilginx_session_id,
                campaign_id: cid,
                campaign_name: cname,
                email,
                tracking_token: pk.unwrap_or_default(),
                matched_by: matched_by.to_string(),
            });
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunnelAttempt {
    pub id: i64,
    pub email: String,
    pub status: String,
    pub error: String,
    pub sent_at: Option<String>,
    pub delivered: bool,
    pub opened: bool,
    pub clicked: bool,
    pub bounced: bool,
    pub complained: bool,
    pub bounce_reason: String,
    /// True when a capture username matches this recipient (submit).
    pub captured: bool,
    pub capture_session_id: Option<i64>,
    pub landing_url: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignFunnel {
    pub campaign: Campaign,
    pub sent: i64,
    pub failed: i64,
    pub pending: i64,
    pub delivered: i64,
    pub opened: i64,
    pub clicked: i64,
    pub bounced: i64,
    pub complained: i64,
    /// Sessions whose landing_url path matches the campaign lure link.
    pub lure_hits: i64,
    pub captures: i64,
    pub attempts: Vec<FunnelAttempt>,
}

fn lure_path_from_link(link_url: &str) -> Option<String> {
    let url = url::Url::parse(link_url).ok()?;
    let path = url.path().to_string();
    if path.is_empty() || path == "/" {
        None
    } else {
        Some(path)
    }
}

fn capture_has_creds(data: &serde_json::Value) -> bool {
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
    let tokens = data.get("tokens").and_then(|v| v.as_object());
    let custom = data.get("custom").and_then(|v| v.as_object());
    let body = data.get("body_tokens").and_then(|v| v.as_object());
    !user.is_empty()
        || !pass.is_empty()
        || tokens.map(|t| !t.is_empty()).unwrap_or(false)
        || custom.map(|t| !t.is_empty()).unwrap_or(false)
        || body.map(|t| !t.is_empty()).unwrap_or(false)
}

/// Delivery → open → click → capture funnel for a campaign.
pub fn campaign_funnel(campaign_id: String) -> AppResult<CampaignFunnel> {
    let campaign =
        get_campaign(&campaign_id)?.ok_or_else(|| AppError::msg("campaign not found"))?;
    let attempts = list_attempts(campaign_id.clone())?;
    let lure_path = lure_path_from_link(&campaign.link_url);
    let tracking_tokens: std::collections::HashSet<String> = attempts
        .iter()
        .filter(|a| !a.tracking_token.is_empty())
        .map(|a| a.tracking_token.clone())
        .collect();

    let captures = if campaign.profile_id.is_empty() {
        vec![]
    } else {
        let _ = crate::sessions::sync_captures(campaign.profile_id.clone());
        crate::db::list_captures(&campaign.profile_id).unwrap_or_default()
    };

    let mut lure_hits = 0i64;
    let mut capture_count = 0i64;
    let mut by_email: std::collections::HashMap<String, (i64, Option<String>, Option<String>)> =
        std::collections::HashMap::new();

    for c in &captures {
        let landing = c
            .data
            .get("landing_url")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let path_hit = lure_path
            .as_ref()
            .map(|p| landing.contains(p.as_str()))
            .unwrap_or(false);
        let pk_hit = pk_from_landing_url(landing)
            .map(|pk| tracking_tokens.contains(&pk))
            .unwrap_or(false);
        if path_hit
            || pk_hit
            || (!campaign.link_url.is_empty() && landing.contains(&campaign.link_url))
        {
            lure_hits += 1;
        }
        if capture_has_creds(&c.data) {
            capture_count += 1;
        }
        let user = c
            .data
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if !user.is_empty() {
            by_email.insert(
                user,
                (
                    c.evilginx_session_id,
                    if landing.is_empty() {
                        None
                    } else {
                        Some(landing.to_string())
                    },
                    c.data
                        .get("username")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                ),
            );
        }
    }

    let mut delivered = 0i64;
    let mut opened = 0i64;
    let mut clicked = 0i64;
    let mut bounced = 0i64;
    let mut complained = 0i64;

    let funnel_attempts: Vec<FunnelAttempt> = attempts
        .into_iter()
        .map(|a| {
            let email_l = a.email.to_ascii_lowercase();
            let hit = by_email.get(&email_l);
            let is_delivered = a.delivered_at.is_some() || a.status == "sent";
            let is_opened = a.opened_at.is_some();
            let is_clicked = a.clicked_at.is_some();
            let is_bounced = a.bounced_at.is_some();
            let is_complained = a.complained_at.is_some();
            if is_delivered {
                delivered += 1;
            }
            if is_opened {
                opened += 1;
            }
            if is_clicked {
                clicked += 1;
            }
            if is_bounced {
                bounced += 1;
            }
            if is_complained {
                complained += 1;
            }
            FunnelAttempt {
                id: a.id,
                email: a.email,
                status: a.status.clone(),
                error: a.error,
                sent_at: a.sent_at,
                delivered: is_delivered,
                opened: is_opened,
                clicked: is_clicked,
                bounced: is_bounced,
                complained: is_complained,
                bounce_reason: a.bounce_reason,
                captured: hit.is_some(),
                capture_session_id: hit.map(|h| h.0),
                landing_url: hit.and_then(|h| h.1.clone()),
                username: hit.and_then(|h| h.2.clone()),
            }
        })
        .collect();

    Ok(CampaignFunnel {
        sent: campaign.sent,
        failed: campaign.failed,
        pending: campaign.pending,
        delivered,
        opened,
        clicked,
        bounced,
        complained,
        lure_hits,
        captures: capture_count,
        attempts: funnel_attempts,
        campaign,
    })
}

/// Requeue failed attempts as pending so the campaign can resume.
pub fn retry_failed(id: String) -> AppResult<Campaign> {
    let now = now_iso();
    let n = with_db(|conn| {
        let updated = conn.execute(
            "UPDATE campaign_attempts SET status = 'pending', error = '', sent_at = NULL,
                 bounced_at = NULL, bounce_reason = ''
             WHERE campaign_id = ?1 AND status = 'failed'",
            params![id],
        )?;
        if updated > 0 {
            conn.execute(
                "UPDATE campaigns SET status = 'paused', finished_at = NULL, updated_at = ?1 WHERE id = ?2",
                params![now, id],
            )?;
        }
        Ok(updated)
    })?;
    if n == 0 {
        return Err(AppError::msg("no failed attempts to retry"));
    }
    get_campaign(&id)?.ok_or_else(|| AppError::msg("campaign not found"))
}

pub fn stop_campaign(id: String) -> AppResult<Campaign> {
    {
        let guard = RUNNER.lock().unwrap();
        if let Some(r) = guard.as_ref() {
            if r.id == id {
                r.cancel.store(true, Ordering::SeqCst);
            }
        }
    }
    let now = now_iso();
    with_db(|conn| {
        conn.execute(
            "UPDATE campaigns SET status = 'paused', updated_at = ?1
             WHERE id = ?2 AND status IN ('running','scheduled')",
            params![now, id],
        )?;
        Ok(())
    })?;
    get_campaign(&id)?.ok_or_else(|| AppError::msg("campaign not found"))
}

/// Permanently remove a campaign and its attempts. Delivery-event state lives on
/// the attempt rows, so clearing attempts + the campaign row is a complete
/// delete. Refuses the campaign that is actively sending.
pub fn delete_campaign(id: String) -> AppResult<()> {
    {
        let guard = RUNNER.lock().unwrap();
        if let Some(r) = guard.as_ref() {
            if r.id == id {
                return Err(AppError::msg(
                    "campaign is running — stop it first, then delete",
                ));
            }
        }
    }
    with_db(|conn| {
        conn.execute(
            "DELETE FROM campaign_attempts WHERE campaign_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM campaigns WHERE id = ?1", params![id])?;
        Ok(())
    })
}

fn load_template(id: &str) -> AppResult<EmailTemplate> {
    mail::get_template(id)?.ok_or_else(|| AppError::msg("template not found"))
}

fn load_recipient(id: i64) -> AppResult<Recipient> {
    with_db(|conn| {
        conn.query_row(
            "SELECT id, list_id, email, first_name, last_name, extras, suppressed
             FROM recipients WHERE id = ?1",
            params![id],
            |r| {
                let extras_s: String = r.get(5)?;
                Ok(Recipient {
                    id: r.get(0)?,
                    list_id: r.get(1)?,
                    email: r.get(2)?,
                    first_name: r.get(3)?,
                    last_name: r.get(4)?,
                    extras: serde_json::from_str(&extras_s).unwrap_or_default(),
                    suppressed: r.get::<_, i64>(6)? != 0,
                })
            },
        )
        .map_err(|e| e.into())
    })
}

fn set_status(campaign_id: &str, status: &str) {
    let now = now_iso();
    let _ = with_db(|conn| {
        conn.execute(
            "UPDATE campaigns SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, now, campaign_id],
        )?;
        Ok(())
    });
}

/// Sleep in ~1s slices so the runner stays responsive to cancel.
fn interruptible_sleep(cancel: &Arc<AtomicBool>, total: Duration) {
    let mut remaining = total;
    let step = Duration::from_millis(1000);
    while remaining > Duration::ZERO {
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        let s = remaining.min(step);
        thread::sleep(s);
        remaining = remaining.saturating_sub(s);
    }
}

fn run_loop(campaign_id: &str, cancel: Arc<AtomicBool>) -> AppResult<()> {
    let camp = get_campaign(campaign_id)?.ok_or_else(|| AppError::msg("missing campaign"))?;
    let (base_subject, base_body) = subject_body_for(&camp)?;
    let settings = settings_for_campaign(&camp)?;
    let delay_ms = (60_000u64 / camp.rate_per_minute.max(1) as u64).max(50);

    // Honor a scheduled launch time before doing any work.
    if let Some(target) = camp.scheduled_at.as_deref().and_then(parse_iso) {
        if target > Utc::now().naive_utc() {
            set_status(campaign_id, "scheduled");
            while Utc::now().naive_utc() < target {
                if cancel.load(Ordering::SeqCst) {
                    set_status(campaign_id, "paused");
                    return Ok(());
                }
                interruptible_sleep(&cancel, Duration::from_secs(5));
            }
            if cancel.load(Ordering::SeqCst) {
                set_status(campaign_id, "paused");
                return Ok(());
            }
            set_status(campaign_id, "running");
        }
    }

    loop {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        // Respect the configured send window; wait rather than send outside it.
        if !in_send_window(&camp.send_window_start, &camp.send_window_end) {
            interruptible_sleep(&cancel, Duration::from_secs(30));
            continue;
        }

        let next: Option<(i64, i64, String, String)> = with_db(|conn| {
            let row = conn.query_row(
                "SELECT id, recipient_id, email, tracked_url FROM campaign_attempts
                 WHERE campaign_id = ?1 AND status = 'pending' ORDER BY id ASC LIMIT 1",
                params![campaign_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            );
            match row {
                Ok(v) => Ok(Some(v)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.into()),
            }
        })?;

        let Some((attempt_id, recipient_id, email, tracked_url)) = next else {
            let now = now_iso();
            with_db(|conn| {
                conn.execute(
                    "UPDATE campaigns SET status = 'completed', finished_at = ?1, updated_at = ?1 WHERE id = ?2",
                    params![now, campaign_id],
                )?;
                Ok(())
            })?;
            break;
        };

        let recipient = load_recipient(recipient_id)?;
        let link = if tracked_url.trim().is_empty() {
            camp.link_url.clone()
        } else {
            tracked_url
        };
        let vars = mail::recipient_vars(&recipient, &link);
        let subject = mail::merge_tags(&base_subject, &vars);
        let body = mail::merge_tags(&base_body, &vars);

        match mail::send_message(&settings, &email, &subject, &body) {
            Ok(receipt) => {
                let now = now_iso();
                with_db(|conn| {
                    conn.execute(
                        "UPDATE campaign_attempts SET status = 'sent', error = '', sent_at = ?1,
                             provider_message_id = CASE WHEN ?2 != '' THEN ?2 ELSE provider_message_id END
                         WHERE id = ?3",
                        params![now, receipt.message_id, attempt_id],
                    )?;
                    conn.execute(
                        "UPDATE campaigns SET updated_at = ?1 WHERE id = ?2",
                        params![now, campaign_id],
                    )?;
                    Ok(())
                })?;
            }
            Err(e) => {
                let now = now_iso();
                let err = e.to_string();
                with_db(|conn| {
                    conn.execute(
                        "UPDATE campaign_attempts SET status = 'failed', error = ?1, sent_at = ?2 WHERE id = ?3",
                        params![err, now, attempt_id],
                    )?;
                    conn.execute(
                        "UPDATE campaigns SET updated_at = ?1 WHERE id = ?2",
                        params![now, campaign_id],
                    )?;
                    Ok(())
                })?;
            }
        }

        interruptible_sleep(&cancel, Duration::from_millis(delay_ms));
    }

    // If paused via cancel, status already set; if completed, set above
    let status = get_campaign(campaign_id)?
        .map(|c| c.status)
        .unwrap_or_default();
    if status == "running" {
        let now = now_iso();
        let _ = with_db(|conn| {
            conn.execute(
                "UPDATE campaigns SET status = 'paused', updated_at = ?1 WHERE id = ?2",
                params![now, campaign_id],
            )?;
            Ok(())
        });
    }
    Ok(())
}

/// Send a single test message using the campaign's bound sender + content.
/// Does not create or mutate production attempts.
pub fn send_campaign_test(campaign_id: String, to: String) -> AppResult<SendReceipt> {
    crate::aup::require_aup()?;
    let to = to.trim().to_string();
    if to.is_empty() || !to.contains('@') {
        return Err(AppError::msg("Enter a valid test recipient email"));
    }
    let camp = get_campaign(&campaign_id)?.ok_or_else(|| AppError::msg("campaign not found"))?;
    let (base_subject, base_body) = subject_body_for(&camp)?;
    let settings = settings_for_campaign(&camp)?;
    let token = new_tracking_token();
    let link =
        append_tracking_param(&camp.link_url, &token).unwrap_or_else(|_| camp.link_url.clone());
    let recipient = Recipient {
        id: 0,
        list_id: String::new(),
        email: to.clone(),
        first_name: "Test".into(),
        last_name: "Recipient".into(),
        extras: serde_json::json!({}),
        suppressed: false,
    };
    let vars = mail::recipient_vars(&recipient, &link);
    let subject = format!("[TEST] {}", mail::merge_tags(&base_subject, &vars));
    let body = mail::merge_tags(&base_body, &vars);
    mail::send_message(&settings, &to, &subject, &body)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCheck {
    pub id: String,
    pub label: String,
    pub ok: bool,
    pub blocking: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignReview {
    pub campaign_id: String,
    pub ready: bool,
    pub checks: Vec<ReviewCheck>,
}

/// Preflight a draft before launch: sender, content, recipients, lure, link.
pub fn campaign_review(campaign_id: String) -> AppResult<CampaignReview> {
    let camp = get_campaign(&campaign_id)?.ok_or_else(|| AppError::msg("campaign not found"))?;
    let mut checks: Vec<ReviewCheck> = Vec::new();

    let aup_ok = crate::aup::get_aup_status()
        .map(|s| s.accepted)
        .unwrap_or(false);
    checks.push(ReviewCheck {
        id: "aup".into(),
        label: "Authorized-use policy accepted".into(),
        ok: aup_ok,
        blocking: true,
        detail: if aup_ok {
            "Accepted".into()
        } else {
            "Accept the AUP before sending".into()
        },
    });

    let settings = settings_for_campaign(&camp)?;
    let sender_ok = !settings.from_email.trim().is_empty();
    checks.push(ReviewCheck {
        id: "sender".into(),
        label: "Sending profile configured".into(),
        ok: sender_ok,
        blocking: true,
        detail: if sender_ok {
            format!("{} via {}", settings.from_email, settings.provider)
        } else {
            "No From address on the bound/active sender".into()
        },
    });

    let (subject, body) = subject_body_for(&camp)?;
    let subject_ok = !subject.trim().is_empty();
    checks.push(ReviewCheck {
        id: "subject".into(),
        label: "Subject line present".into(),
        ok: subject_ok,
        blocking: true,
        detail: if subject_ok {
            subject.clone()
        } else {
            "Template subject is empty".into()
        },
    });

    let has_link = body.contains("{{link}}") || body.contains("{{ link }}");
    checks.push(ReviewCheck {
        id: "link_tag".into(),
        label: "Body contains a {{link}} tag".into(),
        ok: has_link,
        blocking: false,
        detail: if has_link {
            "Recipients get a tracked link".into()
        } else {
            "No {{link}} merge tag — recipients won't get the lure link".into()
        },
    });

    let link_ok = url::Url::parse(&camp.link_url).is_ok();
    checks.push(ReviewCheck {
        id: "link_url".into(),
        label: "Campaign link URL is valid".into(),
        ok: link_ok,
        blocking: true,
        detail: if link_ok {
            camp.link_url.clone()
        } else {
            "Link URL does not parse".into()
        },
    });

    let recipients = camp.total;
    let recipients_ok = recipients > 0;
    checks.push(ReviewCheck {
        id: "recipients".into(),
        label: "Recipients queued".into(),
        ok: recipients_ok,
        blocking: true,
        detail: format!("{recipients} recipient(s)"),
    });

    if !camp.lure_id.is_empty() {
        let lure = lure_ops::get_lure(&camp.lure_id)?;
        let (ok, detail) = match lure {
            Some(l) if !l.paused => (true, format!("{} (active)", l.name)),
            Some(l) => (false, format!("{} is paused", l.name)),
            None => (false, "Bound lure no longer exists".into()),
        };
        checks.push(ReviewCheck {
            id: "lure".into(),
            label: "Bound lure is active".into(),
            ok,
            blocking: false,
            detail,
        });
    }

    if camp.mode == "aitm" {
        let profile_ok = !camp.profile_id.is_empty();
        checks.push(ReviewCheck {
            id: "target".into(),
            label: "AiTM target bound".into(),
            ok: profile_ok,
            blocking: false,
            detail: if profile_ok {
                camp.profile_id.clone()
            } else {
                "No target bound — captures won't attribute automatically".into()
            },
        });
    }

    let ready = checks.iter().all(|c| c.ok || !c.blocking);
    Ok(CampaignReview {
        campaign_id,
        ready,
        checks,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScheduleReq {
    pub campaign_id: String,
    pub scheduled_at: Option<String>,
    pub send_window_start: Option<String>,
    pub send_window_end: Option<String>,
    pub rate_per_minute: Option<i64>,
    pub mode: Option<String>,
}

/// Update scheduling/mode on a not-yet-completed campaign.
pub fn update_campaign_schedule(req: UpdateScheduleReq) -> AppResult<Campaign> {
    let camp =
        get_campaign(&req.campaign_id)?.ok_or_else(|| AppError::msg("campaign not found"))?;
    let scheduled_at = req.scheduled_at.map(|s| s.trim().to_string()).map(|s| {
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    });
    let now = now_iso();
    with_db(|conn| {
        if let Some(sched) = &scheduled_at {
            conn.execute(
                "UPDATE campaigns SET scheduled_at = ?1, updated_at = ?2 WHERE id = ?3",
                params![sched, now, req.campaign_id],
            )?;
        }
        if let Some(ws) = &req.send_window_start {
            conn.execute(
                "UPDATE campaigns SET send_window_start = ?1, updated_at = ?2 WHERE id = ?3",
                params![ws.trim(), now, req.campaign_id],
            )?;
        }
        if let Some(we) = &req.send_window_end {
            conn.execute(
                "UPDATE campaigns SET send_window_end = ?1, updated_at = ?2 WHERE id = ?3",
                params![we.trim(), now, req.campaign_id],
            )?;
        }
        if let Some(rate) = req.rate_per_minute {
            conn.execute(
                "UPDATE campaigns SET rate_per_minute = ?1, updated_at = ?2 WHERE id = ?3",
                params![rate.clamp(1, 600), now, req.campaign_id],
            )?;
        }
        if let Some(mode) = &req.mode {
            let mode = if mode.trim() == "awareness" {
                "awareness"
            } else {
                "aitm"
            };
            conn.execute(
                "UPDATE campaigns SET mode = ?1, updated_at = ?2 WHERE id = ?3",
                params![mode, now, req.campaign_id],
            )?;
        }
        // Reflect a future schedule in status when still a draft.
        if camp.status == "draft" || camp.status == "scheduled" {
            let is_future = scheduled_at
                .as_ref()
                .and_then(|o| o.as_deref())
                .and_then(parse_iso)
                .map(|dt| dt > Utc::now().naive_utc())
                .unwrap_or(false);
            let next = if is_future { "scheduled" } else { "draft" };
            conn.execute(
                "UPDATE campaigns SET status = ?1, updated_at = ?2 WHERE id = ?3 AND status IN ('draft','scheduled')",
                params![next, now, req.campaign_id],
            )?;
        }
        Ok(())
    })?;
    get_campaign(&req.campaign_id)?.ok_or_else(|| AppError::msg("campaign not found"))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsImport {
    pub parsed: i64,
    pub matched: i64,
    pub updated: i64,
    pub unmatched: i64,
}

struct NormEvent {
    email: String,
    kind: String,
    message_id: String,
    ts: String,
    reason: String,
}

fn classify_event(raw: &str) -> Option<&'static str> {
    let r = raw.trim().to_ascii_lowercase();
    let r = r.strip_prefix("email.").unwrap_or(&r);
    match r {
        "delivered" | "delivery" => Some("delivered"),
        "open" | "opened" => Some("opened"),
        "click" | "clicked" => Some("clicked"),
        "bounce" | "bounced" | "dropped" | "failed" | "hard_bounce" | "soft_bounce"
        | "delivery_delay" => Some("bounced"),
        "complaint" | "complained" | "spam" | "spamreport" | "spam_report" => Some("complained"),
        _ => None,
    }
}

fn str_field(v: &serde_json::Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

/// Extract the first email address from a value that may be a string or array.
fn first_email(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(a)) => a
            .iter()
            .find_map(|x| x.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn normalize_event(v: &serde_json::Value) -> Option<NormEvent> {
    // Determine the event type across provider shapes.
    let raw_kind = str_field(
        v,
        &[
            "event",
            "type",
            "RecordType",
            "notificationType",
            "eventType",
        ],
    );
    let kind = classify_event(&raw_kind)?;

    // Email across common shapes (incl. nested provider payloads).
    let mut email = str_field(v, &["email", "recipient", "Recipient", "Email"]);
    if email.is_empty() {
        if let Some(data) = v.get("data") {
            email = str_field(data, &["email", "recipient"]);
            if email.is_empty() {
                email = first_email(data.get("to"));
            }
        }
    }
    if email.is_empty() {
        if let Some(ed) = v.get("event-data") {
            email = str_field(ed, &["recipient"]);
        }
    }
    if email.is_empty() {
        if let Some(mail) = v.get("mail") {
            email = first_email(mail.get("destination"));
        }
    }
    if email.is_empty() {
        email = first_email(v.get("to"));
    }

    let mut message_id = str_field(
        v,
        &[
            "messageId",
            "message_id",
            "MessageID",
            "sg_message_id",
            "email_id",
        ],
    );
    if message_id.is_empty() {
        if let Some(data) = v.get("data") {
            message_id = str_field(data, &["email_id", "message_id", "id"]);
        }
    }
    if message_id.is_empty() {
        if let Some(mail) = v.get("mail") {
            message_id = str_field(mail, &["messageId"]);
        }
    }

    let ts = str_field(v, &["timestamp", "ts", "created_at", "DeliveredAt", "date"]);
    let reason = str_field(
        v,
        &["reason", "Details", "description", "error", "Description"],
    );

    if email.is_empty() && message_id.is_empty() {
        return None;
    }
    Some(NormEvent {
        email: email.to_ascii_lowercase(),
        kind: kind.to_string(),
        message_id,
        ts,
        reason,
    })
}

fn collect_events(root: &serde_json::Value, out: &mut Vec<NormEvent>) {
    match root {
        serde_json::Value::Array(a) => {
            for v in a {
                collect_events(v, out);
            }
        }
        serde_json::Value::Object(_) => {
            // Common envelopes: {events:[...]} / {data:[...]} / {Records:[...]}
            for key in ["events", "data", "Records", "items", "messages"] {
                if let Some(serde_json::Value::Array(a)) = root.get(key) {
                    for v in a {
                        collect_events(v, out);
                    }
                    return;
                }
            }
            if let Some(ev) = normalize_event(root) {
                out.push(ev);
            }
        }
        _ => {}
    }
}

/// Ingest provider delivery/open/click/bounce/complaint events (webhook JSON or
/// exported report) and reconcile them against this campaign's attempts.
pub fn import_delivery_events(campaign_id: String, raw: String) -> AppResult<EventsImport> {
    let root: serde_json::Value = serde_json::from_str(raw.trim())
        .map_err(|e| AppError::msg(format!("Events JSON parse failed: {e}")))?;
    let mut events: Vec<NormEvent> = Vec::new();
    collect_events(&root, &mut events);
    let parsed = events.len() as i64;
    if parsed == 0 {
        return Err(AppError::msg(
            "No recognizable delivery events found in the payload",
        ));
    }

    let attempts = list_attempts(campaign_id.clone())?;
    let mut matched = 0i64;
    let mut updated = 0i64;
    let mut unmatched = 0i64;

    with_db(|conn| {
        for ev in events {
            // Match by provider message id first, then by recipient email.
            let attempt_id: Option<i64> = attempts
                .iter()
                .find(|a| !ev.message_id.is_empty() && a.provider_message_id == ev.message_id)
                .or_else(|| {
                    attempts
                        .iter()
                        .rev()
                        .find(|a| !ev.email.is_empty() && a.email.to_ascii_lowercase() == ev.email)
                })
                .map(|a| a.id);
            let Some(aid) = attempt_id else {
                unmatched += 1;
                continue;
            };
            matched += 1;
            let ts = if ev.ts.trim().is_empty() {
                now_iso()
            } else {
                ev.ts.clone()
            };
            let n = match ev.kind.as_str() {
                "delivered" => conn.execute(
                    "UPDATE campaign_attempts SET delivered_at = COALESCE(delivered_at, ?1) WHERE id = ?2",
                    params![ts, aid],
                )?,
                "opened" => conn.execute(
                    "UPDATE campaign_attempts SET opened_at = COALESCE(opened_at, ?1) WHERE id = ?2",
                    params![ts, aid],
                )?,
                "clicked" => conn.execute(
                    "UPDATE campaign_attempts SET clicked_at = COALESCE(clicked_at, ?1),
                         opened_at = COALESCE(opened_at, ?1) WHERE id = ?2",
                    params![ts, aid],
                )?,
                "bounced" => conn.execute(
                    "UPDATE campaign_attempts SET bounced_at = COALESCE(bounced_at, ?1),
                         bounce_reason = CASE WHEN ?2 != '' THEN ?2 ELSE bounce_reason END
                     WHERE id = ?3",
                    params![ts, ev.reason, aid],
                )?,
                "complained" => conn.execute(
                    "UPDATE campaign_attempts SET complained_at = COALESCE(complained_at, ?1) WHERE id = ?2",
                    params![ts, aid],
                )?,
                _ => 0,
            };
            if n > 0 {
                updated += 1;
            }
        }
        let now = now_iso();
        conn.execute(
            "UPDATE campaigns SET updated_at = ?1 WHERE id = ?2",
            params![now, campaign_id],
        )?;
        Ok(())
    })?;

    Ok(EventsImport {
        parsed,
        matched,
        updated,
        unmatched,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    pub email: String,
    pub status: String,
    pub sent_at: Option<String>,
    pub delivered: bool,
    pub opened: bool,
    pub clicked: bool,
    pub bounced: bool,
    pub complained: bool,
    pub captured: bool,
    pub capture_session_id: Option<i64>,
    pub bounce_reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignReport {
    pub campaign: Campaign,
    pub sent: i64,
    pub failed: i64,
    pub pending: i64,
    pub delivered: i64,
    pub opened: i64,
    pub clicked: i64,
    pub bounced: i64,
    pub complained: i64,
    pub lure_hits: i64,
    pub captures: i64,
    pub rows: Vec<ReportRow>,
    pub generated_at: String,
}

pub fn campaign_report(campaign_id: String) -> AppResult<CampaignReport> {
    let funnel = campaign_funnel(campaign_id)?;
    let rows = funnel
        .attempts
        .iter()
        .map(|a| ReportRow {
            email: a.email.clone(),
            status: a.status.clone(),
            sent_at: a.sent_at.clone(),
            delivered: a.delivered,
            opened: a.opened,
            clicked: a.clicked,
            bounced: a.bounced,
            complained: a.complained,
            captured: a.captured,
            capture_session_id: a.capture_session_id,
            bounce_reason: a.bounce_reason.clone(),
        })
        .collect();
    Ok(CampaignReport {
        campaign: funnel.campaign,
        sent: funnel.sent,
        failed: funnel.failed,
        pending: funnel.pending,
        delivered: funnel.delivered,
        opened: funnel.opened,
        clicked: funnel.clicked,
        bounced: funnel.bounced,
        complained: funnel.complained,
        lure_hits: funnel.lure_hits,
        captures: funnel.captures,
        rows,
        generated_at: now_iso(),
    })
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Export a campaign report as `json` or `csv` (per-recipient rows).
pub fn export_campaign_report(campaign_id: String, format: String) -> AppResult<String> {
    let report = campaign_report(campaign_id)?;
    match format.trim().to_ascii_lowercase().as_str() {
        "csv" => {
            let mut out = String::from(
                "email,status,sent_at,delivered,opened,clicked,bounced,complained,captured,bounce_reason\n",
            );
            for r in &report.rows {
                out.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{},{}\n",
                    csv_escape(&r.email),
                    csv_escape(&r.status),
                    csv_escape(r.sent_at.as_deref().unwrap_or("")),
                    r.delivered,
                    r.opened,
                    r.clicked,
                    r.bounced,
                    r.complained,
                    r.captured,
                    csv_escape(&r.bounce_reason),
                ));
            }
            Ok(out)
        }
        _ => serde_json::to_string_pretty(&report)
            .map_err(|e| AppError::msg(format!("report serialize: {e}"))),
    }
}
