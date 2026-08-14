#[path = "support.rs"]
mod support;

use phishkit_core::assessment::{create_assessment, CreateAssessment};
use phishkit_core::mail::{
    create_recipient_list, import_recipients_csv, list_recipients, recipient_vars, Recipient,
};
use support::isolate_storage;

#[test]
fn import_plain_emails_and_recipient_vars() {
    let (_tmp, _env) = isolate_storage();
    let a = create_assessment(CreateAssessment {
        name: "Mail".into(),
        primary_domain: "example.test".into(),
        authorization_ref: None,
        authorized_by: None,
        authorized_at: None,
        notes: None,
        scopes: None,
    })
    .expect("assessment");
    let list = create_recipient_list("unit-list".into(), Some(a.id)).expect("list");
    let imported = import_recipients_csv(
        list.id.clone(),
        "alice@example.test\nbob@example.test\n".into(),
    )
    .expect("import");
    assert_eq!(imported.imported, 2);
    let rows = list_recipients(list.id).expect("rows");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|r| r.email == "alice@example.test"));

    let vars = recipient_vars(
        &Recipient {
            id: 1,
            list_id: "x".into(),
            email: "alice@example.test".into(),
            first_name: "Alice".into(),
            last_name: "A".into(),
            extras: serde_json::json!({"role": "lead"}),
            suppressed: false,
        },
        "https://lure.test/x",
    );
    assert_eq!(vars.get("email").unwrap(), "alice@example.test");
    assert_eq!(vars.get("first_name").unwrap(), "Alice");
    assert_eq!(vars.get("link").unwrap(), "https://lure.test/x");
    assert_eq!(vars.get("role").unwrap(), "lead");
}
