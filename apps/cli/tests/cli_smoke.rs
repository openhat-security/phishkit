use phishkit_core::cli::{self, help_plain};
use serde_json::Value;
use std::process::Command;

#[test]
fn help_text_covers_setup_and_aup() {
    let help = help_plain();
    assert!(help.contains("setup-get"));
    assert!(help.contains("paths"));
    assert!(help.contains("aup-status"));
    assert!(help.contains("sync-community"));
    assert!(help.contains("wiz quickstart"));
    assert!(help.contains("-i, --id"));
    assert!(help.contains("-p, --profile-id"));
}

#[test]
fn help_command_returns_json() {
    let v = cli::run("help", &[]).expect("help");
    assert!(v
        .get("help")
        .and_then(|h| h.as_str())
        .unwrap()
        .contains("phishkit"));
}

#[test]
fn missing_required_flag_is_err() {
    let err = cli::run("get-assessment", &[]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("missing"), "{msg}");
    assert!(msg.contains("--id"), "{msg}");
}

#[test]
fn short_id_flag_accepted() {
    // Missing resource is ok — we only assert flag parsing reaches the engine.
    let err = cli::run("get-assessment", &["-i".into(), "no-such-id".into()]);
    // Either Ok(null) or Err(not found) depending on DB; must not be "missing -i/--id"
    if let Err(e) = err {
        assert!(!e.to_string().contains("missing"), "{e}");
    }
}

#[test]
fn unknown_command_mentions_help() {
    let err = cli::run("not-a-real-command", &[]).unwrap_err();
    assert!(err.to_string().contains("unknown command"));
    assert!(err.to_string().contains("--help"));
}

#[test]
fn binary_prints_json_error_shape() {
    let bin = env!("CARGO_BIN_EXE_phishkit");
    let out = Command::new(bin)
        .args(["get-assessment"])
        .output()
        .expect("run phishkit");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|_| {
        panic!("expected JSON error on stderr, got: {stderr}");
    });
    assert!(v
        .get("error")
        .and_then(|e| e.as_str())
        .unwrap()
        .contains("missing"));
}

#[test]
fn binary_help_exits_zero() {
    let bin = env!("CARGO_BIN_EXE_phishkit");
    let out = Command::new(bin)
        .env("NO_COLOR", "1")
        .args(["--help"])
        .output()
        .expect("run phishkit --help");
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("USAGE"));
    assert!(stderr.contains("setup-get"));
    assert!(!stderr.contains('\u{1b}'), "NO_COLOR should disable ANSI");
}
