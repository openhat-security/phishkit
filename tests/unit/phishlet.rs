use std::fs;
use std::io::Write;

use phishkit_core::engagement::phishlet_is_valid;

#[test]
fn phishlet_is_valid_rejects_placeholders_and_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("nope.yaml");
    assert!(!phishlet_is_valid(&missing));

    let ready = tmp.path().join("ready.yaml");
    fs::write(
        &ready,
        "name: 'demo'\nproxy_hosts:\n  - {phish_sub: 'app'}\n",
    )
    .unwrap();
    assert!(phishlet_is_valid(&ready));

    let stub = tmp.path().join("stub.yaml");
    let mut f = fs::File::create(&stub).unwrap();
    writeln!(f, "min_ver: '3.0.0'").unwrap();
    writeln!(f, "proxy_hosts:").unwrap();
    writeln!(
        f,
        "  - {{phish_sub: 'app', orig_sub: 'app', domain: 'TARGET_DOMAIN'}}"
    )
    .unwrap();
    assert!(!phishlet_is_valid(&stub));
}
