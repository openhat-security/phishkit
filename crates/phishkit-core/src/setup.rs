//! First-run setup + durable preferences (OS config/data dirs).

use std::fs;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "phishkit";
const APPLICATION: &str = "phishkit";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StorageMode {
    Persistent,
    Ephemeral,
}

impl Default for StorageMode {
    fn default() -> Self {
        Self::Persistent
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Persona {
    #[serde(rename = "businessOwner")]
    BusinessOwner,
    Developer,
    #[serde(rename = "penetrationTester")]
    PenetrationTester,
    #[serde(rename = "cybersecStudent")]
    CybersecStudent,
}

impl Default for Persona {
    fn default() -> Self {
        Self::CybersecStudent
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupConfig {
    pub setup_complete: bool,
    pub persona: Persona,
    pub tutorial_completed: bool,
    pub storage_mode: StorageMode,
    /// Optional override for the persistent data root.
    pub custom_data_dir: Option<String>,
    /// Optional kit root override (sources).
    pub kit_root_override: Option<String>,
    pub ephemeral_id: Option<String>,
}

impl Default for SetupConfig {
    fn default() -> Self {
        Self {
            setup_complete: false,
            persona: Persona::default(),
            tutorial_completed: false,
            storage_mode: StorageMode::default(),
            custom_data_dir: None,
            kit_root_override: None,
            ephemeral_id: None,
        }
    }
}

fn project_dirs() -> AppResult<ProjectDirs> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .ok_or_else(|| AppError::msg("could not resolve OS project directories"))
}

pub fn config_dir() -> AppResult<PathBuf> {
    if let Ok(p) = std::env::var("PHISHKIT_CONFIG") {
        let path = PathBuf::from(p);
        fs::create_dir_all(&path)?;
        return Ok(path);
    }
    let dirs = project_dirs()?;
    let path = dirs.config_dir().to_path_buf();
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn setup_path() -> AppResult<PathBuf> {
    Ok(config_dir()?.join("setup.json"))
}

pub fn load_setup() -> AppResult<SetupConfig> {
    let path = setup_path()?;
    if !path.is_file() {
        return Ok(SetupConfig::default());
    }
    let raw = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save_setup(cfg: &SetupConfig) -> AppResult<()> {
    let path = setup_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(cfg)?;
    fs::write(&path, raw)?;
    Ok(())
}

pub fn complete_setup(mut cfg: SetupConfig) -> AppResult<SetupConfig> {
    cfg.setup_complete = true;
    if cfg.storage_mode == StorageMode::Ephemeral && cfg.ephemeral_id.is_none() {
        cfg.ephemeral_id = Some(uuid::Uuid::new_v4().to_string());
    }
    if cfg.storage_mode == StorageMode::Persistent {
        cfg.ephemeral_id = None;
        // Wipe leftover ephemeral sandboxes from prior sessions.
        wipe_ephemeral_sandboxes()?;
    }
    save_setup(&cfg)?;
    let _ = data_dir()?; // ensure created
    Ok(cfg)
}

pub fn set_tutorial_completed(done: bool) -> AppResult<SetupConfig> {
    let mut cfg = load_setup()?;
    cfg.tutorial_completed = done;
    save_setup(&cfg)?;
    Ok(cfg)
}

fn ephemeral_base() -> AppResult<PathBuf> {
    let base = std::env::temp_dir().join("phishkit-ephemeral");
    fs::create_dir_all(&base)?;
    Ok(base)
}

fn wipe_ephemeral_sandboxes() -> AppResult<()> {
    let base = ephemeral_base()?;
    if base.is_dir() {
        let _ = fs::remove_dir_all(&base);
    }
    Ok(())
}

/// Resolve the mutable data root (DB, evilginx runtime). Honors PHISHKIT_DATA.
pub fn data_dir() -> AppResult<PathBuf> {
    if let Ok(p) = std::env::var("PHISHKIT_DATA") {
        let path = PathBuf::from(p);
        fs::create_dir_all(&path)?;
        return Ok(path);
    }

    let cfg = load_setup()?;
    match cfg.storage_mode {
        StorageMode::Ephemeral => {
            let id = cfg.ephemeral_id.clone().unwrap_or_else(|| "default".into());
            let path = ephemeral_base()?.join(id);
            fs::create_dir_all(&path)?;
            Ok(path)
        }
        StorageMode::Persistent => {
            if let Some(custom) = cfg
                .custom_data_dir
                .as_ref()
                .filter(|s| !s.trim().is_empty())
            {
                let path = PathBuf::from(custom);
                fs::create_dir_all(&path)?;
                return Ok(path);
            }
            let dirs = project_dirs()?;
            let path = dirs.data_dir().to_path_buf();
            fs::create_dir_all(&path)?;
            Ok(path)
        }
    }
}

pub fn db_file_path() -> AppResult<PathBuf> {
    let dir = data_dir()?;
    Ok(dir.join("phishkit.db"))
}

/// Mutable evilginx runtime (`config.json`, buntdb, blacklist) — OS data dir, not the git tree.
pub fn evilginx_data_dir() -> AppResult<PathBuf> {
    let dir = data_dir()?.join("evilginx");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn evilginx_log_path() -> AppResult<PathBuf> {
    Ok(evilginx_data_dir()?.join("evilginx.log"))
}

pub fn evilginx_pid_path() -> AppResult<PathBuf> {
    Ok(evilginx_data_dir()?.join("evilginx.pid"))
}

/// One-time migrate legacy kit-relative DBs into the OS data dir when missing.
pub fn migrate_legacy_db_if_needed(dest: &Path) -> AppResult<()> {
    if dest.is_file() {
        return Ok(());
    }
    let root = crate::kit::kit_root().ok();
    let candidates = [
        root.as_ref().map(|r| r.join("run/desktop/phishkit.db")),
        root.as_ref().map(|r| r.join("run/webui/phishkit.db")),
        BaseDirs::new().map(|b| b.home_dir().join(".phishkit/phishkit.db")),
    ];
    for cand in candidates.into_iter().flatten() {
        if cand.is_file() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&cand, dest)?;
            let bak = cand.with_extension("db.bak");
            let _ = fs::copy(&cand, bak);
            break;
        }
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Call on app start: if ephemeral leftover exists without live config, wipe.
pub fn bootstrap_storage() -> AppResult<SetupConfig> {
    let cfg = load_setup()?;
    if cfg.storage_mode == StorageMode::Ephemeral {
        // Ensure sandbox exists for this session id.
        let _ = data_dir()?;
    } else {
        // Persistent mode: clear any abandoned ephemeral sandboxes.
        let _ = wipe_ephemeral_sandboxes();
    }
    Ok(cfg)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathsInfo {
    pub config_dir: String,
    pub data_dir: String,
    pub db_path: String,
    pub evilginx_data_dir: String,
    pub setup_path: String,
    pub kit_root: String,
    pub storage_mode: StorageMode,
    pub setup_complete: bool,
    pub persona: Persona,
    pub tutorial_completed: bool,
}

pub fn paths_info() -> AppResult<PathsInfo> {
    let cfg = load_setup()?;
    let kit = crate::kit::kit_root()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "(unresolved)".into());
    Ok(PathsInfo {
        config_dir: config_dir()?.display().to_string(),
        data_dir: data_dir()?.display().to_string(),
        db_path: db_file_path()?.display().to_string(),
        evilginx_data_dir: evilginx_data_dir()?.display().to_string(),
        setup_path: setup_path()?.display().to_string(),
        kit_root: kit,
        storage_mode: cfg.storage_mode,
        setup_complete: cfg.setup_complete,
        persona: cfg.persona,
        tutorial_completed: cfg.tutorial_completed,
    })
}
