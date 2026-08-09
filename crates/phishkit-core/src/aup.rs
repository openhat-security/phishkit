use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use crate::db::{now_iso, with_db};
use crate::error::{AppError, AppResult};

const KEY: &str = "aup_accepted_at";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AupStatus {
    pub accepted: bool,
    pub accepted_at: Option<String>,
}

pub fn get_aup_status() -> AppResult<AupStatus> {
    with_db(|conn| {
        let accepted_at: Option<String> = conn
            .query_row(
                "SELECT value FROM app_meta WHERE key = ?1",
                params![KEY],
                |r| r.get(0),
            )
            .optional()?;
        Ok(AupStatus {
            accepted: accepted_at
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false),
            accepted_at,
        })
    })
}

pub fn accept_aup() -> AppResult<AupStatus> {
    let now = now_iso();
    with_db(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO app_meta(key, value) VALUES(?1, ?2)",
            params![KEY, now],
        )?;
        Ok(())
    })?;
    get_aup_status()
}

pub fn require_aup() -> AppResult<()> {
    let s = get_aup_status()?;
    if !s.accepted {
        return Err(AppError::msg(
            "Accept the authorized-use acknowledgment before sending campaigns",
        ));
    }
    Ok(())
}
