use std::sync::Mutex;

use once_cell::sync::Lazy;
use phishkit_core::assessment::{
    archive_assessment, create_assessment, delete_assessment, get_assessment, list_assessments,
    unarchive_assessment, CreateAssessment,
};
use phishkit_core::db::normalize_profile_id;
use phishkit_core::setup::{complete_setup, data_dir, SetupConfig, StorageMode};

static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn isolate_storage() -> (tempfile::TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = ENV_LOCK.lock().unwrap();
    let tmp = tempfile::tempdir().expect("tempdir");
    std::env::set_var("PHISHKIT_CONFIG", tmp.path().join("cfg"));
    std::env::set_var("PHISHKIT_DATA", tmp.path().join("data"));
    let mut cfg = SetupConfig::default();
    cfg.storage_mode = StorageMode::Persistent;
    cfg.custom_data_dir = Some(tmp.path().join("data").display().to_string());
    complete_setup(cfg).expect("setup");
    assert!(data_dir().unwrap().exists());
    (tmp, guard)
}

#[test]
fn normalize_profile_id_slugifies() {
    assert_eq!(normalize_profile_id(Some("ok-id"), None).unwrap(), "ok-id");
    assert_eq!(
        normalize_profile_id(None, Some("Hello World")).unwrap(),
        "hello-world"
    );
}

#[test]
fn archive_unarchive_and_delete_assessment() {
    let (_tmp, _env) = isolate_storage();
    let a = create_assessment(CreateAssessment {
        name: "Lifecycle".into(),
        primary_domain: "example.test".into(),
        authorization_ref: None,
        authorized_by: None,
        authorized_at: None,
        notes: None,
        scopes: None,
    })
    .expect("create");
    assert_eq!(a.status, "active");

    let archived = archive_assessment(&a.id).expect("archive");
    assert_eq!(archived.status, "archived");
    let active_only = list_assessments(false).expect("list");
    assert!(!active_only.iter().any(|x| x.id == a.id));
    let all = list_assessments(true).expect("list all");
    assert!(all.iter().any(|x| x.id == a.id));

    let restored = unarchive_assessment(&a.id).expect("unarchive");
    assert_eq!(restored.status, "active");

    let deleted = delete_assessment(&a.id).expect("delete");
    assert_eq!(deleted.id, a.id);
    assert!(get_assessment(&a.id).unwrap().is_none());
}
