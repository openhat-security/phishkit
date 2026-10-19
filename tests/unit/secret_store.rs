#[path = "support.rs"]
mod support;

use phishkit_core::mail::{self, MailSettings};
use phishkit_core::secret_store;
use support::isolate_storage;

/// End-to-end: saving an SMTP password must not leave the plaintext in the DB
/// file (audit H1 repro used `strings` on the store).
#[test]
fn saved_smtp_password_is_not_plaintext_in_db_file() {
    let (_tmp, _env) = isolate_storage();

    const PLAIN: &str = "audit-repro-smtp-password-XYZ";

    let settings = MailSettings {
        provider: "smtp".into(),
        host: "smtp.example.test".into(),
        port: 587,
        username: "ops@example.test".into(),
        password: PLAIN.into(),
        from_email: "ops@example.test".into(),
        from_name: "Ops".into(),
        use_starttls: true,
        api_key: "sg-live-api-key-ABCDEF".into(),
        region: String::new(),
        domain: String::new(),
        smtp_security: String::new(),
        accept_invalid_certs: false,
    };
    mail::save_mail_settings(settings).expect("save");

    let db = phishkit_core::setup::db_file_path().expect("db path");
    let bytes = std::fs::read(&db).expect("read db");
    let hay = String::from_utf8_lossy(&bytes);
    assert!(
        !hay.contains(PLAIN),
        "DB file must not contain the plaintext SMTP password"
    );
    assert!(
        !hay.contains("sg-live-api-key-ABCDEF"),
        "DB file must not contain the plaintext API key"
    );
    // Sealed blobs should be present.
    assert!(
        hay.contains("enc:v1:"),
        "DB should contain sealed enc:v1: blobs"
    );

    // Round-trip: engine still returns the real secret for sending.
    let loaded = mail::get_mail_settings().expect("load");
    assert_eq!(loaded.password, PLAIN);
    assert_eq!(loaded.api_key, "sg-live-api-key-ABCDEF");
}

/// Legacy plaintext rows are upgraded on the next mail read and disappear from
/// the DB file afterwards.
#[test]
fn legacy_plaintext_is_resealed_on_next_read() {
    let (_tmp, _env) = isolate_storage();

    // Force a DB open, then poke a plaintext password straight into sqlite.
    let _ = mail::get_mail_settings().expect("init db");
    let db = phishkit_core::setup::db_file_path().expect("db path");
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute(
            "UPDATE smtp_settings SET password = ?1, api_key = ?2, host = 'smtp.example.test',
             from_email = 'ops@example.test', username = 'ops' WHERE id = 1",
            rusqlite::params!["legacy-plain-pass", "legacy-api-key"],
        )
        .unwrap();
        // Also ensure there is an active account with plaintext, or clear accounts
        // so get_mail_settings falls back to smtp_settings.
        conn.execute("DELETE FROM mail_accounts", []).unwrap();
        conn.execute("DELETE FROM app_meta WHERE key = 'active_mail_account'", [])
            .unwrap();
    }

    let before = String::from_utf8_lossy(&std::fs::read(&db).unwrap()).into_owned();
    assert!(before.contains("legacy-plain-pass"));

    let loaded = mail::get_mail_settings().expect("load triggers reseal");
    assert_eq!(loaded.password, "legacy-plain-pass");
    assert_eq!(loaded.api_key, "legacy-api-key");

    let after = String::from_utf8_lossy(&std::fs::read(&db).unwrap()).into_owned();
    assert!(
        !after.contains("legacy-plain-pass"),
        "plaintext must be gone after reseal migration"
    );
    assert!(
        !after.contains("legacy-api-key"),
        "api key plaintext must be gone after reseal"
    );
    assert!(after.contains("enc:v1:"));
    assert!(secret_store::is_sealed(
        // sanity on the module helper
        &secret_store::seal("x").unwrap()
    ));
}
