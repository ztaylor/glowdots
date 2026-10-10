//! Data feeds: JSON files that something else writes and scenes read.
//!
//! The simplest way to put live data on the display is a folder of JSON
//! files. A cron job, a shell script with `curl`, a home automation hub,
//! or an AI agent writes them; a scene reads its file again each time it
//! starts. This module reads such a file and says whether it is fresh.
//!
//! A feed file holds `items`, and may say which day it is for:
//!
//! ```json
//! {"date": "2026-10-06", "items": [{"title": "Trampled by Turtles"}]}
//! ```

use chrono::{DateTime, Local};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::path::Path;

/// What a scene gets when it reads a feed.
pub enum Feed<T> {
    /// No file, or a file we cannot read.
    Missing,
    /// The file is for another day (its date).
    Stale(String),
    Ready(Vec<T>),
}

#[derive(Deserialize)]
struct File<T> {
    /// YYYY-MM-DD. If present, the feed is only good for that day.
    date: Option<String>,
    items: Vec<T>,
}

/// Read a feed file of items. A file dated another day is `Stale`.
pub fn load<T: DeserializeOwned>(path: &Path) -> Feed<T> {
    let parsed = std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str::<File<T>>(&s).map_err(|e| e.to_string()));
    match parsed {
        Err(_) => Feed::Missing,
        Ok(f) => {
            let today = crate::time::now().format("%Y-%m-%d").to_string();
            match f.date {
                Some(d) if d != today => Feed::Stale(d),
                _ => Feed::Ready(f.items),
            }
        }
    }
}

/// Read any JSON document. `None` if the file is missing or malformed.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// How old a feed is, from its "generated_at" (RFC 3339), in minutes.
pub fn age_minutes(generated_at: Option<&str>) -> Option<i64> {
    let at = DateTime::parse_from_rfc3339(generated_at?).ok()?;
    Some((crate::time::now() - at.with_timezone(&Local)).num_minutes())
}
