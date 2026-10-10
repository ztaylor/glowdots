//! Data feeds: JSON that something else writes and scenes read.
//!
//! The simplest way to put live data on the display is a folder of JSON
//! files. A cron job, a shell script with `curl`, a home automation hub,
//! or an AI agent writes them; a scene reads its feed again each time it
//! starts. Scenes read through [`Feeds`], so the same scene can read a
//! [`Folder`] on a Raspberry Pi and [`Bundled`] samples in a browser.
//!
//! A list feed holds `items`, and may say which day it is for:
//!
//! ```json
//! {"date": "2026-10-06", "items": [{"title": "Trampled by Turtles"}]}
//! ```

use chrono::{DateTime, Local};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::io;
use std::path::{Path, PathBuf};

/// Where scenes read their feeds.
pub trait Feeds {
    /// The text of the feed called `name` (like `"weather.json"`). A feed
    /// that does not exist is an error of kind [`io::ErrorKind::NotFound`].
    fn read(&self, name: &str) -> io::Result<String>;
}

impl<F: Feeds + ?Sized> Feeds for Box<F> {
    fn read(&self, name: &str) -> io::Result<String> {
        (**self).read(name)
    }
}

/// No feeds at all.
#[derive(Clone, Copy, Default, Debug)]
pub struct NoFeeds;

impl Feeds for NoFeeds {
    fn read(&self, _name: &str) -> io::Result<String> {
        Err(io::ErrorKind::NotFound.into())
    }
}

/// Feeds as files in a folder: `name` is a file name in it.
#[derive(Clone, Debug)]
pub struct Folder {
    dir: PathBuf,
}

impl Folder {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

impl Feeds for Folder {
    fn read(&self, name: &str) -> io::Result<String> {
        std::fs::read_to_string(self.dir.join(name))
    }
}

/// Feeds held in memory: samples built into a program, or data that came
/// from somewhere else.
#[derive(Clone, Default, Debug)]
pub struct Bundled {
    files: Vec<(String, String)>,
}

impl Bundled {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add (or replace) a feed.
    pub fn with(mut self, name: impl Into<String>, text: impl Into<String>) -> Self {
        self.insert(name, text);
        self
    }

    /// Add (or replace) a feed.
    pub fn insert(&mut self, name: impl Into<String>, text: impl Into<String>) {
        let (name, text) = (name.into(), text.into());
        match self.files.iter_mut().find(|(n, _)| *n == name) {
            Some(f) => f.1 = text,
            None => self.files.push((name, text)),
        }
    }

    /// The names of the feeds.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(|(n, _)| n.as_str())
    }
}

impl Feeds for Bundled {
    fn read(&self, name: &str) -> io::Result<String> {
        self.files.iter().find(|(n, _)| n == name).map(|(_, t)| t.clone()).ok_or_else(|| io::ErrorKind::NotFound.into())
    }
}

/// Several sources, in order: each feed comes from the first that has it.
/// For example a folder of live data, then built-in samples.
#[derive(Default)]
pub struct Layered {
    layers: Vec<Box<dyn Feeds>>,
}

impl Layered {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source after the ones already here.
    pub fn then(mut self, feeds: impl Feeds + 'static) -> Self {
        self.layers.push(Box::new(feeds));
        self
    }
}

impl Feeds for Layered {
    fn read(&self, name: &str) -> io::Result<String> {
        for layer in &self.layers {
            match layer.read(name) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                other => return other,
            }
        }
        Err(io::ErrorKind::NotFound.into())
    }
}

/// Why a feed could not be read.
#[non_exhaustive]
#[derive(Clone, PartialEq, Debug)]
pub enum FeedError {
    /// There is no such feed (yet).
    Missing,
    /// It is there but could not be read.
    Unreadable(String),
    /// It is not valid JSON, or not the shape the scene expects.
    Invalid { line: usize, column: usize, message: String },
}

impl FeedError {
    /// A short message for the panel, in capitals: "NO FEED YET" or
    /// "BAD FEED: LINE 3".
    pub fn short(&self) -> String {
        match self {
            FeedError::Missing => "NO FEED YET".into(),
            FeedError::Unreadable(_) => "CANNOT READ FEED".into(),
            FeedError::Invalid { line, .. } => format!("BAD FEED: LINE {line}"),
        }
    }
}

impl std::fmt::Display for FeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedError::Missing => write!(f, "no such feed"),
            FeedError::Unreadable(e) => write!(f, "cannot read the feed: {e}"),
            FeedError::Invalid { message, .. } => write!(f, "bad feed: {message}"),
        }
    }
}

impl std::error::Error for FeedError {}

/// Read any JSON feed into `T`.
pub fn json<T: DeserializeOwned>(feeds: &dyn Feeds, name: &str) -> Result<T, FeedError> {
    let text = feeds.read(name).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => FeedError::Missing,
        _ => FeedError::Unreadable(e.to_string()),
    })?;
    serde_json::from_str(&text).map_err(|e| FeedError::Invalid { line: e.line(), column: e.column(), message: e.to_string() })
}

/// What a scene gets when it reads a list feed.
#[non_exhaustive]
pub enum Feed<T> {
    /// The items.
    Ready(Vec<T>),
    /// The feed is dated another day (that date, YYYY-MM-DD).
    Stale(String),
    /// The feed is missing or bad.
    Failed(FeedError),
}

impl<T> Feed<T> {
    /// Turn each item into something else (for example a card to show).
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Feed<U> {
        match self {
            Feed::Ready(items) => Feed::Ready(items.into_iter().map(f).collect()),
            Feed::Stale(date) => Feed::Stale(date),
            Feed::Failed(e) => Feed::Failed(e),
        }
    }
}

/// Read a list feed: `{"date": "YYYY-MM-DD", "items": [...]}`. `date` is
/// optional; if it is there and is not today, the feed is [`Feed::Stale`].
pub fn items<T: DeserializeOwned>(feeds: &dyn Feeds, now: DateTime<Local>, name: &str) -> Feed<T> {
    #[derive(Deserialize)]
    struct File<T> {
        date: Option<String>,
        items: Vec<T>,
    }
    match json::<File<T>>(feeds, name) {
        Err(e) => Feed::Failed(e),
        Ok(f) => match f.date {
            Some(d) if d != now.format("%Y-%m-%d").to_string() => Feed::Stale(d),
            _ => Feed::Ready(f.items),
        },
    }
}

/// How old a feed is at `now`, from its "generated_at" (RFC 3339), in minutes.
pub fn age_minutes(now: DateTime<Local>, generated_at: Option<&str>) -> Option<i64> {
    let at = DateTime::parse_from_rfc3339(generated_at?).ok()?;
    Some((now - at.with_timezone(&Local)).num_minutes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_fall_through_to_samples() {
        let feeds = Layered::new().then(Bundled::new().with("a.json", "1")).then(Bundled::new().with("a.json", "2").with("b.json", "3"));
        assert_eq!(json::<i32>(&feeds, "a.json"), Ok(1));
        assert_eq!(json::<i32>(&feeds, "b.json"), Ok(3));
        assert_eq!(json::<i32>(&feeds, "c.json"), Err(FeedError::Missing));
    }

    #[test]
    fn bad_json_says_where() {
        let feeds = Bundled::new().with("x.json", "{\n  \"items\": [1,,2]\n}");
        let now = crate::time::parse_local("2026-10-06 12:00").unwrap();
        match items::<i32>(&feeds, now, "x.json") {
            Feed::Failed(e) => assert_eq!(e.short(), "BAD FEED: LINE 2"),
            _ => panic!("expected a bad feed"),
        }
    }

    #[test]
    fn dated_feeds_go_stale() {
        let feeds = Bundled::new().with("x.json", r#"{"date": "2026-10-05", "items": [1]}"#);
        let now = crate::time::parse_local("2026-10-06 12:00").unwrap();
        assert!(matches!(items::<i32>(&feeds, now, "x.json"), Feed::Stale(d) if d == "2026-10-05"));
    }
}
