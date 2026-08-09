use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use chrono::Utc;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tar::Archive;

use crate::error::{AppError, AppResult};
use crate::kit::kit_root;

#[derive(Debug, Deserialize)]
struct LockFile {
    sources: Vec<LockSource>,
}

#[derive(Debug, Deserialize, Clone)]
struct LockSource {
    id: String,
    repo: String,
    commit: String,
    priority: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommunityPhishlet {
    pub name: String,
    pub source: String,
    pub repo: String,
    pub commit: String,
    pub priority: u32,
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub merged_count: usize,
    pub collision_count: usize,
    pub sources: Vec<SourceMeta>,
    pub index_path: String,
}

#[derive(Debug, Serialize)]
pub struct SourceMeta {
    pub id: String,
    pub repo: String,
    pub commit: String,
    pub priority: u32,
    pub yaml_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub name: String,
    pub dest: String,
}

fn sha256_bytes(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

fn lock_path(root: &Path) -> PathBuf {
    root.join("kit/evilginx/community-phishlets.lock.json")
}

fn out_root(root: &Path) -> PathBuf {
    root.join("vendor/community-phishlets")
}

fn fetch_tarball(repo: &str, commit: &str) -> AppResult<Vec<u8>> {
    let url = format!("https://codeload.github.com/{repo}/tar.gz/{commit}");
    let client = reqwest::blocking::Client::builder()
        .user_agent("phishkit-desktop/0.1 (authorized-assessment-kit)")
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let resp = client.get(&url).send()?.error_for_status()?;
    Ok(resp.bytes()?.to_vec())
}

fn extract_yamls(tarball: &[u8], dest: &Path) -> AppResult<Vec<PathBuf>> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest)?;

    let gz = GzDecoder::new(Cursor::new(tarball));
    let mut archive = Archive::new(gz);
    let mut written = Vec::new();

    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path()?.to_path_buf();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.starts_with('.') {
            continue;
        }
        let lower = name.to_ascii_lowercase();
        if !(lower.ends_with(".yaml") || lower.ends_with(".yml")) {
            continue;
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        let mut out = dest.join(&name);
        if out.exists() {
            let existing = fs::read(&out)?;
            if sha256_bytes(&existing) != sha256_bytes(&data) {
                let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or("dup");
                let ext = out.extension().and_then(|s| s.to_str()).unwrap_or("yaml");
                out = dest.join(format!("{stem}__dup.{ext}"));
            }
        }
        fs::write(&out, &data)?;
        written.push(out);
    }
    written.sort();
    Ok(written)
}

pub fn sync_community_phishlets() -> AppResult<SyncResult> {
    let root = kit_root()?;
    let lock: LockFile = serde_json::from_str(&fs::read_to_string(lock_path(&root))?)?;
    let mut sources = lock.sources;
    sources.sort_by_key(|s| s.priority);

    let out = out_root(&root);
    fs::create_dir_all(&out)?;

    let mut meta_sources = Vec::new();
    let mut per_source: HashMap<String, Vec<CommunityPhishlet>> = HashMap::new();

    for src in &sources {
        let dest = out.join(&src.id);
        let blob = fetch_tarball(&src.repo, &src.commit)?;
        let paths = extract_yamls(&blob, &dest)?;
        let mut entries = Vec::new();
        for p in paths {
            let raw = fs::read(&p)?;
            let rel = p
                .strip_prefix(&out)
                .unwrap_or(&p)
                .to_string_lossy()
                .to_string();
            entries.push(CommunityPhishlet {
                name: p.file_name().unwrap().to_string_lossy().to_string(),
                source: src.id.clone(),
                repo: src.repo.clone(),
                commit: src.commit.clone(),
                priority: src.priority,
                path: rel,
                sha256: sha256_bytes(&raw),
                bytes: raw.len() as u64,
            });
        }
        let yaml_count = entries.len();
        per_source.insert(src.id.clone(), entries);
        meta_sources.push(SourceMeta {
            id: src.id.clone(),
            repo: src.repo.clone(),
            commit: src.commit.clone(),
            priority: src.priority,
            yaml_count,
        });
    }

    let mut merged: HashMap<String, CommunityPhishlet> = HashMap::new();
    let mut collisions = 0usize;
    for src in &sources {
        for ent in per_source.get(&src.id).into_iter().flatten() {
            let key = ent.name.to_ascii_lowercase();
            if merged.contains_key(&key) {
                collisions += 1;
                continue;
            }
            merged.insert(key, ent.clone());
        }
    }

    let mut list: Vec<_> = merged.into_values().collect();
    list.sort_by_key(|a| a.name.to_ascii_lowercase());

    let index = serde_json::json!({
        "generated_at": Utc::now().to_rfc3339(),
        "lock_file": "kit/evilginx/community-phishlets.lock.json",
        "count": list.len(),
        "collision_count": collisions,
        "phishlets": list,
    });
    let index_path = out.join("index.json");
    fs::write(&index_path, serde_json::to_string_pretty(&index)? + "\n")?;

    let meta = serde_json::json!({
        "generated_at": Utc::now().to_rfc3339(),
        "sources": meta_sources,
        "merged_count": index["count"],
        "collision_count": collisions,
    });
    fs::write(
        out.join("_meta.json"),
        serde_json::to_string_pretty(&meta)? + "\n",
    )?;

    Ok(SyncResult {
        merged_count: index["count"].as_u64().unwrap_or(0) as usize,
        collision_count: collisions,
        sources: meta_sources,
        index_path: index_path.display().to_string(),
    })
}

pub fn list_community_phishlets(query: Option<String>) -> AppResult<Vec<CommunityPhishlet>> {
    let root = kit_root()?;
    let index_path = out_root(&root).join("index.json");
    if !index_path.is_file() {
        return Err(AppError::msg(
            "Community pack index missing under vendor/community-phishlets/. Packs ship in-repo; run Sync community phishlets to refresh pins if needed.",
        ));
    }
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(index_path)?)?;
    let mut list: Vec<CommunityPhishlet> = serde_json::from_value(v["phishlets"].clone())?;
    if let Some(q) = query {
        let q = q.to_ascii_lowercase();
        if !q.is_empty() {
            list.retain(|p| p.name.to_ascii_lowercase().contains(&q));
        }
    }
    Ok(list)
}

pub fn import_community_phishlet(name: String) -> AppResult<ImportResult> {
    let root = kit_root()?;
    let list = list_community_phishlets(None)?;
    let item = list
        .into_iter()
        .find(|p| p.name == name || p.name.eq_ignore_ascii_case(&name))
        .ok_or_else(|| AppError::msg(format!("Phishlet not in community index: {name}")))?;

    let src = out_root(&root).join(&item.path);
    if !src.is_file() {
        return Err(AppError::msg(format!(
            "Missing file on disk: {}",
            src.display()
        )));
    }

    // Sanitize destination basename
    let safe = Path::new(&item.name)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AppError::msg("Invalid phishlet name"))?;
    if !safe.ends_with(".yaml") && !safe.ends_with(".yml") {
        return Err(AppError::msg("Phishlet must be a .yaml file"));
    }

    let dest_dir = root.join("kit/evilginx/phishlets");
    fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(safe);
    fs::copy(&src, &dest)?;

    Ok(ImportResult {
        name: safe.to_string(),
        dest: dest.display().to_string(),
    })
}

pub fn list_active_phishlets() -> AppResult<Vec<String>> {
    let root = kit_root()?;
    let dir = root.join("kit/evilginx/phishlets");
    let mut names = Vec::new();
    if dir.is_dir() {
        for e in fs::read_dir(dir)? {
            let e = e?;
            let p = e.path();
            if p.extension()
                .map(|x| x == "yaml" || x == "yml")
                .unwrap_or(false)
            {
                if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
                    names.push(n.to_string());
                }
            }
        }
    }
    names.sort();
    Ok(names)
}
