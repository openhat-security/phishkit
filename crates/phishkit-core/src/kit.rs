use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

/// Resolve the phishkit kit root (repo/dir containing kit/evilginx/, vendor/).
///
/// Resolution order:
///   1. `PHISHKIT_ROOT` env var (explicit override).
///   2. Parents of the Cargo manifest dir (development / `cargo run`).
///   3. Parents of the running executable, plus macOS `.app` sibling dirs
///      (`../Resources`, `../share/phishkit`) for bundled layouts.
///   4. Standard installed locations (Homebrew `share/phishkit`, the app's
///      Application Support dir, `~/.phishkit`) so a brew-installed desktop
///      app or CLI finds the kit without a manual `PHISHKIT_ROOT`.
pub fn kit_root() -> AppResult<PathBuf> {
    if let Ok(p) = std::env::var("PHISHKIT_ROOT") {
        let root = PathBuf::from(p);
        validate_kit_root(&root)?;
        return Ok(root);
    }

    for candidate in candidate_roots() {
        if validate_kit_root(&candidate).is_ok() {
            return Ok(candidate);
        }
    }

    Err(AppError::msg(
        "Could not find phishkit root. Set PHISHKIT_ROOT to the kit directory.",
    ))
}

/// Ordered list of directories that might be a kit root, excluding the explicit
/// `PHISHKIT_ROOT` override (handled separately so its failure is surfaced).
fn candidate_roots() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    // Walk parents from the Cargo manifest (apps/desktop/src-tauri -> repo root).
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    push_ancestors(&mut out, &manifest, 8);

    // Packaged: look next to the executable and walk its parents.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // macOS .app: exe is Contents/MacOS/<bin>; bundled resources live in
            // Contents/Resources (Tauri `bundle.resources`) and a CLI cask may
            // drop the kit beside the binary under share/phishkit.
            out.push(dir.join("../Resources"));
            out.push(dir.join("../Resources/kit-root"));
            out.push(dir.join("../share/phishkit"));
            // CLI tarball ships a `kit-root/` next to the binary (and the cask
            // symlinks the binary out of the staged Caskroom dir, which
            // current_exe resolves back to), so look beside and one up.
            out.push(dir.join("kit-root"));
            out.push(dir.join("../kit-root"));
            push_ancestors(&mut out, dir, 8);
        }
    }

    // Standard installed locations.
    for prefix in install_prefixes() {
        out.push(prefix.join("share/phishkit"));
        out.push(prefix.join("share/phishkit/kit-root"));
    }
    if let Some(home) = home_dir() {
        // macOS Application Support (both the historic and current bundle ids).
        for id in ["com.phishkit.phishkit", "com.phishkit.app"] {
            out.push(
                home.join("Library/Application Support")
                    .join(id)
                    .join("kit-root"),
            );
        }
        out.push(home.join(".phishkit"));
    }

    out
}

fn push_ancestors(out: &mut Vec<PathBuf>, start: &Path, max: usize) {
    let mut cur = Some(start.to_path_buf());
    for _ in 0..max {
        match cur {
            Some(ref dir) => {
                out.push(dir.clone());
                cur = dir.parent().map(|p| p.to_path_buf());
            }
            None => break,
        }
    }
}

fn install_prefixes() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("HOMEBREW_PREFIX") {
        if !p.is_empty() {
            v.push(PathBuf::from(p));
        }
    }
    v.push(PathBuf::from("/opt/homebrew"));
    v.push(PathBuf::from("/usr/local"));
    v
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn validate_kit_root(root: &Path) -> AppResult<()> {
    // Prefer kit/evilginx; accept legacy evilginx/ during migration.
    if root.join("kit/evilginx/phishlets").is_dir() || root.join("evilginx/phishlets").is_dir() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn validate_kit_root_and_evilginx_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(validate_kit_root(tmp.path()).is_err());
        fs::create_dir_all(tmp.path().join("kit/evilginx/phishlets")).unwrap();
        assert!(validate_kit_root(tmp.path()).is_ok());
        assert_eq!(evilginx_dir(tmp.path()), tmp.path().join("kit/evilginx"));
    }

    #[test]
    fn validate_kit_root_accepts_legacy_layout() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("evilginx/phishlets")).unwrap();
        assert!(validate_kit_root(tmp.path()).is_ok());
    }

    #[test]
    fn candidate_roots_include_homebrew_share() {
        // SAFETY: single-threaded test; restores env afterwards.
        let prev = std::env::var_os("HOMEBREW_PREFIX");
        unsafe { std::env::set_var("HOMEBREW_PREFIX", "/fake/brew") };
        let roots = candidate_roots();
        match prev {
            Some(v) => unsafe { std::env::set_var("HOMEBREW_PREFIX", v) },
            None => unsafe { std::env::remove_var("HOMEBREW_PREFIX") },
        }
        assert!(
            roots.contains(&PathBuf::from("/fake/brew/share/phishkit")),
            "expected homebrew share/phishkit candidate, got: {roots:?}"
        );
    }
}
