use std::collections::HashMap;

use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use regex::Regex;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use mailparse::MailHeaderMap;

use crate::db::{now_iso, with_db};
use crate::error::{AppError, AppResult};

/// BYO delivery settings — SMTP or HTTP ESP adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailSettings {
    /// smtp | ses_smtp | gmail | resend | sendgrid | mailgun | postmark
    pub provider: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    pub use_starttls: bool,
    pub api_key: String,
    /// SES region (us-east-1) or Mailgun region (us|eu)
    pub region: String,
    /// Mailgun sending domain
    pub domain: String,
}

/// Named saved sender (multiple per method).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAccount {
    pub id: String,
    pub label: String,
    pub provider: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    pub use_starttls: bool,
    pub api_key: String,
    pub region: String,
    pub domain: String,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

pub const SECRET_MASK: &str = "••••••••";

/// Mask non-empty secrets for API/CLI responses (empty stays empty).
pub fn mask_secret(s: &str) -> String {
    if s.is_empty() {
        String::new()
    } else {
        SECRET_MASK.into()
    }
}

fn is_masked(s: &str) -> bool {
    s == SECRET_MASK || s.chars().all(|c| c == '•')
}

fn redact_mail_settings(mut s: MailSettings) -> MailSettings {
    s.password = mask_secret(&s.password);
    s.api_key = mask_secret(&s.api_key);
    s
}

fn redact_mail_account(mut a: MailAccount) -> MailAccount {
    a.password = mask_secret(&a.password);
    a.api_key = mask_secret(&a.api_key);
    a
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertMailAccount {
    pub id: Option<String>,
    pub label: String,
    pub provider: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from_email: String,
    pub from_name: Option<String>,
    pub use_starttls: Option<bool>,
    pub api_key: Option<String>,
    pub region: Option<String>,
    pub domain: Option<String>,
    /// If true (default), make this the active sender after save
    pub activate: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedEmailTemplate {
    pub name: String,
    pub subject: String,
    pub html_body: String,
    pub source: String,
    pub message: String,
}

/// Backward-compatible alias used by Tauri commands.
pub type SmtpSettings = MailSettings;

#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub to: String,
    pub subject: String,
    pub html_body: String,
}

impl Default for MailSettings {
    fn default() -> Self {
        Self {
            provider: "smtp".into(),
            host: String::new(),
            port: 587,
            username: String::new(),
            password: String::new(),
            from_email: String::new(),
            from_name: String::new(),
            use_starttls: true,
            api_key: String::new(),
            region: String::new(),
            domain: String::new(),
        }
    }
}

impl MailSettings {
    pub fn normalize(mut self) -> Self {
        let p = self.provider.trim().to_ascii_lowercase();
        self.provider = match p.as_str() {
            "ses" | "ses_smtp" | "amazon_ses" => "ses_smtp".into(),
            "gmail" | "gmail_smtp" | "google" => "gmail".into(),
            "resend" | "sendgrid" | "mailgun" | "postmark" | "smtp" => p,
            "" => "smtp".into(),
            other => other.to_string(),
        };
        if self.provider == "gmail" {
            self.host = "smtp.gmail.com".into();
            self.port = 587;
            self.use_starttls = true;
            // Gmail SMTP auth user is the mailbox address
            if self.username.trim().is_empty() {
                self.username = self.from_email.trim().to_string();
            }
            if self.from_email.trim().is_empty() && !self.username.trim().is_empty() {
                self.from_email = self.username.trim().to_string();
            }
        }
        if self.provider == "ses_smtp" {
            let region = if self.region.trim().is_empty() {
                "us-east-1".to_string()
            } else {
                self.region.trim().to_string()
            };
            self.region = region.clone();
            if self.host.trim().is_empty() || self.host.contains("amazonaws.com") {
                self.host = format!("email-smtp.{region}.amazonaws.com");
            }
            if self.port == 0 {
                self.port = 587;
            }
            self.use_starttls = true;
        }
        self
    }

    pub fn uses_smtp(&self) -> bool {
        matches!(self.provider.as_str(), "smtp" | "ses_smtp" | "gmail")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailTemplate {
    pub id: String,
    pub name: String,
    pub subject: String,
    pub html_body: String,
    /// None = shared library template (assessment_id IS NULL).
    pub assessment_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertTemplate {
    pub id: Option<String>,
    pub name: String,
    pub subject: String,
    pub html_body: String,
    pub assessment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipientList {
    pub id: String,
    pub name: String,
    pub assessment_id: String,
    pub recipient_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipient {
    pub id: i64,
    pub list_id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub extras: serde_json::Value,
    pub suppressed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendReceipt {
    pub to: String,
    pub message: String,
    /// Provider message id when available (used to reconcile delivery events).
    #[serde(default)]
    pub message_id: String,
}

pub trait MailTransport {
    fn send(&self, msg: &OutboundMessage) -> AppResult<SendReceipt>;
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub list_id: String,
    pub imported: usize,
    pub skipped: usize,
}

pub fn get_smtp_settings() -> AppResult<MailSettings> {
    Ok(redact_mail_settings(get_mail_settings()?))
}

fn write_active_smtp(conn: &rusqlite::Connection, settings: &MailSettings) -> AppResult<()> {
    let now = now_iso();
    conn.execute(
        "INSERT INTO smtp_settings(id, host, port, username, password, from_email, from_name,
             use_starttls, provider, api_key, region, domain, updated_at)
         VALUES(1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
           host=excluded.host,
           port=excluded.port,
           username=excluded.username,
           password=excluded.password,
           from_email=excluded.from_email,
           from_name=excluded.from_name,
           use_starttls=excluded.use_starttls,
           provider=excluded.provider,
           api_key=excluded.api_key,
           region=excluded.region,
           domain=excluded.domain,
           updated_at=excluded.updated_at",
        params![
            settings.host.trim(),
            settings.port as i64,
            settings.username,
            settings.password,
            settings.from_email.trim(),
            settings.from_name,
            if settings.use_starttls { 1 } else { 0 },
            settings.provider,
            settings.api_key,
            settings.region,
            settings.domain,
            now,
        ],
    )?;
    Ok(())
}

pub fn active_mail_account_id() -> AppResult<Option<String>> {
    with_db(|conn| Ok(active_account_id(conn)))
}

/// Resolve the delivery settings bound to a specific saved sender account.
/// Returns None when the account no longer exists (caller falls back to active).
pub fn get_settings_for_account(id: &str) -> AppResult<Option<MailSettings>> {
    if id.trim().is_empty() {
        return Ok(None);
    }
    with_db(|conn| {
        let row = conn
            .query_row(
                "SELECT id, label, provider, host, port, username, password, from_email, from_name,
                        use_starttls, api_key, region, domain
                 FROM mail_accounts WHERE id = ?1",
                params![id],
                settings_from_account_row,
            )
            .optional()?;
        Ok(row)
    })
}

fn active_account_id(conn: &rusqlite::Connection) -> Option<String> {
    conn.query_row(
        "SELECT value FROM app_meta WHERE key = 'active_mail_account'",
        [],
        |r| r.get(0),
    )
    .ok()
}

fn settings_from_account_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<MailSettings> {
    Ok(MailSettings {
        host: r.get(3)?,
        port: r.get::<_, i64>(4)? as u16,
        username: r.get(5)?,
        password: r.get(6)?,
        from_email: r.get(7)?,
        from_name: r.get(8)?,
        use_starttls: r.get::<_, i64>(9)? != 0,
        provider: r.get(2)?,
        api_key: r.get(10)?,
        region: r.get(11)?,
        domain: r.get(12)?,
    }
    .normalize())
}

fn migrate_legacy_smtp_to_accounts(conn: &rusqlite::Connection) -> AppResult<()> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM mail_accounts", [], |r| r.get(0))
        .unwrap_or(0);
    if count > 0 {
        return Ok(());
    }
    let legacy = conn.query_row(
        "SELECT host, port, username, password, from_email, from_name, use_starttls,
                COALESCE(provider, 'smtp'), COALESCE(api_key, ''), COALESCE(region, ''),
                COALESCE(domain, '')
         FROM smtp_settings WHERE id = 1",
        [],
        |r| {
            Ok(MailSettings {
                host: r.get(0)?,
                port: r.get::<_, i64>(1)? as u16,
                username: r.get(2)?,
                password: r.get(3)?,
                from_email: r.get(4)?,
                from_name: r.get(5)?,
                use_starttls: r.get::<_, i64>(6)? != 0,
                provider: r.get(7)?,
                api_key: r.get(8)?,
                region: r.get(9)?,
                domain: r.get(10)?,
            })
        },
    );
    let Ok(s) = legacy else {
        return Ok(());
    };
    if s.from_email.trim().is_empty() && s.username.trim().is_empty() && s.password.is_empty() {
        return Ok(());
    }
    let s = s.normalize();
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    let label = if !s.from_email.is_empty() {
        format!("{} · {}", s.provider, s.from_email)
    } else {
        format!("{} sender", s.provider)
    };
    conn.execute(
        "INSERT INTO mail_accounts(id, label, provider, host, port, username, password,
             from_email, from_name, use_starttls, api_key, region, domain, created_at, updated_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?14)",
        params![
            id,
            label,
            s.provider,
            s.host,
            s.port as i64,
            s.username,
            s.password,
            s.from_email,
            s.from_name,
            if s.use_starttls { 1 } else { 0 },
            s.api_key,
            s.region,
            s.domain,
            now,
        ],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO app_meta(key, value) VALUES('active_mail_account', ?1)",
        params![id],
    )?;
    Ok(())
}

pub fn get_mail_settings() -> AppResult<MailSettings> {
    with_db(|conn| {
        migrate_legacy_smtp_to_accounts(conn)?;
        if let Some(aid) = active_account_id(conn) {
            let row = conn.query_row(
                "SELECT id, label, provider, host, port, username, password, from_email, from_name,
                        use_starttls, api_key, region, domain
                 FROM mail_accounts WHERE id = ?1",
                params![aid],
                settings_from_account_row,
            );
            if let Ok(s) = row {
                return Ok(s);
            }
        }
        let row = conn.query_row(
            "SELECT host, port, username, password, from_email, from_name, use_starttls,
                    COALESCE(provider, 'smtp'), COALESCE(api_key, ''), COALESCE(region, ''),
                    COALESCE(domain, '')
             FROM smtp_settings WHERE id = 1",
            [],
            |r| {
                Ok(MailSettings {
                    host: r.get(0)?,
                    port: r.get::<_, i64>(1)? as u16,
                    username: r.get(2)?,
                    password: r.get(3)?,
                    from_email: r.get(4)?,
                    from_name: r.get(5)?,
                    use_starttls: r.get::<_, i64>(6)? != 0,
                    provider: r.get(7)?,
                    api_key: r.get(8)?,
                    region: r.get(9)?,
                    domain: r.get(10)?,
                })
            },
        );
        Ok(row.unwrap_or_default().normalize())
    })
}

pub fn save_smtp_settings(settings: MailSettings) -> AppResult<MailSettings> {
    save_mail_settings(settings)
}

/// Save as the active sender (creates/updates a mail_account and mirrors to smtp_settings).
pub fn save_mail_settings(settings: MailSettings) -> AppResult<MailSettings> {
    let settings = settings.normalize();
    let label = if !settings.from_email.is_empty() {
        format!("{} · {}", settings.provider, settings.from_email)
    } else {
        format!("{} sender", settings.provider)
    };
    let req = UpsertMailAccount {
        id: None,
        label,
        provider: settings.provider.clone(),
        host: Some(settings.host.clone()),
        port: Some(settings.port),
        username: Some(settings.username.clone()),
        password: Some(settings.password.clone()),
        from_email: settings.from_email.clone(),
        from_name: Some(settings.from_name.clone()),
        use_starttls: Some(settings.use_starttls),
        api_key: Some(settings.api_key.clone()),
        region: Some(settings.region.clone()),
        domain: Some(settings.domain.clone()),
        activate: Some(true),
    };
    // Reuse existing active account id if same provider+from
    let existing = list_mail_accounts()?.into_iter().find(|a| {
        a.active && a.provider == settings.provider && a.from_email == settings.from_email
    });
    let req = if let Some(a) = existing {
        UpsertMailAccount {
            id: Some(a.id),
            ..req
        }
    } else {
        req
    };
    upsert_mail_account(req)?;
    get_mail_settings()
}

pub fn list_mail_accounts() -> AppResult<Vec<MailAccount>> {
    with_db(|conn| {
        migrate_legacy_smtp_to_accounts(conn)?;
        let active = active_account_id(conn).unwrap_or_default();
        let mut stmt = conn.prepare(
            "SELECT id, label, provider, host, port, username, password, from_email, from_name,
                    use_starttls, api_key, region, domain, created_at, updated_at
             FROM mail_accounts ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            let id: String = r.get(0)?;
            Ok(MailAccount {
                id: id.clone(),
                label: r.get(1)?,
                provider: r.get(2)?,
                host: r.get(3)?,
                port: r.get::<_, i64>(4)? as u16,
                username: r.get(5)?,
                password: r.get(6)?,
                from_email: r.get(7)?,
                from_name: r.get(8)?,
                use_starttls: r.get::<_, i64>(9)? != 0,
                api_key: r.get(10)?,
                region: r.get(11)?,
                domain: r.get(12)?,
                active: id == active,
                created_at: r.get(13)?,
                updated_at: r.get(14)?,
            })
        })?;
        Ok(rows
            .filter_map(|r| r.ok())
            .map(redact_mail_account)
            .collect())
    })
}

pub fn upsert_mail_account(req: UpsertMailAccount) -> AppResult<MailAccount> {
    let mut settings = MailSettings {
        provider: req.provider,
        host: req.host.unwrap_or_default(),
        port: req.port.unwrap_or(587),
        username: req.username.unwrap_or_default(),
        password: req.password.unwrap_or_default(),
        from_email: req.from_email,
        from_name: req.from_name.unwrap_or_default(),
        use_starttls: req.use_starttls.unwrap_or(true),
        api_key: req.api_key.unwrap_or_default(),
        region: req.region.unwrap_or_default(),
        domain: req.domain.unwrap_or_default(),
    }
    .normalize();
    if settings.provider == "gmail" && settings.username.is_empty() {
        settings.username = settings.from_email.clone();
    }
    let id = req
        .id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    if let Ok(Some(existing)) = get_settings_for_account(&id) {
        if is_masked(&settings.password) || settings.password.is_empty() {
            settings.password = existing.password;
        }
        if is_masked(&settings.api_key) {
            settings.api_key = existing.api_key;
        }
    }
    if let Ok(Some(existing)) = get_settings_for_account(&id) {
        if is_masked(&settings.password)
            || (settings.password.is_empty() && !existing.password.is_empty())
        {
            // Empty on update keeps previous password (legacy UI behavior).
            if is_masked(&settings.password) || settings.password.is_empty() {
                settings.password = existing.password;
            }
        }
        if is_masked(&settings.api_key) {
            settings.api_key = existing.api_key;
        }
    }
    let label = {
        let l = req.label.trim();
        if l.is_empty() {
            if !settings.from_email.is_empty() {
                format!("{} · {}", settings.provider, settings.from_email)
            } else {
                format!("{} sender", settings.provider)
            }
        } else {
            l.to_string()
        }
    };
    let now = now_iso();
    let activate = req.activate.unwrap_or(true);

    with_db(|conn| {
        migrate_legacy_smtp_to_accounts(conn)?;
        // Preserve secrets if updating and new values are empty
        let existing: Option<(String, String)> = conn
            .query_row(
                "SELECT password, api_key FROM mail_accounts WHERE id = ?1",
                params![&id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        if let Some((pass, key)) = existing {
            if settings.password.is_empty() {
                settings.password = pass;
            }
            if settings.api_key.is_empty() {
                settings.api_key = key;
            }
        }
        let created: String = conn
            .query_row(
                "SELECT created_at FROM mail_accounts WHERE id = ?1",
                params![&id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| now.clone());
        conn.execute(
            "INSERT INTO mail_accounts(id, label, provider, host, port, username, password,
                 from_email, from_name, use_starttls, api_key, region, domain, created_at, updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
             ON CONFLICT(id) DO UPDATE SET
               label=excluded.label,
               provider=excluded.provider,
               host=excluded.host,
               port=excluded.port,
               username=excluded.username,
               password=excluded.password,
               from_email=excluded.from_email,
               from_name=excluded.from_name,
               use_starttls=excluded.use_starttls,
               api_key=excluded.api_key,
               region=excluded.region,
               domain=excluded.domain,
               updated_at=excluded.updated_at",
            params![
                id,
                label,
                settings.provider,
                settings.host,
                settings.port as i64,
                settings.username,
                settings.password,
                settings.from_email,
                settings.from_name,
                if settings.use_starttls { 1 } else { 0 },
                settings.api_key,
                settings.region,
                settings.domain,
                created,
                now,
            ],
        )?;
        if activate {
            conn.execute(
                "INSERT OR REPLACE INTO app_meta(key, value) VALUES('active_mail_account', ?1)",
                params![id],
            )?;
            write_active_smtp(conn, &settings)?;
        }
        Ok(())
    })?;

    list_mail_accounts()?
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| AppError::msg("account missing after upsert"))
}

pub fn activate_mail_account(id: String) -> AppResult<MailAccount> {
    with_db(|conn| {
        let settings = conn.query_row(
            "SELECT id, label, provider, host, port, username, password, from_email, from_name,
                    use_starttls, api_key, region, domain
             FROM mail_accounts WHERE id = ?1",
            params![id],
            settings_from_account_row,
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO app_meta(key, value) VALUES('active_mail_account', ?1)",
            params![id],
        )?;
        write_active_smtp(conn, &settings)?;
        Ok(())
    })?;
    list_mail_accounts()?
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| AppError::msg("account not found"))
}

pub fn delete_mail_account(id: String) -> AppResult<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM mail_accounts WHERE id = ?1", params![id])?;
        let active = active_account_id(conn);
        if active.as_deref() == Some(id.as_str()) {
            conn.execute("DELETE FROM app_meta WHERE key = 'active_mail_account'", [])?;
            // Activate most recent remaining
            let next: Option<String> = conn
                .query_row(
                    "SELECT id FROM mail_accounts ORDER BY updated_at DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .ok();
            if let Some(nid) = next {
                let settings = conn.query_row(
                    "SELECT id, label, provider, host, port, username, password, from_email, from_name,
                            use_starttls, api_key, region, domain
                     FROM mail_accounts WHERE id = ?1",
                    params![nid],
                    settings_from_account_row,
                )?;
                conn.execute(
                    "INSERT OR REPLACE INTO app_meta(key, value) VALUES('active_mail_account', ?1)",
                    params![nid],
                )?;
                write_active_smtp(conn, &settings)?;
            }
        }
        Ok(())
    })
}

/// Import HTML from pasted HTML or a raw .eml message.
pub fn import_email_source(
    raw: String,
    filename: Option<String>,
) -> AppResult<ImportedEmailTemplate> {
    let raw = raw.trim_start_matches('\u{feff}').to_string();
    let fname = filename.unwrap_or_default().to_ascii_lowercase();
    let looks_eml = fname.ends_with(".eml")
        || raw.contains("MIME-Version:")
        || raw.contains("Content-Type:") && (raw.starts_with("From ") || raw.contains("\nFrom:"));

    if looks_eml {
        return import_from_eml(&raw);
    }
    import_from_html(&raw, &fname)
}

fn import_from_html(raw: &str, fname: &str) -> AppResult<ImportedEmailTemplate> {
    let mut html = raw.trim().to_string();
    if html.is_empty() {
        return Err(AppError::msg("Empty HTML"));
    }
    // If user pasted a full document, keep it; ensure {{link}} tip in message
    let subject = Regex::new(r"(?is)<title[^>]*>(.*?)</title>")
        .ok()
        .and_then(|re| {
            re.captures(&html)
                .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Imported email".into());

    // Light cleanup: strip scripts
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    html = re_script.replace_all(&html, "").into_owned();

    let name = if !fname.is_empty() {
        fname
            .trim_end_matches(".html")
            .trim_end_matches(".htm")
            .to_string()
    } else {
        subject.clone()
    };

    Ok(ImportedEmailTemplate {
        name,
        subject,
        html_body: html,
        source: "html".into(),
        message: "Imported HTML. Add {{link}} (and optional {{first_name}}) where the CTA should go, then Save.".into(),
    })
}

fn import_from_eml(raw: &str) -> AppResult<ImportedEmailTemplate> {
    let parsed = mailparse::parse_mail(raw.as_bytes())
        .map_err(|e| AppError::msg(format!("EML parse failed: {e}")))?;

    let subject = parsed
        .headers
        .get_first_value("Subject")
        .unwrap_or_else(|| "Imported email".into());

    let html = extract_html_from_parsed(&parsed).ok_or_else(|| {
        AppError::msg(
            "No text/html part found in this .eml — try Save as HTML or copy the HTML source",
        )
    })?;

    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    let html = re_script.replace_all(&html, "").into_owned();

    Ok(ImportedEmailTemplate {
        name: subject.clone(),
        subject,
        html_body: html,
        source: "eml".into(),
        message: "Imported from .eml. Replace the original CTA URL with {{link}}, then Save."
            .into(),
    })
}

fn extract_html_from_parsed(mail: &mailparse::ParsedMail<'_>) -> Option<String> {
    let ctype = mail.ctype.mimetype.to_ascii_lowercase();
    if ctype == "text/html" {
        return mail.get_body().ok();
    }
    for part in &mail.subparts {
        if let Some(h) = extract_html_from_parsed(part) {
            return Some(h);
        }
    }
    // Fallback: some clients nest differently
    if ctype.starts_with("multipart/") {
        for part in &mail.subparts {
            let mt = part.ctype.mimetype.to_ascii_lowercase();
            if mt == "text/html" {
                return part.get_body().ok();
            }
        }
    }
    None
}

fn build_transport(settings: &MailSettings) -> AppResult<SmtpTransport> {
    if settings.host.trim().is_empty() {
        return Err(AppError::msg("SMTP host is required"));
    }
    if settings.from_email.trim().is_empty() {
        return Err(AppError::msg("From email is required"));
    }
    let mut builder = if settings.use_starttls {
        SmtpTransport::starttls_relay(&settings.host)
            .map_err(|e| AppError::msg(format!("SMTP relay: {e}")))?
            .port(settings.port)
    } else {
        SmtpTransport::relay(&settings.host)
            .map_err(|e| AppError::msg(format!("SMTP relay: {e}")))?
            .port(settings.port)
    };
    if !settings.username.is_empty() {
        builder = builder.credentials(Credentials::new(
            settings.username.clone(),
            settings.password.clone(),
        ));
    }
    Ok(builder.build())
}

fn from_mailbox(settings: &MailSettings) -> AppResult<Mailbox> {
    let addr = settings.from_email.trim();
    if settings.from_name.trim().is_empty() {
        addr.parse()
            .map_err(|e| AppError::msg(format!("invalid from email: {e}")))
    } else {
        format!("{} <{}>", settings.from_name.trim(), addr)
            .parse()
            .map_err(|e| AppError::msg(format!("invalid from mailbox: {e}")))
    }
}

fn send_smtp(settings: &MailSettings, msg: &OutboundMessage) -> AppResult<SendReceipt> {
    let transport = build_transport(settings)?;
    let to_mb: Mailbox = msg
        .to
        .trim()
        .parse()
        .map_err(|e| AppError::msg(format!("invalid recipient: {e}")))?;
    let email = Message::builder()
        .from(from_mailbox(settings)?)
        .to(to_mb)
        .subject(&msg.subject)
        .multipart(MultiPart::alternative().singlepart(SinglePart::html(msg.html_body.clone())))
        .map_err(|e| AppError::msg(format!("build message: {e}")))?;
    transport
        .send(&email)
        .map_err(|e| AppError::msg(format!("SMTP send failed: {e}")))?;
    Ok(SendReceipt {
        to: msg.to.trim().to_string(),
        message: format!("sent via {}", settings.provider),
        message_id: String::new(),
    })
}

impl MailTransport for MailSettings {
    fn send(&self, msg: &OutboundMessage) -> AppResult<SendReceipt> {
        let settings = self.clone().normalize();
        if settings.uses_smtp() {
            send_smtp(&settings, msg)
        } else {
            crate::providers::send_http(&settings, msg)
        }
    }
}

pub fn merge_tags(template: &str, vars: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\{\{\s*([a-zA-Z0-9_]+)\s*\}\}").unwrap();
    re.replace_all(template, |caps: &regex::Captures| {
        let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        vars.get(key).cloned().unwrap_or_else(|| {
            caps.get(0)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default()
        })
    })
    .into_owned()
}

pub fn send_message(
    settings: &MailSettings,
    to: &str,
    subject: &str,
    html_body: &str,
) -> AppResult<SendReceipt> {
    settings.send(&OutboundMessage {
        to: to.to_string(),
        subject: subject.to_string(),
        html_body: html_body.to_string(),
    })
}

pub fn send_test(to: String) -> AppResult<SendReceipt> {
    let settings = get_mail_settings()?;
    send_message(
        &settings,
        &to,
        "phishkit delivery test",
        "<p>This is a phishkit delivery test. Authorized assessment tooling only.</p>",
    )
}

fn row_to_template(r: &rusqlite::Row<'_>) -> rusqlite::Result<EmailTemplate> {
    Ok(EmailTemplate {
        id: r.get(0)?,
        name: r.get(1)?,
        subject: r.get(2)?,
        html_body: r.get(3)?,
        assessment_id: r.get::<_, Option<String>>(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
    })
}

pub fn list_templates(assessment_id: Option<String>) -> AppResult<Vec<EmailTemplate>> {
    with_db(|conn| {
        if let Some(aid) = assessment_id.filter(|s| !s.is_empty()) {
            let mut stmt = conn.prepare(
                "SELECT id, name, subject, html_body, assessment_id, created_at, updated_at
                 FROM email_templates
                 WHERE assessment_id IS NULL OR assessment_id = ?1
                 ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map(params![aid], row_to_template)?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, name, subject, html_body, assessment_id, created_at, updated_at
                 FROM email_templates ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], row_to_template)?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    })
}

pub fn upsert_template(req: UpsertTemplate) -> AppResult<EmailTemplate> {
    let id = req
        .id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let now = now_iso();
    with_db(|conn| {
        let created: String = conn
            .query_row(
                "SELECT created_at FROM email_templates WHERE id = ?1",
                params![&id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| now.clone());
        let assessment_id = req.assessment_id.filter(|s| !s.is_empty());
        conn.execute(
            "INSERT INTO email_templates(id, name, subject, html_body, assessment_id, created_at, updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name,
               subject=excluded.subject,
               html_body=excluded.html_body,
               assessment_id=excluded.assessment_id,
               updated_at=excluded.updated_at",
            params![
                id,
                req.name.trim(),
                req.subject,
                req.html_body,
                assessment_id,
                created,
                now
            ],
        )?;
        Ok(())
    })?;
    list_templates(None)?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| AppError::msg("template missing after upsert"))
}

pub fn delete_template(id: String) -> AppResult<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM email_templates WHERE id = ?1", params![id])?;
        Ok(())
    })
}

pub fn get_template(id: &str) -> AppResult<Option<EmailTemplate>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, subject, html_body, assessment_id, created_at, updated_at
             FROM email_templates WHERE id = ?1",
        )?;
        let t = stmt.query_row(params![id], row_to_template).optional()?;
        Ok(t)
    })
}

fn row_to_recipient_list(r: &rusqlite::Row<'_>) -> rusqlite::Result<RecipientList> {
    Ok(RecipientList {
        id: r.get(0)?,
        name: r.get(1)?,
        assessment_id: r.get(2)?,
        created_at: r.get(3)?,
        updated_at: r.get(4)?,
        recipient_count: r.get(5)?,
    })
}

pub fn list_recipient_lists(assessment_id: Option<String>) -> AppResult<Vec<RecipientList>> {
    with_db(|conn| {
        let sql = "SELECT l.id, l.name, l.assessment_id, l.created_at, l.updated_at,
                          (SELECT COUNT(*) FROM recipients r WHERE r.list_id = l.id AND r.suppressed = 0)
                   FROM recipient_lists l";
        if let Some(aid) = assessment_id.filter(|s| !s.is_empty()) {
            let mut stmt = conn.prepare(&format!(
                "{sql} WHERE l.assessment_id = ?1 OR l.assessment_id = '' ORDER BY l.updated_at DESC"
            ))?;
            let rows = stmt.query_map(params![aid], row_to_recipient_list)?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        } else {
            let mut stmt = conn.prepare(&format!("{sql} ORDER BY l.updated_at DESC"))?;
            let rows = stmt.query_map([], row_to_recipient_list)?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    })
}

pub fn create_recipient_list(
    name: String,
    assessment_id: Option<String>,
) -> AppResult<RecipientList> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    let aid = assessment_id
        .filter(|s| !s.is_empty())
        .or_else(|| crate::db::get_active_assessment_id().ok().flatten())
        .unwrap_or_default();
    with_db(|conn| {
        conn.execute(
            "INSERT INTO recipient_lists(id, name, assessment_id, created_at, updated_at)
             VALUES(?1,?2,?3,?4,?5)",
            params![id, name.trim(), aid, now, now],
        )?;
        Ok(())
    })?;
    list_recipient_lists(None)?
        .into_iter()
        .find(|l| l.id == id)
        .ok_or_else(|| AppError::msg("list missing after create"))
}

pub fn delete_recipient_list(id: String) -> AppResult<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM recipient_lists WHERE id = ?1", params![id])?;
        Ok(())
    })
}

pub fn list_recipients(list_id: String) -> AppResult<Vec<Recipient>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, list_id, email, first_name, last_name, extras, suppressed
             FROM recipients WHERE list_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![list_id], |r| {
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
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    })
}

pub fn import_recipients_csv(list_id: String, csv_text: String) -> AppResult<ImportResult> {
    let text = csv_text.trim();
    if text.is_empty() {
        return Err(AppError::msg("Paste at least one email address"));
    }

    // Plain list: one email per line (or comma/semicolon separated), no header required
    let looks_plain = !text
        .lines()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .contains("email")
        && text.lines().take(5).any(|l| l.contains('@'));

    if looks_plain {
        return import_plain_emails(list_id, text);
    }

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let headers = rdr
        .headers()
        .map(|h| h.iter().map(|s| s.to_ascii_lowercase()).collect::<Vec<_>>())
        .map_err(|e| AppError::msg(format!("CSV header: {e}")))?;

    let email_idx = headers
        .iter()
        .position(|h| h == "email" || h == "e-mail" || h == "mail");
    let Some(email_idx) = email_idx else {
        // Header missing — treat whole blob as plain emails
        return import_plain_emails(list_id, text);
    };
    let first_idx = headers
        .iter()
        .position(|h| h == "first_name" || h == "firstname" || h == "first");
    let last_idx = headers
        .iter()
        .position(|h| h == "last_name" || h == "lastname" || h == "last");

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let now = now_iso();

    with_db(|conn| {
        for result in rdr.records() {
            let record = match result {
                Ok(r) => r,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            let email = record.get(email_idx).unwrap_or("").trim().to_string();
            if email.is_empty() || !email.contains('@') {
                skipped += 1;
                continue;
            }
            let first = first_idx
                .and_then(|i| record.get(i))
                .unwrap_or("")
                .to_string();
            let last = last_idx
                .and_then(|i| record.get(i))
                .unwrap_or("")
                .to_string();
            let mut extras = serde_json::Map::new();
            for (i, h) in headers.iter().enumerate() {
                if i == email_idx || Some(i) == first_idx || Some(i) == last_idx {
                    continue;
                }
                if let Some(v) = record.get(i) {
                    if !v.is_empty() {
                        extras.insert(h.clone(), serde_json::Value::String(v.to_string()));
                    }
                }
            }
            conn.execute(
                "INSERT INTO recipients(list_id, email, first_name, last_name, extras, suppressed)
                 VALUES(?1,?2,?3,?4,?5,0)",
                params![
                    list_id,
                    email,
                    first,
                    last,
                    serde_json::Value::Object(extras).to_string()
                ],
            )?;
            imported += 1;
        }
        conn.execute(
            "UPDATE recipient_lists SET updated_at = ?1 WHERE id = ?2",
            params![now, list_id],
        )?;
        Ok(())
    })?;

    Ok(ImportResult {
        list_id,
        imported,
        skipped,
    })
}

fn import_plain_emails(list_id: String, text: &str) -> AppResult<ImportResult> {
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let now = now_iso();
    let mut seen = std::collections::HashSet::new();
    with_db(|conn| {
        for raw in text.split(|c: char| {
            c == '\n' || c == '\r' || c == ',' || c == ';' || c == '\t' || c == ' '
        }) {
            let email = raw
                .trim()
                .trim_matches(|c| c == '<' || c == '>' || c == '"' || c == '\'')
                .to_ascii_lowercase();
            if email.is_empty() {
                continue;
            }
            if !email.contains('@') || !email.contains('.') {
                skipped += 1;
                continue;
            }
            if !seen.insert(email.clone()) {
                skipped += 1;
                continue;
            }
            conn.execute(
                "INSERT INTO recipients(list_id, email, first_name, last_name, extras, suppressed)
                 VALUES(?1,?2,'','','{}',0)",
                params![list_id, email],
            )?;
            imported += 1;
        }
        conn.execute(
            "UPDATE recipient_lists SET updated_at = ?1 WHERE id = ?2",
            params![now, list_id],
        )?;
        Ok(())
    })?;
    Ok(ImportResult {
        list_id,
        imported,
        skipped,
    })
}

pub fn recipient_vars(r: &Recipient, link: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("email".into(), r.email.clone());
    m.insert("first_name".into(), r.first_name.clone());
    m.insert("last_name".into(), r.last_name.clone());
    m.insert("link".into(), link.to_string());
    if let Some(obj) = r.extras.as_object() {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                m.insert(k.clone(), s.to_string());
            }
        }
    }
    m
}
