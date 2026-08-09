use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

/// Resolve the phishkit kit root (repo containing kit/evilginx/, vendor/).
pub fn kit_root() -> AppResult<PathBuf> {
    if let Ok(p) = std::env::var("PHISHKIT_ROOT") {
        let root = PathBuf::from(p);
        validate_kit_root(&root)?;
        return Ok(root);
    }

    // Walk parents from the Cargo manifest (apps/desktop/src-tauri → repo root).
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut cur = Some(manifest.as_path());
    for _ in 0..8 {
        if let Some(dir) = cur {
            if validate_kit_root(dir).is_ok() {
                return Ok(dir.to_path_buf());
            }
            cur = dir.parent();
        }
    }

    // Packaged: look next to the executable, then walk parents.
    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..8 {
            if let Some(ref dir) = cur {
                if validate_kit_root(dir).is_ok() {
                    return Ok(dir.clone());
                }
                cur = dir.parent().map(|p| p.to_path_buf());
            }
        }
    }

    Err(AppError::msg(
        "Could not find phishkit root. Set PHISHKIT_ROOT to the kit directory.",
    ))
}

pub fn validate_kit_root(root: &Path) -> AppResult<()> {
    // Prefer kit/evilginx; accept legacy evilginx/ during migration.
    if root.join("kit/evilginx/phishlets").is_dir() || root.join("kit/evilginx/phishlets").is_dir()
    {
        Ok(())
    } else {
        Err(AppError::msg(format!(
            "Not a phishkit root: {}",
            root.display()
        )))
    }
}

/// Immutable kit evilginx directory (`kit/evilginx`, with legacy `evilginx` fallback).
pub fn evilginx_dir(root: &Path) -> PathBuf {
    let modern = root.join("kit/evilginx");
    if modern.is_dir() {
        modern
    } else {
        root.join("evilginx")
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct KitInfo {
    pub root: String,
    pub evilginx_bin: bool,
    pub community_index: bool,
    pub active_phishlets: usize,
}

pub fn kit_info() -> AppResult<KitInfo> {
    let root = kit_root()?;
    let eg = evilginx_dir(&root);
    let evilginx_bin = eg.join("run/evilginx").is_file() || eg.join("bin/evilginx").is_file();
    let community_index = root.join("vendor/community-phishlets/index.json").is_file();
    let phishlets_dir = eg.join("phishlets");
    let active_phishlets = std::fs::read_dir(&phishlets_dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|x| x == "yaml" || x == "yml")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0);

    Ok(KitInfo {
        root: root.display().to_string(),
        evilginx_bin,
        community_index,
        active_phishlets,
    })
}
