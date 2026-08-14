//! Interactive CLI wizards (`phishkit wiz …`). TTY-only guided flows.

use std::io::{self, BufRead, IsTerminal, Write};

use serde_json::json;

use crate::assessment::{self, CreateAssessment};
use crate::campaign::{self, CreateCampaign};
use crate::cli_help;
use crate::db;
use crate::destination;
use crate::error::{AppError, AppResult};
use crate::hosts;
use crate::lure_ops;
use crate::mail::{self, UpsertMailAccount};
use crate::sessions;

fn color() -> bool {
    cli_help::want_color()
}

fn paint(code: &str, s: &str) -> String {
    if color() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn err_tty() -> AppResult<()> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(AppError::msg(
            "phishkit wiz … requires an interactive terminal (TTY).\n\
             Tip: run from a real shell, or use non-interactive commands from `phishkit --help`.",
        ));
    }
    Ok(())
}

fn out(msg: &str) {
    let _ = writeln!(io::stderr(), "{msg}");
}

fn step(title: &str) {
    out("");
    out(&paint("1;35", &format!("── {title} ──")));
}

fn prompt_line(label: &str) -> AppResult<String> {
    eprint!("{} ", paint("1;33", label));
    let _ = io::stderr().flush();
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn prompt_default(label: &str, default: &str) -> AppResult<String> {
    let v = prompt_line(&format!("{label} [{default}]:"))?;
    if v.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(v)
    }
}

fn confirm(label: &str, default_yes: bool) -> AppResult<bool> {
    let hint = if default_yes { "Y/n" } else { "y/N" };
    let v = prompt_line(&format!("{label} ({hint})"))?;
    if v.is_empty() {
        return Ok(default_yes);
    }
    Ok(matches!(
        v.to_ascii_lowercase().as_str(),
        "y" | "yes" | "1" | "true"
    ))
}

fn require_authorized() -> AppResult<()> {
    step("Authorized use");
    out(&paint(
        "2",
        "phishkit is for ethical, authorized security assessments only.",
    ));
    out(&paint(
        "2",
        "You must have written authorization for the domains and people you test.",
    ));
    out(&paint("2", "Docs: docs/guide/authorized-use.md"));
    if !confirm(
        "I have written authorization for this engagement and will not abuse this tool.",
        false,
    )? {
        return Err(AppError::msg(
            "Aborted — authorized-use confirmation required.",
        ));
    }
    Ok(())
}

pub fn menu() -> AppResult<()> {
    err_tty()?;
    out(&paint("1;36", "phishkit wiz"));
    out(&paint(
        "2",
        "Interactive wizards (authorized assessments only)",
    ));
    out("");
    out(&format!(
        "  {}  {}",
        paint("1;32", "quickstart"),
        paint(
            "2",
            "New assessment end-to-end (target → SMTP → next steps)"
        )
    ));
    out(&format!(
        "  {}         {}",
        paint("1;32", "send"),
        paint("2", "Pick template / list and send (SMTP already set up)")
    ));
    out(&format!(
        "  {}     {}",
        paint("1;32", "sessions"),
        paint("2", "Sync and browse captures")
    ));
    out("");
    out(&paint("2", "Run:  phishkit wiz <quickstart|send|sessions>"));
    Ok(())
}

pub fn run(sub: &str, _args: &[String]) -> AppResult<serde_json::Value> {
    match sub {
        "" | "help" | "--help" | "-h" | "menu" => {
            menu()?;
            Ok(json!({ "ok": true, "menu": true }))
        }
        "quickstart" => quickstart(),
        "send" => send_flow(),
        "sessions" => sessions_flow(),
        other => Err(AppError::msg(format!(
            "unknown wiz subcommand: {other}\nTry: phishkit wiz quickstart|send|sessions"
        ))),
    }
}

fn quickstart() -> AppResult<serde_json::Value> {
    err_tty()?;
    require_authorized()?;

    step("Target");
    let domain = prompt_line("Target domain or URL:")?;
    if domain.is_empty() {
        return Err(AppError::msg("domain required"));
    }
    let default_name = domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("assessment")
        .to_string();
    let name = prompt_default("Assessment name", &default_name)?;
    let auth_ref = prompt_line("Authorization ref (optional ticket/email id):")?;

    step("Assessment");
    let assessment = assessment::create_assessment(CreateAssessment {
        name: name.clone(),
        primary_domain: default_name.clone(),
        authorization_ref: if auth_ref.is_empty() {
            None
        } else {
            Some(auth_ref)
        },
        authorized_by: None,
        authorized_at: None,
        notes: Some("Created via phishkit wiz quickstart".into()),
        scopes: None,
    })?;
    assessment::set_active_assessment(&assessment.id)?;
    out(&format!(
        "{} {}",
        paint("1;32", "created assessment"),
        assessment.id
    ));

    step("Destination (detect / scaffold / profile)");
    let dest = destination::ensure_destination(
        domain.clone(),
        Some(name.clone()),
        false,
        Some(assessment.id.clone()),
    )?;
    let profile_id = dest.profile.id.clone();
    out(&dest.message.to_string());
    out(&format!(
        "profile={} phishlet={} dryrun={}",
        dest.profile.id, dest.profile.phishlet, dest.profile.dryrun_domain
    ));

    if !dest.profile.dryrun_domain.is_empty()
        && confirm("Add /etc/hosts entries for the dry-run domain now?", false)?
    {
        step("Hosts");
        match hosts::hosts_fix(
            dest.profile.dryrun_domain.clone(),
            Some(dest.profile.phishlet.clone()),
        ) {
            Ok(v) => out(&format!("{v}")),
            Err(e) => out(&paint("1;31", &format!("hosts-fix failed: {e}"))),
        }
    }

    step("Mail / SMTP");
    let accounts = mail::list_mail_accounts().unwrap_or_default();
    if accounts.is_empty() || confirm("Configure / update a mail account now?", true)? {
        let label = prompt_default("Account label", "default")?;
        let provider = prompt_default(
            "Provider (smtp|resend|sendgrid|mailgun|postmark|ses)",
            "smtp",
        )?;
        let from_email = prompt_line("From email:")?;
        let mut req = UpsertMailAccount {
            id: None,
            label,
            provider: provider.clone(),
            host: None,
            port: None,
            username: None,
            password: None,
            from_email: from_email.clone(),
            from_name: None,
            use_starttls: Some(true),
            api_key: None,
            region: None,
            domain: None,
            activate: Some(true),
        };
        if provider.eq_ignore_ascii_case("smtp") {
            req.host = Some(prompt_line("SMTP host:")?);
            let port_s = prompt_default("SMTP port", "587")?;
            req.port = port_s.parse().ok();
            req.username = Some(prompt_line("SMTP username:")?);
            req.password = Some(prompt_line("SMTP password:")?);
        } else {
            req.api_key = Some(prompt_line("API key:")?);
        }
        let acct = mail::upsert_mail_account(req)?;
        out(&format!(
            "{} mail account {}",
            paint("1;32", "active"),
            acct.id
        ));
        if confirm("Send a test email now?", true)? {
            let to = prompt_line("Test recipient email:")?;
            match mail::send_test(to) {
                Ok(r) => out(&format!("test send: {:?}", r)),
                Err(e) => out(&paint("1;31", &format!("send-test failed: {e}"))),
            }
        }
    } else {
        out(&paint("2", "Keeping existing mail accounts."));
    }

    step("Lure (optional)");
    let mut lure_url = String::new();
    if confirm(
        "Start evilginx lure now? (needs built binary + privileges)",
        false,
    )? {
        match crate::evilginx_ctl::start_with_lure(
            dest.profile.id.clone(),
            dest.profile.dryrun_domain.clone(),
            dest.profile.phishlet.clone(),
            None,
        ) {
            Ok(r) => {
                lure_url = r.lure_url.clone();
                out(&r.message);
            }
            Err(e) => out(&paint("1;31", &format!("start-lure failed: {e}"))),
        }
    }

    step("Next steps");
    out("Templates — desktop Templates page, or:");
    out(&format!(
        "  phishkit upsert-template -j '{{\"name\":\"Welcome\",\"subject\":\"…\",\"htmlBody\":\"…\",\"assessmentId\":\"{}\"}}'",
        assessment.id
    ));
    out("Recipients — desktop Recipients, or create-list + import-recipients");
    out("Campaigns — desktop Campaigns / Guided, or:");
    out("  phishkit wiz send");
    out("Watch live — Results in desktop, or campaign-funnel / campaign-report");
    out("Sessions — phishkit wiz sessions   (or sync-captures)");
    if !lure_url.is_empty() {
        out(&format!("Lure URL: {lure_url}"));
    }

    Ok(json!({
        "ok": true,
        "wizard": "quickstart",
        "assessmentId": assessment.id,
        "profileId": profile_id,
        "lureUrl": lure_url,
    }))
}

fn send_flow() -> AppResult<serde_json::Value> {
    err_tty()?;
    require_authorized()?;

    let accounts = mail::list_mail_accounts()?;
    if accounts.is_empty() {
        return Err(AppError::msg(
            "No mail accounts configured. Run: phishkit wiz quickstart",
        ));
    }

    step("Assessment");
    let assessments = assessment::list_assessments(false)?;
    if assessments.is_empty() {
        return Err(AppError::msg(
            "No assessments. Run: phishkit wiz quickstart",
        ));
    }
    for (i, a) in assessments.iter().enumerate() {
        out(&format!("  [{}] {}  ({})", i + 1, a.name, a.primary_domain));
    }
    let idx: usize = prompt_default("Choose assessment #", "1")?
        .parse::<usize>()
        .unwrap_or(1)
        .saturating_sub(1);
    let assessment = assessments
        .get(idx)
        .ok_or_else(|| AppError::msg("invalid assessment selection"))?;
    let _ = assessment::set_active_assessment(&assessment.id);

    step("Template");
    let templates = mail::list_templates(Some(assessment.id.clone()))?;
    let template_id = if templates.is_empty() {
        out("No templates — create a minimal one.");
        let subject = prompt_default("Subject", "Action required")?;
        let html = prompt_default(
            "HTML body (use {{.URL}} for the link)",
            "<p>Please continue: <a href=\"{{.URL}}\">link</a></p>",
        )?;
        let t = mail::upsert_template(mail::UpsertTemplate {
            id: None,
            name: prompt_default("Template name", "wiz-template")?,
            subject,
            html_body: html,
            assessment_id: Some(assessment.id.clone()),
        })?;
        if confirm("Open HTML preview in your browser?", true)? {
            let path = std::env::temp_dir().join("phishkit-template-preview.html");
            std::fs::write(&path, &t.html_body)?;
            let _ = std::process::Command::new("open").arg(&path).spawn();
            out(&format!("preview → {}", path.display()));
        }
        t.id
    } else {
        for (i, t) in templates.iter().enumerate() {
            out(&format!("  [{}] {} — {}", i + 1, t.name, t.subject));
        }
        let idx: usize = prompt_default("Choose template #", "1")?
            .parse::<usize>()
            .unwrap_or(1)
            .saturating_sub(1);
        let t = templates
            .get(idx)
            .ok_or_else(|| AppError::msg("invalid template"))?;
        if confirm("Open HTML preview in your browser?", false)? {
            let path = std::env::temp_dir().join("phishkit-template-preview.html");
            std::fs::write(&path, &t.html_body)?;
            let _ = std::process::Command::new("open").arg(&path).spawn();
        }
        t.id.clone()
    };

    step("Recipients");
    let lists = mail::list_recipient_lists(Some(assessment.id.clone()))?;
    let list_id = if lists.is_empty() {
        let list = mail::create_recipient_list(
            prompt_default("List name", "wiz-list")?,
            Some(assessment.id.clone()),
        )?;
        let email = prompt_line("Recipient email (single test address):")?;
        if !email.is_empty() {
            let csv = format!("email\n{email}\n");
            let _ = mail::import_recipients_csv(list.id.clone(), csv)?;
        }
        list.id
    } else {
        for (i, l) in lists.iter().enumerate() {
            out(&format!("  [{}] {}", i + 1, l.name));
        }
        let idx: usize = prompt_default("Choose list #", "1")?
            .parse::<usize>()
            .unwrap_or(1)
            .saturating_sub(1);
        lists
            .get(idx)
            .ok_or_else(|| AppError::msg("invalid list"))?
            .id
            .clone()
    };

    step("Link URL");
    let profiles = assessment::list_targets(&assessment.id)?;
    let mut default_link = String::new();
    if let Some(p) = profiles.first() {
        if let Ok(Some(lure)) = lure_ops::get_default_lure(&p.id) {
            if !lure.lure_url.is_empty() {
                default_link = lure.lure_url;
            }
        }
        if default_link.is_empty() && !p.lure_url.is_empty() {
            default_link = p.lure_url.clone();
        }
    }
    let link = if default_link.is_empty() {
        prompt_line("Campaign link URL (lure):")?
    } else {
        prompt_default("Campaign link URL", &default_link)?
    };

    step("Campaign");
    let campaign = campaign::create_campaign(CreateCampaign {
        name: prompt_default("Campaign name", "wiz-campaign")?,
        template_id,
        list_id,
        link_url: link,
        profile_id: profiles.first().map(|p| p.id.clone()),
        assessment_id: Some(assessment.id.clone()),
        lure_id: None,
        sender_account_id: None,
        rate_per_minute: None,
        mode: None,
        scheduled_at: None,
        send_window_start: None,
        send_window_end: None,
    })?;
    out(&format!("{} {}", paint("1;32", "created"), campaign.id));

    if confirm("Send a one-off campaign test email?", true)? {
        let to = prompt_line("Test to:")?;
        match campaign::send_campaign_test(campaign.id.clone(), to) {
            Ok(_) => out(&paint("1;32", "test queued/sent")),
            Err(e) => out(&paint("1;31", &format!("test failed: {e}"))),
        }
    }
    if confirm("Start the campaign now?", false)? {
        match campaign::start_campaign(campaign.id.clone()) {
            Ok(c) => out(&format!("status={}", c.status)),
            Err(e) => out(&paint("1;31", &format!("start failed: {e}"))),
        }
    }

    out("Watch: desktop Results, or `phishkit campaign-funnel -i <id>`");
    Ok(json!({
        "ok": true,
        "wizard": "send",
        "campaignId": campaign.id,
        "assessmentId": assessment.id,
    }))
}

fn sessions_flow() -> AppResult<serde_json::Value> {
    err_tty()?;
    require_authorized()?;

    step("Profile");
    let profiles = db::list_profiles()?;
    if profiles.is_empty() {
        return Err(AppError::msg("No profiles. Run: phishkit wiz quickstart"));
    }
    for (i, p) in profiles.iter().enumerate() {
        out(&format!(
            "  [{}] {}  target={} phishlet={}",
            i + 1,
            p.name,
            p.target_domain,
            p.phishlet
        ));
    }
    let idx: usize = prompt_default("Choose profile #", "1")?
        .parse::<usize>()
        .unwrap_or(1)
        .saturating_sub(1);
    let profile = profiles
        .get(idx)
        .ok_or_else(|| AppError::msg("invalid profile"))?;

    step("Sync");
    let rows = match sessions::sync_captures(profile.id.clone()) {
        Ok(r) => r,
        Err(e) => {
            out(&paint("1;33", &format!("sync note: {e}")));
            sessions::list_captures(profile.id.clone())?
        }
    };
    if rows.is_empty() {
        out("No captures yet. Start a lure and complete a login on the dry-run domain.");
        return Ok(json!({ "ok": true, "wizard": "sessions", "count": 0 }));
    }
    for (i, c) in rows.iter().enumerate() {
        let user = c
            .data
            .pointer("/username")
            .or_else(|| c.data.pointer("/creds/username"))
            .and_then(|v| v.as_str())
            .unwrap_or("(unknown)");
        out(&format!(
            "  [{}] session={}  user={}",
            i + 1,
            c.evilginx_session_id,
            user
        ));
    }
    let idx: usize = prompt_default("Choose session # (or 0 to exit)", "1")?
        .parse::<usize>()
        .unwrap_or(1);
    if idx == 0 {
        return Ok(json!({ "ok": true, "wizard": "sessions", "count": rows.len() }));
    }
    let cap = rows
        .get(idx - 1)
        .ok_or_else(|| AppError::msg("invalid session"))?;

    if confirm("Export cookies (JSON)?", true)? {
        let cookies = sessions::export_capture_cookies(
            profile.id.clone(),
            cap.evilginx_session_id,
            "json".into(),
        )?;
        out(&cookies);
    }
    if confirm("Attribute captures to campaign sends?", false)? {
        match campaign::attribute_captures(profile.id.clone()) {
            Ok(v) => out(&format!("{v:?}")),
            Err(e) => out(&paint("1;31", &format!("{e}"))),
        }
    }

    Ok(json!({
        "ok": true,
        "wizard": "sessions",
        "profileId": profile.id,
        "sessionId": cap.evilginx_session_id,
        "count": rows.len(),
    }))
}
