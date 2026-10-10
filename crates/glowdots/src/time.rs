//! The clock, with a way to pretend it is another time, and the moon.

use chrono::{DateTime, Local, NaiveDateTime, TimeDelta};
use std::sync::OnceLock;

/// How far the simulated clock is from the real one (see [`simulate_now`]).
static CLOCK_OFFSET: OnceLock<TimeDelta> = OnceLock::new();

/// Pretend the time is `start` ("YYYY-MM-DD HH:MM"). The clock then runs
/// on from there. For previews and screenshots.
pub fn simulate_now(start: &str) -> Result<(), String> {
    let naive = NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M").map_err(|e| format!("--now {start}: {e}"))?;
    let at = naive.and_local_timezone(Local).single().ok_or(format!("--now {start}: not a local time"))?;
    CLOCK_OFFSET.set(at - Local::now()).map_err(|_| "clock already set".to_string())
}

/// The local time now (or the simulated time, after [`simulate_now`]).
pub fn now() -> DateTime<Local> {
    Local::now() + CLOCK_OFFSET.get().copied().unwrap_or_default()
}

/// The moon's phase now: (lit fraction 0..1, waxing?). From the mean
/// synodic month and a known new moon; good to within about a day.
pub fn moon_phase() -> (f32, bool) {
    const SYNODIC_DAYS: f64 = 29.530_588_853;
    // A new moon: 2000-01-06 18:14 UTC.
    let known_new = DateTime::parse_from_rfc3339("2000-01-06T18:14:00Z").expect("valid date");
    let days = (now() - known_new.with_timezone(&Local)).num_seconds() as f64 / 86_400.0;
    let age = days.rem_euclid(SYNODIC_DAYS) / SYNODIC_DAYS; // 0 new, 0.5 full
    let lit = (1.0 - (age * std::f64::consts::TAU).cos()) / 2.0;
    (lit as f32, age < 0.5)
}

/// Today's date at a clock time like "6:42 PM".
pub fn today_at(clock: &str) -> Option<DateTime<Local>> {
    let time = chrono::NaiveTime::parse_from_str(clock.trim(), "%I:%M %p").ok()?;
    now().date_naive().and_time(time).and_local_timezone(Local).single()
}
