use std::sync::Mutex;

use once_cell::sync::Lazy;
use phishkit_core::setup::{complete_setup, data_dir, SetupConfig, StorageMode};

pub static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Isolate PHISHKIT_CONFIG / PHISHKIT_DATA under a temp dir. Hold the guard
/// for the life of the test so parallel tests do not clobber process env.
pub fn isolate_storage() -> (tempfile::TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = ENV_LOCK.lock().unwrap();
    let tmp = tempfile::tempdir().expect("tempdir");
    std::env::set_var("PHISHKIT_CONFIG", tmp.path().join("cfg"));
    std::env::set_var("PHISHKIT_DATA", tmp.path().join("data"));
    let cfg = SetupConfig {
        storage_mode: StorageMode::Persistent,
        custom_data_dir: Some(tmp.path().join("data").display().to_string()),
        ..SetupConfig::default()
    };
    complete_setup(cfg).expect("setup");
    assert!(data_dir().unwrap().exists());
    (tmp, guard)
}
