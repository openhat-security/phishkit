#[path = "support.rs"]
mod support;

use phishkit_core::{aup, setup};
use support::isolate_storage;

/// End-to-end: a real DB operation through the normal engine must leave the
/// on-disk store and its directory unreadable by other local users (H2).
#[cfg(unix)]
#[test]
fn live_db_open_yields_0600_file_and_0700_dir() {
    use std::os::unix::fs::PermissionsExt;

    let (_tmp, _env) = isolate_storage();

    // Any DB op opens/creates the store via the engine.
    aup::get_aup_status().expect("db op");

    let db = setup::db_file_path().expect("db path");
    assert!(db.exists(), "db should exist after a db op");
    let mode = std::fs::metadata(&db).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "db must be 0600, got {mode:o}");

    let dir = db.parent().expect("data dir");
    let dmode = std::fs::metadata(dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(dmode, 0o700, "data dir must be 0700, got {dmode:o}");
}

/// A store restored with loose permissions (e.g. from a backup or sync) is
/// re-tightened on the next open rather than left world-readable (H2).
#[cfg(unix)]
#[test]
fn loose_permissions_are_repaired_on_next_open() {
    use std::os::unix::fs::PermissionsExt;

    let (_tmp, _env) = isolate_storage();
    aup::get_aup_status().expect("create db");

    let db = setup::db_file_path().expect("db path");
    let dir = db.parent().expect("data dir").to_path_buf();
    std::fs::set_permissions(&db, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Next engine DB op must repair both.
    aup::get_aup_status().expect("reopen db");

    assert_eq!(
        std::fs::metadata(&db).unwrap().permissions().mode() & 0o777,
        0o600,
        "db must be re-tightened to 0600"
    );
    assert_eq!(
        std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
        0o700,
        "data dir must be re-tightened to 0700"
    );
}
