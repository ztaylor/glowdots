//! The clock, and things worked out from the time: the moon, and today's
//! date at a time of day.

use chrono::{DateTime, Local, NaiveDateTime, TimeDelta};

/// Where a deck gets the time.
#[derive(Clone, Copy, Debug)]
pub struct Clock {
    mode: Mode,
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    /// The real time, moved by an offset.
    Real { offset: TimeDelta },
    /// A clock that only moves when the deck does: the same frames every run.
    Stepped { start: DateTime<Local>, elapsed: f64 },
}

impl Clock {
    /// The real local time.
    pub fn system() -> Self {
        Self { mode: Mode::Real { offset: TimeDelta::zero() } }
    }

    /// Real time, but starting at `start`: for previews of another time.
    pub fn starting_at(start: DateTime<Local>) -> Self {
        Self { mode: Mode::Real { offset: start - Local::now() } }
    }

    /// A clock that starts at `start` and moves only as frames are drawn,
    /// so a run gives the same frames every time. For screenshots, GIFs,
    /// and tests.
    pub fn stepped(start: DateTime<Local>) -> Self {
        Self { mode: Mode::Stepped { start, elapsed: 0.0 } }
    }

    pub fn now(&self) -> DateTime<Local> {
        match self.mode {
            Mode::Real { offset } => Local::now() + offset,
            Mode::Stepped { start, elapsed } => start + TimeDelta::microseconds((elapsed * 1e6).round() as i64),
        }
    }

    /// Move a stepped clock on by `dt` seconds. A real clock moves itself.
    pub fn advance(&mut self, dt: f32) {
        if let Mode::Stepped { elapsed, .. } = &mut self.mode {
            *elapsed += dt as f64;
        }
    }

    pub fn is_stepped(&self) -> bool {
        matches!(self.mode, Mode::Stepped { .. })
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::system()
    }
}

/// A local time from "YYYY-MM-DD HH:MM" (or with ":SS").
pub fn parse_local(text: &str) -> Result<DateTime<Local>, String> {
    let naive = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S"))
        .map_err(|e| format!("{text:?} is not \"YYYY-MM-DD HH:MM\": {e}"))?;
    naive.and_local_timezone(Local).single().ok_or(format!("{text:?} is not a local time"))
}

/// The moon's phase at `now`: (lit fraction 0..1, waxing?). From the mean
/// synodic month and a known new moon; good to within about a day.
pub fn moon_phase(now: DateTime<Local>) -> (f32, bool) {
    const SYNODIC_DAYS: f64 = 29.530_588_853;
    // A new moon: 2000-01-06 18:14 UTC.
    let known_new = DateTime::parse_from_rfc3339("2000-01-06T18:14:00Z").expect("valid date");
    let days = (now - known_new.with_timezone(&Local)).num_seconds() as f64 / 86_400.0;
    let age = days.rem_euclid(SYNODIC_DAYS) / SYNODIC_DAYS; // 0 new, 0.5 full
    let lit = (1.0 - (age * std::f64::consts::TAU).cos()) / 2.0;
    (lit as f32, age < 0.5)
}

/// The date of `now` at a clock time like "6:42 PM".
pub fn today_at(now: DateTime<Local>, clock: &str) -> Option<DateTime<Local>> {
    let time = chrono::NaiveTime::parse_from_str(clock.trim(), "%I:%M %p").ok()?;
    now.date_naive().and_time(time).and_local_timezone(Local).single()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepped_clocks_move_with_frames() {
        let start = parse_local("2026-10-09 15:42").unwrap();
        let mut c = Clock::stepped(start);
        for _ in 0..120 {
            c.advance(1.0 / 60.0);
        }
        assert_eq!((c.now() - start).num_milliseconds(), 2000);
    }

    #[test]
    fn full_moon() {
        // A full moon: 2026-01-03 10:03 UTC.
        let (lit, _) = moon_phase(DateTime::parse_from_rfc3339("2026-01-03T10:03:00Z").unwrap().with_timezone(&Local));
        assert!(lit > 0.98, "{lit}");
    }
}
