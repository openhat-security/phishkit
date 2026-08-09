//! HTTP ESP adapters (BYO API keys). Same OutboundMessage path as SMTP.

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;

use crate::error::{AppError, AppResult};
use crate::mail::{MailSettings, OutboundMessage, SendReceipt};

fn client() -> AppResult<Client> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| AppError::msg(format!("HTTP client: {e}")))
}

fn require_api_key(settings: &MailSettings) -> AppResult<&str> {
    let k = settings.api_key.trim();
    if k.is_empty() {
        return Err(AppError::msg("API key is required for this provider"));
    }
    Ok(k)
}

fn require_from(settings: &MailSettings) -> AppResult<&str> {
    let f = settings.from_email.trim();
    if f.is_empty() {
        return Err(AppError::msg("From email is required"));
    }
    Ok(f)
}

fn from_header(settings: &MailSettings) -> String {
    let email = settings.from_email.trim();
    if settings.from_name.trim().is_empty() {
        email.to_string()
    } else {
        format!("{} <{}>", settings.from_name.trim(), email)
    }
}

/// Best-effort extraction of a provider message id from a JSON response body.
fn json_message_id(text: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
        return String::new();
    };
    for key in ["id", "MessageID", "message_id", "messageId"] {
        if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

pub fn send_http(settings: &MailSettings, msg: &OutboundMessage) -> AppResult<SendReceipt> {
    match settings.provider.as_str() {
        "resend" => send_resend(settings, msg),
        "sendgrid" => send_sendgrid(settings, msg),
        "mailgun" => send_mailgun(settings, msg),
        "postmark" => send_postmark(settings, msg),
        other => Err(AppError::msg(format!("unknown HTTP provider: {other}"))),
    }
}

fn send_resend(settings: &MailSettings, msg: &OutboundMessage) -> AppResult<SendReceipt> {
    let key = require_api_key(settings)?;
    require_from(settings)?;
    let body = json!({
        "from": from_header(settings),
        "to": [msg.to.trim()],
        "subject": msg.subject,
        "html": msg.html_body,
    });
    let res = client()?
        .post("https://api.resend.com/emails")
        .header(AUTHORIZATION, format!("Bearer {key}"))
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .map_err(|e| AppError::msg(format!("Resend request: {e}")))?;
    let status = res.status();
    let text = res.text().unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::msg(format!("Resend {status}: {text}")));
    }
    Ok(SendReceipt {
        to: msg.to.trim().to_string(),
        message: "sent via resend".into(),
        message_id: json_message_id(&text),
    })
}

fn send_sendgrid(settings: &MailSettings, msg: &OutboundMessage) -> AppResult<SendReceipt> {
    let key = require_api_key(settings)?;
    require_from(settings)?;
    let body = json!({
        "personalizations": [{ "to": [{ "email": msg.to.trim() }] }],
        "from": {
            "email": settings.from_email.trim(),
            "name": settings.from_name.trim(),
        },
        "subject": msg.subject,
        "content": [{ "type": "text/html", "value": msg.html_body }],
    });
    let res = client()?
        .post("https://api.sendgrid.com/v3/mail/send")
        .header(AUTHORIZATION, format!("Bearer {key}"))
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .map_err(|e| AppError::msg(format!("SendGrid request: {e}")))?;
    let status = res.status();
    let message_id = res
        .headers()
        .get("x-message-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let text = res.text().unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::msg(format!("SendGrid {status}: {text}")));
    }
    Ok(SendReceipt {
        to: msg.to.trim().to_string(),
        message: "sent via sendgrid".into(),
        message_id,
    })
}

fn send_mailgun(settings: &MailSettings, msg: &OutboundMessage) -> AppResult<SendReceipt> {
    let key = require_api_key(settings)?;
    require_from(settings)?;
    let domain = settings.domain.trim();
    if domain.is_empty() {
        return Err(AppError::msg(
            "Mailgun domain is required (e.g. mg.example.com)",
        ));
    }
    let region = settings.region.trim().to_ascii_lowercase();
    let base = if region == "eu" {
        "https://api.eu.mailgun.net"
    } else {
        "https://api.mailgun.net"
    };
    let url = format!("{base}/v3/{domain}/messages");
    let res = client()?
        .post(&url)
        .basic_auth("api", Some(key))
        .form(&[
            ("from", from_header(settings)),
            ("to", msg.to.trim().to_string()),
            ("subject", msg.subject.clone()),
            ("html", msg.html_body.clone()),
        ])
        .send()
        .map_err(|e| AppError::msg(format!("Mailgun request: {e}")))?;
    let status = res.status();
    let text = res.text().unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::msg(format!("Mailgun {status}: {text}")));
    }
    Ok(SendReceipt {
        to: msg.to.trim().to_string(),
        message: "sent via mailgun".into(),
        message_id: json_message_id(&text),
    })
}

fn send_postmark(settings: &MailSettings, msg: &OutboundMessage) -> AppResult<SendReceipt> {
    let key = require_api_key(settings)?;
    require_from(settings)?;
    let body = json!({
        "From": from_header(settings),
        "To": msg.to.trim(),
        "Subject": msg.subject,
        "HtmlBody": msg.html_body,
        "MessageStream": "outbound",
    });
    let res = client()?
        .post("https://api.postmarkapp.com/email")
        .header("X-Postmark-Server-Token", key)
        .header(CONTENT_TYPE, "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .map_err(|e| AppError::msg(format!("Postmark request: {e}")))?;
    let status = res.status();
    let text = res.text().unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::msg(format!("Postmark {status}: {text}")));
    }
    Ok(SendReceipt {
        to: msg.to.trim().to_string(),
        message: "sent via postmark".into(),
        message_id: json_message_id(&text),
    })
}
