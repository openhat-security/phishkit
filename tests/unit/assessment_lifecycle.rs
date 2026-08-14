#[path = "support.rs"]
mod support;

use phishkit_core::assessment::{
    archive_assessment, clone_assessment, create_assessment, delete_assessment,
    get_active_assessment, get_assessment, list_assessments, set_active_assessment,
    unarchive_assessment, CreateAssessment,
};
use phishkit_core::db::normalize_profile_id;
use support::isolate_storage;

fn sample(name: &str) -> CreateAssessment {
    CreateAssessment {
        name: name.into(),
        primary_domain: "example.test".into(),
        authorization_ref: Some("SOW-1".into()),
        authorized_by: Some("Security lead".into()),
        authorized_at: None,
        notes: Some("unit".into()),
        scopes: None,
    }
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
    let a = create_assessment(sample("Lifecycle")).expect("create");
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

#[test]
fn clone_and_set_active_assessment() {
    let (_tmp, _env) = isolate_storage();
    let a = create_assessment(sample("Source")).expect("create");
    set_active_assessment(&a.id).expect("set active");
    let active = get_active_assessment().expect("get active").expect("some");
    assert_eq!(active.id, a.id);

    let copy = clone_assessment(&a.id).expect("clone");
    assert_ne!(copy.id, a.id);
    assert!(copy.name.contains("(copy)"));
    assert_eq!(copy.primary_domain, a.primary_domain);
    assert_eq!(copy.status, "active");
    assert!(get_assessment(&copy.id).unwrap().is_some());
}
