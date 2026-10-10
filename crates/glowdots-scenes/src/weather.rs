//! Weather: an animated picture of the sky as it is now, and the numbers.
//!
//! Left: a window onto the sky that matches the current conditions. Sun
//! with turning rays (or the moon at its phase, and stars, at night),
//! drifting clouds, fog banks, drizzle, rain, sleet, snow, or a storm with
//! lightning. Clouds and rain move with the real wind.
//!
//! Right: the temperature in big digits, and short pages that change:
//! the condition with today's high and low, wind, the next 12 hours as a
//! little chart, rain coming, tomorrow, and sunset or sunrise. A National
//! Weather Service alert takes over the pages.
//!
//! It reads `weather.json` (see `samples/weather.json`): the conditions
//! now, today, tomorrow, the next 12 hours, and any alerts. `icon` is one
//! of clear, partly_cloudy, cloudy, fog, drizzle, rain, sleet, snow, or
//! thunder.

use anyhow::Result;
use chrono::{DateTime, Local};
use glowdots::art;
use glowdots::font::{SMALL, clean};
use glowdots::math::{Rng, hash};
use glowdots::time::{moon_phase, today_at};
use glowdots::{Context, Frame, MAX, Rect, Scene, Size, Sprite, Transition};
use serde::Deserialize;

const HEIGHT: usize = crate::SIZE.h as usize;

/// The sky window and the text column.
const ART: Rect = Rect::new(0, 0, 44, HEIGHT as i32);
const TEXT_X: i32 = 45;
const TEXT_W: i32 = 35;
const PAGE_SECONDS: f32 = 3.5;
/// Letters per second as a page types in.
const TYPE_SPEED: f32 = 50.0;

#[derive(Deserialize, Default)]
#[serde(default)]
struct Feed {
    now: Now,
    today: Day,
    tomorrow: Day,
    next_12h: Vec<Hour>,
    alerts: Vec<serde_json::Value>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Now {
    temp_f: Option<f32>,
    feels_f: Option<f32>,
    icon: String,
    is_day: Option<bool>,
    wind_mph: f32,
    gust_mph: f32,
    wind_dir: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Day {
    high_f: Option<f32>,
    low_f: Option<f32>,
    icon: String,
    sunrise: String,
    sunset: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Hour {
    time: String,
    temp_f: Option<f32>,
    icon: String,
    precip_chance_pct: f32,
}

/// What the right column shows, a page at a time.
enum Page {
    Text(String, String),
    /// The wind's direction with an arrow showing where it blows ("WNW"),
    /// over its speed ("4 MPH", or "8-14 MPH" with gusts).
    Wind(String, String),
    /// The next 12 hours: temperatures and rain chances.
    Chart(Vec<f32>, Vec<f32>),
}

/// Rain, sleet pellets, or snowflakes.
struct Drop {
    x: f32,
    y: f32,
    speed: f32,
    phase: f32,
}

pub struct Weather {
    name: String,
    feed_name: String,
    waiting: Option<String>,
    error: String,
    now: DateTime<Local>,
    digits: Vec<Sprite>,
    feed: Option<Feed>,
    pages: Vec<Page>,
    drops: Vec<Drop>,
    rng: Rng,
    /// Lightning: time of the next strike, and when the last one was.
    next_strike: f32,
    last_strike: f32,
    bolt: Vec<(i32, i32)>,
}

impl Weather {
    pub fn new() -> Result<Self> {
        Ok(Self {
            name: "weather".into(),
            feed_name: "weather.json".into(),
            waiting: None,
            error: String::new(),
            now: Local::now(),
            digits: crate::digits::big()?,
            feed: None,
            pages: Vec::new(),
            drops: Vec::new(),
            rng: Rng(0x5a17_7e42),
            next_strike: 2.0,
            last_strike: -10.0,
            bolt: Vec::new(),
        })
    }

    /// Call the scene something else (for logs and choosing it).
    pub fn named(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    /// Read another feed than `weather.json`.
    pub fn feed(mut self, name: &str) -> Self {
        self.feed_name = name.into();
        self
    }

    /// What to say under "NO WEATHER YET" when the feed is missing or bad.
    /// By default it says what is wrong ("NO FEED YET", "BAD FEED: LINE 3").
    pub fn waiting(mut self, text: &str) -> Self {
        self.waiting = Some(text.into());
        self
    }

    fn icon(&self) -> &str {
        self.feed.as_ref().map_or("", |f| f.now.icon.as_str())
    }

    fn is_day(&self) -> bool {
        self.feed.as_ref().and_then(|f| f.now.is_day).unwrap_or(true)
    }

    /// Wind in dots per second along x (+ = blowing to the right/east).
    fn drift(&self) -> f32 {
        let Some(f) = &self.feed else { return 0.0 };
        let deg = art::compass(&f.now.wind_dir).unwrap_or(270.0);
        // Wind from the west (270°) blows east, to the right.
        -deg.to_radians().sin() * (0.6 + f.now.wind_mph * 0.35)
    }

    fn build_pages(&mut self) {
        let Some(f) = &self.feed else { return };
        let deg = |v: Option<f32>| v.map_or("--".to_string(), |v| format!("{}°", v.round() as i32));
        let mut pages = Vec::new();

        for alert in &f.alerts {
            let name = ["event", "headline", "title"].iter().find_map(|k| alert.get(k).and_then(|v| v.as_str())).unwrap_or("WEATHER ALERT");
            // A long name is split over pages; small text that scrolls smears.
            let name = clean(name).replace("THUNDERSTORM", "T-STORM");
            for part in SMALL.wrap(&name, TEXT_W) {
                pages.push(Page::Text("! ALERT !".into(), part));
            }
        }
        let hl = |d: &Day| format!("H{} L{}", deg(d.high_f).trim_end_matches('°'), deg(d.low_f).trim_end_matches('°'));
        pages.push(Page::Text(condition(&f.now.icon, self.is_day()).into(), hl(&f.today)));

        if let (Some(t), Some(feels)) = (f.now.temp_f, f.now.feels_f) {
            if (t - feels).abs() >= 3.0 {
                pages.push(Page::Text("FEELS".into(), format!("LIKE {}", deg(Some(feels)))));
            }
        }
        // Wind in mph; gusts as a range when they are well above it: "8-14".
        // The unit goes when there is no room for it.
        let wind = f.now.wind_mph.round() as i32;
        let gust = f.now.gust_mph.round() as i32;
        let speed = if gust >= wind + 5 { format!("{wind}-{gust}") } else { wind.to_string() };
        let with_unit = format!("{speed} MPH");
        let speed = if SMALL.width(&with_unit) <= TEXT_W { with_unit } else { speed };
        pages.push(Page::Wind(clean(&f.now.wind_dir), speed));

        if f.next_12h.len() >= 2 {
            let temps = f.next_12h.iter().map(|h| h.temp_f.unwrap_or(f32::NAN)).collect();
            let rain = f.next_12h.iter().map(|h| h.precip_chance_pct).collect();
            pages.push(Page::Chart(temps, rain));
        }
        if let Some(h) = f.next_12h.iter().find(|h| h.precip_chance_pct >= 50.0) {
            let what = if h.icon == "snow" { "SNOW" } else { "RAIN" };
            // "RAIN 60%"; when that is too wide (100%), it is certain: just "RAIN".
            let chance = format!("{what} {}%", h.precip_chance_pct.round());
            let line = if SMALL.width(&chance) <= TEXT_W { chance } else { what.to_string() };
            pages.push(Page::Text(line, format!("AT {}", clean(&h.time))));
        }

        // The next of sunset (today) or sunrise (tomorrow).
        match today_at(self.now, &f.today.sunset) {
            Some(sunset) if self.now < sunset => pages.push(Page::Text("SUNSET".into(), clean(&f.today.sunset))),
            _ if !f.tomorrow.sunrise.is_empty() => pages.push(Page::Text("SUNRISE".into(), clean(&f.tomorrow.sunrise))),
            _ => {}
        }
        pages.push(Page::Text("TOMORROW".into(), hl(&f.tomorrow)));
        self.pages = pages;
    }

    /// The temperature in big digits, with a degree ring, at the top of the column.
    fn draw_temp(&self, out: &mut Frame) {
        let Some(t) = self.feed.as_ref().and_then(|f| f.now.temp_f) else { return };
        let t = t.round() as i32;
        let text = t.abs().to_string();
        let minus = if t < 0 { 6 } else { 0 };
        let width = minus + text.len() as i32 * 12 - 2 + 5;
        let mut x = TEXT_X + (TEXT_W - width) / 2;
        if t < 0 {
            out.fill(Rect::new(x, 9, x + 5, 11), MAX);
            x += minus;
        }
        for c in text.chars() {
            out.blit(&self.digits[c.to_digit(10).unwrap() as usize], x, 1);
            x += 12;
        }
        // Degree sign: a small ring.
        for (dx, dy) in [(1, 0), (2, 0), (0, 1), (3, 1), (0, 2), (3, 2), (1, 3), (2, 3)] {
            out.set(x - 1 + dx, 2 + dy, MAX);
        }
    }

    fn draw_page(&self, out: &mut Frame, t: f32) {
        if self.pages.is_empty() {
            return;
        }
        let i = (t / PAGE_SECONDS) as usize % self.pages.len();
        let local = t % PAGE_SECONDS;
        let shown = (local * TYPE_SPEED) as usize;
        let alerting = self.feed.as_ref().is_some_and(|f| !f.alerts.is_empty());
        match &self.pages[i] {
            Page::Text(a, b) => {
                let blink_off = alerting && i < self.feed.as_ref().map_or(0, |f| f.alerts.len()) && (t * 3.0) as i32 % 2 == 1;
                // Lines type in (clipped to the column, just in case).
                let column = Rect::new(TEXT_X, 0, TEXT_X + TEXT_W, HEIGHT as i32);
                let line = |out: &mut Frame, text: &str, skip: usize, y: i32, level: u8| {
                    let typed: String = text.chars().take(shown.saturating_sub(skip)).collect();
                    SMALL.draw_clip(out, &typed, TEXT_X, y, level, column);
                };
                if !blink_off {
                    line(out, a, 0, 21, MAX);
                }
                line(out, b, a.chars().count(), 27, 2);
            }
            Page::Wind(dir, speed) => {
                let n = dir.chars().count();
                let typed: String = dir.chars().take(shown).collect();
                SMALL.draw(out, &typed, TEXT_X, 21, MAX);
                if shown >= n {
                    art::wind_arrow(out, TEXT_X + SMALL.width(dir) + 3, 20, dir, MAX);
                }
                let typed: String = speed.chars().take(shown.saturating_sub(n)).collect();
                SMALL.draw(out, &typed, TEXT_X, 27, 2);
            }
            Page::Chart(temps, rain) => {
                SMALL.draw(out, "NEXT 12H", TEXT_X, 21, MAX);
                let valid: Vec<f32> = temps.iter().copied().filter(|v| v.is_finite()).collect();
                let (lo, hi) = valid.iter().fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
                let span = (hi - lo).max(1.0);
                let n = temps.len().min(12);
                // Draw the points up to where the chart has "drawn" so far.
                let upto = ((local / 1.2) * n as f32) as usize;
                let mut prev: Option<(i32, i32)> = None;
                for (k, (&v, &r)) in temps.iter().zip(rain).take(n.min(upto + 1)).enumerate() {
                    let x = TEXT_X + 1 + k as i32 * 3;
                    let bars = (r / 25.0).round() as i32; // rain chance: 0..4 dim dots
                    for b in 0..bars {
                        out.set(x, 31 - b, 1);
                    }
                    if !v.is_finite() {
                        continue;
                    }
                    let y = 31 - ((v - lo) / span * 4.0).round() as i32;
                    if let Some((px, py)) = prev {
                        out.set(px + 1, (py + y) / 2, 2);
                        out.set(px + 2, (py + 2 * y) / 3, 2);
                    }
                    out.set(x, y, MAX);
                    prev = Some((x, y));
                }
            }
        }
    }

    // ---- The sky window ----------------------------------------------

    fn draw_sky(&mut self, dt: f32, t: f32, out: &mut Frame) {
        let icon = self.icon().to_string();
        let day = self.is_day();
        let drift = self.drift();
        let w = ART.x1 as f32;

        // Sky light: day or night, sun or moon.
        let bright = matches!(icon.as_str(), "clear" | "partly_cloudy");
        if !day && bright {
            art::stars(out, Rect::new(0, 0, ART.x1, 26), t, 0.035);
        }
        let (bx, by, r) = if icon == "clear" { (22.0, 15.0, 6.0) } else { (28.0, 8.0, 5.0) };
        if bright {
            if day {
                art::sun(out, bx, by, r, t, ART);
            } else {
                let (lit, waxing) = moon_phase(self.now);
                art::moon(out, bx, by, r, lit, waxing, ART);
            }
        }

        // Clouds: how many, how dark, and whether rain falls from them.
        let (count, fill, edge) = match icon.as_str() {
            "partly_cloudy" => (2, 1, MAX),
            "cloudy" => (4, 1, 2),
            "drizzle" | "rain" | "sleet" | "snow" => (3, 1, 2),
            "thunder" => (4, 1, 1),
            _ => (0, 0, 0),
        };
        for k in 0..count {
            let speed = drift * (0.5 + 0.25 * k as f32);
            let span = w + 30.0;
            let x = (6.0 + k as f32 * 17.0 + speed * t).rem_euclid(span) - 15.0;
            let y = 6.0 + ((k * 7) % 5) as f32 + if icon == "partly_cloudy" { 9.0 } else { 0.0 };
            art::cloud(out, x, y, 1.0 - 0.1 * (k % 2) as f32, fill, edge, ART);
        }

        match icon.as_str() {
            "fog" => self.fog(t, out),
            "drizzle" => self.precipitate(dt, out, 4.0, 14.0, 1, false),
            "rain" => self.precipitate(dt, out, 12.0, 26.0, 2, false),
            "sleet" => self.precipitate(dt, out, 9.0, 18.0, 1, true),
            "snow" => self.snow(dt, t, out),
            "thunder" => {
                self.precipitate(dt, out, 16.0, 30.0, 2, false);
                self.lightning(t, out);
            }
            _ => {}
        }
        // The ground.
        for x in ART.x0..ART.x1 {
            if out.get(x, ART.y1 - 1) == 0 {
                out.set(x, ART.y1 - 1, 1);
            }
        }
    }

    /// Banks of fog sliding past, dithered.
    fn fog(&self, t: f32, out: &mut Frame) {
        let drift = self.drift().abs().max(0.8);
        for y in ART.y0..ART.y1 {
            for x in ART.x0..ART.x1 {
                let (fx, fy) = (x as f32, y as f32);
                let v =
                    0.35 + 0.22 * (fx * 0.13 + t * drift * 0.3 + fy * 0.7).sin() + 0.15 * (fx * 0.31 - t * drift * 0.5 + fy * 1.9).sin();
                art::shade(out, x, y, v * (0.4 + 0.6 * fy / ART.y1 as f32));
            }
        }
    }

    /// Rain, drizzle, or sleet: streaks that fall from the clouds, slanted
    /// by the wind, and splash where they land.
    fn precipitate(&mut self, dt: f32, out: &mut Frame, per_sec: f32, speed: f32, length: i32, sleet: bool) {
        let drift = self.drift();
        let mut spawn = per_sec * 4.0 * dt;
        while spawn > 0.0 {
            if self.rng.next_f32() < spawn {
                let x = self.rng.range(-10.0, ART.x1 as f32 + 10.0);
                self.drops.push(Drop { x, y: self.rng.range(8.0, 12.0), speed: speed * self.rng.range(0.8, 1.2), phase: 0.0 });
            }
            spawn -= 1.0;
        }
        let slant = drift / speed;
        for d in &mut self.drops {
            d.y += d.speed * dt;
            d.x += drift * dt;
        }
        let ground = ART.y1 as f32 - 1.0;
        for d in &self.drops {
            let (x, y) = (d.x.round() as i32, d.y.round() as i32);
            if sleet && (x + y) % 3 == 0 {
                out.set(x.clamp(ART.x0 - 1, ART.x1), y, MAX); // a pellet
                continue;
            }
            for k in 0..length {
                let v = if k == 0 { MAX } else { 2 };
                let (px, py) = ((d.x - slant * k as f32).round() as i32, y - k);
                if ART.contains(px, py) {
                    out.set(px, py, v);
                }
            }
            if d.y >= ground - 0.5 && ART.contains(x, ground as i32 - 1) {
                out.set(x - 1, ground as i32 - 1, 2); // splash
                out.set(x + 1, ground as i32 - 1, 2);
            }
        }
        self.drops.retain(|d| d.y < ground);
    }

    /// Snowflakes: slow, swaying, and they settle on the ground.
    fn snow(&mut self, dt: f32, t: f32, out: &mut Frame) {
        let drift = self.drift();
        for _ in 0..2 {
            if self.rng.next_f32() > 12.0 * dt {
                continue;
            }
            let x = self.rng.range(-8.0, ART.x1 as f32 + 8.0);
            // In this order, and 6.28 rather than TAU: the original's flakes, exactly.
            let (y, speed) = (self.rng.range(8.0, 12.0), self.rng.range(3.0, 6.0));
            #[allow(clippy::approx_constant)]
            let phase = self.rng.range(0.0, 6.28);
            self.drops.push(Drop { x, y, speed, phase });
        }
        for d in &mut self.drops {
            d.y += d.speed * dt;
            d.x += (drift * 0.6 + (t * 2.0 + d.phase).sin() * 2.0) * dt;
        }
        for d in &self.drops {
            out.set(d.x.round() as i32, d.y.round() as i32, if d.speed > 4.5 { MAX } else { 2 });
        }
        self.drops.retain(|d| d.y < ART.y1 as f32 - 1.0);
        // A layer of snow on the ground.
        for x in ART.x0..ART.x1 {
            out.set(x, ART.y1 - 1, 2);
            if hash(x as u32, 77) < 0.5 {
                out.set(x, ART.y1 - 2, 2);
            }
        }
    }

    /// Lightning: now and then a jagged bolt, and the sky flashes.
    fn lightning(&mut self, t: f32, out: &mut Frame) {
        if t >= self.next_strike {
            self.last_strike = t;
            self.next_strike = t + self.rng.range(2.5, 6.0);
            let mut x = self.rng.range(8.0, ART.x1 as f32 - 8.0);
            self.bolt.clear();
            for y in 12..ART.y1 - 1 {
                if y % 3 == 0 {
                    x += self.rng.range(-2.5, 2.5);
                }
                self.bolt.push((x.round() as i32, y));
            }
        }
        let since = t - self.last_strike;
        if since < 0.12 {
            // The flash lights the whole sky.
            for y in ART.y0..ART.y1 {
                for x in ART.x0..ART.x1 {
                    let v = out.get(x, y).max(1);
                    out.set(x, y, v);
                }
            }
        }
        if since < 0.25 || (0.32..0.42).contains(&since) {
            for &(x, y) in &self.bolt {
                out.set(x, y, MAX);
                out.set(x + 1, y, MAX);
            }
        }
    }
}

/// Words for the icon, short enough for the column.
fn condition(icon: &str, day: bool) -> &'static str {
    match icon {
        "clear" if day => "SUNNY",
        "clear" => "CLEAR",
        "partly_cloudy" if day => "PT SUNNY",
        "partly_cloudy" => "PT CLEAR",
        "cloudy" => "CLOUDY",
        "fog" => "FOG",
        "drizzle" => "DRIZZLE",
        "rain" => "RAIN",
        "sleet" => "SLEET",
        "snow" => "SNOW",
        "thunder" => "STORMS",
        _ => "",
    }
}

impl Scene for Weather {
    fn name(&self) -> &str {
        &self.name
    }

    fn enter(&mut self, cx: &Context) {
        self.now = cx.now;
        self.feed = match cx.json::<Feed>(&self.feed_name) {
            Ok(f) => Some(f),
            Err(e) => {
                self.error = self.waiting.clone().unwrap_or_else(|| e.short());
                None
            }
        };
        self.drops.clear();
        self.next_strike = 1.5;
        self.last_strike = -10.0;
        self.build_pages();
    }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        let (dt, t) = (cx.dt, cx.t);
        self.now = cx.now;
        if self.feed.is_none() {
            SMALL.draw_centered(out, "NO WEATHER YET", 10, MAX);
            SMALL.draw_centered(out, &self.error, 18, 2);
            return;
        }
        self.draw_sky(dt, t, out);
        self.draw_temp(out);
        self.draw_page(out, t);
    }

    fn duration(&self) -> f32 {
        (self.pages.len().max(2) as f32 * PAGE_SECONDS).max(12.0)
    }

    fn transition(&self) -> Transition {
        Transition::Dissolve
    }

    fn size(&self) -> Option<Size> {
        Some(crate::SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every line of every page must fit the text column, even for long
    /// values: wide letters (W, M), 3-digit and negative temperatures,
    /// gusts, rain coming, and an alert.
    #[test]
    fn pages_fit_the_column() {
        for icon in ["clear", "partly_cloudy", "cloudy", "fog", "drizzle", "rain", "sleet", "snow", "thunder"] {
            let feed = serde_json::json!({
                "now": {"temp_f": -10, "feels_f": -28, "icon": icon, "is_day": true,
                        "wind_mph": 28, "gust_mph": 44, "wind_dir": "WNW"},
                "today": {"high_f": 100, "low_f": -10, "sunset": "11:59 PM"},
                "tomorrow": {"high_f": -10, "low_f": -20, "sunrise": "12:00 PM"},
                "next_12h": [{"time": "12 PM", "temp_f": 1, "icon": "snow", "precip_chance_pct": 100},
                             {"time": "1 PM", "temp_f": 2, "icon": "rain", "precip_chance_pct": 90}],
                "alerts": [{"event": "Severe Thunderstorm Warning"}]
            });
            let mut w = Weather {
                name: String::new(),
                feed_name: String::new(),
                waiting: None,
                error: String::new(),
                now: glowdots::time::parse_local("2026-10-09 12:00").unwrap(),
                digits: Vec::new(),
                feed: Some(serde_json::from_value(feed).unwrap()),
                pages: Vec::new(),
                drops: Vec::new(),
                rng: Rng(1),
                next_strike: 0.0,
                last_strike: 0.0,
                bolt: Vec::new(),
            };
            w.build_pages();
            for page in &w.pages {
                let lines = match page {
                    Page::Text(a, b) => vec![a.clone(), b.clone()],
                    Page::Wind(a, b) => {
                        // The direction, a gap, and the 7-dot arrow.
                        assert!(SMALL.width(a) + 3 + 7 <= TEXT_W);
                        vec![b.clone()]
                    }
                    Page::Chart(..) => vec!["NEXT 12H".into()],
                };
                for line in lines {
                    assert!(SMALL.width(&line) <= TEXT_W, "{icon}: {line:?} is {} dots", SMALL.width(&line));
                }
            }
        }
    }
}
