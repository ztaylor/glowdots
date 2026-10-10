//! A status board: is a system alive, and are its parts working? For a
//! home server, a build farm, a fleet of sensors, or an AI agent.
//!
//! - A "SYSTEM STATUS" header bar, with a small mark at its end: a check,
//!   a "!", or an X (these two blink).
//! - The status, with a heart monitor: a steady beat when all is well, an
//!   uneven one for problems, and a flat line when the reports stop.
//! - SYSTEM, JOBS, and FEEDS, each with one light per check. They come on
//!   one by one like a pinball lamp test; then, if all is well, a light
//!   scans across them. A warning blinks slowly, an error fast.
//! - Each problem then gets a page of its own.
//!
//! It reads `status.json` (see `samples/status.json`): `generated_at`
//! (RFC 3339), an overall `status` (ok, warn, or error), and `checks`, each
//! with a group, name, status, and detail. Checks of one group should be
//! next to each other. A report older than 30 minutes means the system has
//! gone silent.

use glowdots::feed::age_minutes;
use glowdots::font::{REGULAR, SMALL, clean};
use glowdots::math::hash;
use glowdots::{Context, Frame, MAX, Rect, Scene, Size, Sprite, Transition, ui};
use serde::Deserialize;

const WIDTH: usize = crate::SIZE.w as usize;

/// The report is written every 10 minutes; older than this means silence.
const STALE_MINUTES: i64 = 30;
const SUMMARY_SECONDS: f32 = 8.0;
const PROBLEM_SECONDS: f32 = 4.5;
/// The status text and the heart monitor start here.
const TEXT_X: i32 = 2;
/// The heart monitor's baseline row, and how fast its write head moves.
const BEAT_BASE: i32 = 20;
const BEAT_SPEED: f32 = 28.0;
/// The groups: label row and light row.
const LABEL_Y: i32 = 23;
const LIGHT_Y: i32 = 29;
const LAMP_TEST_SECONDS: f32 = 0.9;

#[derive(Deserialize, Default)]
#[serde(default)]
struct Report {
    generated_at: Option<String>,
    status: String,
    checks: Vec<Check>,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
struct Check {
    group: String,
    name: String,
    status: String,
    detail: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Level {
    Ok,
    Warn,
    Error,
}

impl Level {
    fn of(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "ok" => Level::Ok,
            "warn" | "warning" => Level::Warn,
            _ => Level::Error,
        }
    }
}

/// How the heart monitor beats.
#[derive(Clone, Copy, PartialEq)]
enum Pulse {
    Steady,
    Uneven,
    Flat,
}

/// A group of checks: its label and the status of each check.
struct Group {
    label: String,
    lights: Vec<Level>,
}

pub struct Status {
    name: String,
    header: String,
    feed_name: String,
    stale_minutes: i64,
    overall: Level,
    pulse: Pulse,
    title: String,
    groups: Vec<Group>,
    problems: Vec<Check>,
    marks: [Sprite; 3],
}

impl Status {
    pub fn new() -> Self {
        Self {
            name: "status".into(),
            header: "SYSTEM STATUS".into(),
            feed_name: "status.json".into(),
            stale_minutes: STALE_MINUTES,
            overall: Level::Ok,
            pulse: Pulse::Steady,
            title: String::new(),
            groups: Vec::new(),
            problems: Vec::new(),
            marks: [Sprite::from_rows(&CHECK), Sprite::from_rows(&WARN), Sprite::from_rows(&CROSS)],
        }
    }

    /// Call the scene something else (for logs and choosing it).
    pub fn named(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    /// The header ("SYSTEM STATUS").
    pub fn title(mut self, title: &str) -> Self {
        self.header = clean(title);
        self
    }

    /// Read another feed than `status.json`.
    pub fn feed(mut self, name: &str) -> Self {
        self.feed_name = name.into();
        self
    }

    /// How old a report can be, in minutes, before the system counts as
    /// silent (default 30).
    pub fn stale_after(mut self, minutes: i64) -> Self {
        self.stale_minutes = minutes;
        self
    }

    /// The header bar, with a small status mark at its right end. A
    /// problem's mark blinks.
    fn header(&self, out: &mut Frame, status: Level, t: f32) {
        ui::header(out, &self.header, "");
        let mark = &self.marks[status as usize];
        if status == Level::Ok || (t * 2.0) as i32 % 2 == 0 {
            out.blit(mark, WIDTH as i32 - 2 - mark.w as i32, 1);
        }
    }

    /// The heart monitor: a trace drawn by a write head that sweeps left
    /// to right. Behind the head the trace fades; just ahead it is blank.
    fn draw_monitor(&self, out: &mut Frame, t: f32) {
        let (x0, x1) = (TEXT_X, WIDTH as i32 - 1);
        let width = x1 - x0;
        let sweep = t * BEAT_SPEED;
        let head = x0 + (sweep % width as f32) as i32;
        let lap = (sweep / width as f32) as u32;
        let mut prev: Option<i32> = None;
        for x in x0..x1 {
            // How far behind the head this column is (it wraps around).
            let behind = (head - x).rem_euclid(width);
            if behind == 0 || behind > width - 4 {
                prev = None;
                continue; // the gap just ahead of the head
            }
            // Columns left of the head were drawn this lap, the rest last lap.
            let this_lap = if x <= head { lap } else { lap.wrapping_sub(1) };
            let y = BEAT_BASE - self.beat(x - x0, this_lap);
            let level = if behind < 6 {
                MAX
            } else if behind < 30 {
                2
            } else {
                1
            };
            // Join steep steps so spikes are solid lines.
            if let Some(py) = prev {
                for yy in py.min(y)..=py.max(y) {
                    out.set(x, yy, level);
                }
            }
            out.set(x, y, level);
            prev = Some(y);
        }
    }

    /// Height of the trace above the baseline at column x.
    fn beat(&self, x: i32, lap: u32) -> i32 {
        const PERIOD: i32 = 21;
        // A heartbeat: P wave, the QRS spike, T wave.
        const SHAPE: [i32; PERIOD as usize] = [0, 0, 0, 1, 1, 0, 0, -1, 4, -1, 0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0];
        let k = x.rem_euclid(PERIOD) as usize;
        match self.pulse {
            Pulse::Flat => 0,
            Pulse::Steady => SHAPE[k],
            Pulse::Uneven => {
                // Some beats are missed and the rest vary in height.
                let h = hash((x / PERIOD) as u32, lap);
                if h < 0.3 { 0 } else { (SHAPE[k] as f32 * (0.5 + h * 0.6)).round() as i32 }
            }
        }
    }

    /// SYSTEM, JOBS, FEEDS: a label over one light per check, each pair
    /// centred over the other.
    fn draw_groups(&self, out: &mut Frame, t: f32) {
        const LIGHT_W: i32 = 2;
        const STEP: i32 = 3;
        let lights_w = |g: &Group| g.lights.len() as i32 * STEP - (STEP - LIGHT_W);
        let widths: Vec<i32> = self.groups.iter().map(|g| SMALL.width(&g.label).max(lights_w(g))).collect();
        let gap = 5;
        let total: i32 = widths.iter().sum::<i32>() + gap * (widths.len() as i32 - 1).max(0);
        let mut x = (WIDTH as i32 - total) / 2;
        let all_ok = self.overall == Level::Ok && self.pulse != Pulse::Flat;
        let count: usize = self.groups.iter().map(|g| g.lights.len()).sum();
        // The scanning light goes back and forth across all the lights.
        let scan = {
            let p = ((t - LAMP_TEST_SECONDS) * 0.8).rem_euclid(2.0);
            let p = if p > 1.0 { 2.0 - p } else { p };
            (p * (count.max(1) - 1) as f32).round() as usize
        };
        let mut n = 0;
        for (g, &w) in self.groups.iter().zip(&widths) {
            SMALL.draw(out, &g.label, x + (w - SMALL.width(&g.label)) / 2, LABEL_Y, 2);
            let lights_x = x + (w - lights_w(g)) / 2;
            for (i, &s) in g.lights.iter().enumerate() {
                let lit_at = n as f32 / count.max(1) as f32 * LAMP_TEST_SECONDS;
                let level = if t < lit_at {
                    0
                } else if t < LAMP_TEST_SECONDS {
                    MAX // lamp test: all on, one after another
                } else {
                    match s {
                        Level::Ok if all_ok && n == scan => MAX,
                        Level::Ok => 1,
                        Level::Warn => {
                            if (t * 2.5) as i32 % 2 == 0 {
                                MAX
                            } else {
                                1
                            }
                        }
                        Level::Error => {
                            if (t * 6.0) as i32 % 2 == 0 {
                                MAX
                            } else {
                                0
                            }
                        }
                    }
                };
                let lx = lights_x + i as i32 * STEP;
                out.fill(Rect::new(lx, LIGHT_Y, lx + LIGHT_W, LIGHT_Y + 3), level);
                n += 1;
            }
            x += w + gap;
        }
    }
}

impl Scene for Status {
    fn name(&self) -> &str {
        &self.name
    }

    fn enter(&mut self, cx: &Context) {
        let report = cx.json::<Report>(&self.feed_name).ok();
        self.groups.clear();
        self.problems.clear();
        let Some(r) = report else {
            self.overall = Level::Warn;
            self.pulse = Pulse::Flat;
            self.title = "NO REPORT".into();
            return;
        };
        if age_minutes(cx.now, r.generated_at.as_deref()).is_some_and(|a| a > self.stale_minutes) {
            // The reports stopped coming: that is the problem.
            self.overall = Level::Warn;
            self.pulse = Pulse::Flat;
            self.title = "SILENT".into();
        } else {
            self.overall = Level::of(&r.status);
            self.pulse = if self.overall == Level::Ok { Pulse::Steady } else { Pulse::Uneven };
            self.title = match self.overall {
                Level::Ok => "ALL GOOD",
                Level::Warn => "WARNING",
                Level::Error => "ERROR",
            }
            .into();
        }
        for c in &r.checks {
            let label = clean(&c.group);
            match self.groups.last_mut() {
                Some(g) if g.label == label => g.lights.push(Level::of(&c.status)),
                _ => self.groups.push(Group { label, lights: vec![Level::of(&c.status)] }),
            }
        }
        self.problems = r.checks.iter().filter(|c| Level::of(&c.status) != Level::Ok).cloned().collect();
    }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        let t = cx.t;
        if t < SUMMARY_SECONDS || self.problems.is_empty() {
            self.header(out, self.overall, t);
            REGULAR.draw(out, &self.title, TEXT_X, 8, MAX);
            self.draw_monitor(out, t);
            self.draw_groups(out, t);
            return;
        }
        // A page for each problem: what failed, and why.
        let i = (((t - SUMMARY_SECONDS) / PROBLEM_SECONDS) as usize).min(self.problems.len() - 1);
        let local = t - SUMMARY_SECONDS - i as f32 * PROBLEM_SECONDS;
        let c = &self.problems[i];
        self.header(out, Level::of(&c.status), local);
        let width = WIDTH as i32 - TEXT_X - 1;
        let name = SMALL.wrap(&clean(&c.name), width);
        let detail = SMALL.wrap(&clean(&c.detail), width);
        let mut y = 9;
        for line in name.iter().take(2) {
            SMALL.draw(out, line, TEXT_X, y, MAX);
            y += 6;
        }
        y += 1;
        for line in detail.iter().take(if name.len() > 1 { 2 } else { 3 }) {
            SMALL.draw(out, line, TEXT_X, y, 2);
            y += 6;
        }
    }

    fn duration(&self) -> f32 {
        SUMMARY_SECONDS + self.problems.len() as f32 * PROBLEM_SECONDS
    }

    fn transition(&self) -> Transition {
        Transition::Wipe
    }

    fn size(&self) -> Option<Size> {
        Some(crate::SIZE)
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::new()
    }
}

// Header marks, 5 dots tall: '#' lit, ' ' off. Order matches Level:
// Ok, Warn, Error.

const CHECK: [&str; 5] = ["    #", "   # ", "# #  ", " #   ", "     "];
const WARN: [&str; 5] = ["#", "#", "#", " ", "#"];
const CROSS: [&str; 5] = ["#   #", " # # ", "  #  ", " # # ", "#   #"];
