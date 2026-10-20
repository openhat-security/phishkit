//! Shared phishkit engine used by the desktop app and CLI.

pub mod assessment;
pub mod aup;
pub mod campaign;
pub mod cli;
mod cli_help;
pub mod community;
pub mod db;
pub mod destination;
pub mod engagement;
pub mod error;
pub mod evilginx_ctl;
pub mod firebase;
pub mod hosts;
pub mod kit;
pub mod logs;
pub mod lure_ops;
pub mod mail;
pub mod phishlet;
pub mod providers;
pub mod readiness;
pub mod recon;
pub mod secret_store;
pub mod services;
pub mod sessions;
pub mod setup;
pub mod wiz;

pub use error::{AppError, AppResult};

#[cfg(test)]
pub(crate) mod test_lock {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    /// Process-wide guard for tests that mutate shared environment variables
    /// (`PHISHKIT_DATA`, `PHISHKIT_CONFIG`). Separate per-module locks let tests
    /// in different modules race on these globals, which flaked the secret_store
    /// round-trip on CI. Poison-tolerant so a single failing test does not
    /// cascade a `PoisonError` into every other env-dependent test.
    pub(crate) fn env_guard() -> MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
