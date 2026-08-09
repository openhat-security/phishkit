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
pub mod services;
pub mod sessions;
pub mod setup;
pub mod wiz;

pub use error::{AppError, AppResult};
