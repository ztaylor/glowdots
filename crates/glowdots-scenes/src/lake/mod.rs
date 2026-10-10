//! A lake, live, with the fishing facts.
//!
//! A full-width panorama: the moon at its real phase (or the sun on its
//! arc from sunrise to sunset), stars, a dawn or dusk glow, clouds when
//! the weather says so, the treeline on the far shore, and the water.
//! Ripples move with the current wind, the moon or sun shimmers on the
//! water, a bobber bobs and rings the surface, and now and then a fish
//! jumps. Once a visit, a duck flies in, lands, and paddles away.
//!
//! The facts type out in the sky, a page at a time: the dawn and dusk
//! windows, the pressure trend, and the moon.
//!
//! When the feed says the lake is iced over, it is winter instead (see
//! `winter.rs`).
//!
//! It reads `lake.json` (see `samples/lake.json`): dawn and dusk windows,
//! the pressure trend, the moon, sun times, and whether the lake is iced
//! over. It also reads `weather.json` for the wind and clouds right now.
//! A scene like this shows how to build a living picture from a few
//! numbers: every layer is drawn dot by dot from the time and the data.

mod winter;

use chrono::{DateTime, Local};
use glowdots::art;
use glowdots::font::{SMALL, clean};
use glowdots::math::{Rng, hash};
use glowdots::time::{moon_phase, today_at};
use glowdots::{Context, Frame, MAX, Rect, Scene, Size, Sprite, Transition};
use serde::Deserialize;
use std::f32::consts::{PI, TAU};

const W: i32 = crate::SIZE.w as i32;
const H: i32 = crate::SIZE.h as i32;
/// The far shore meets the water here.
const HORIZON: i32 = 15;
const PAGE_SECONDS: f32 = 4.0;
const TYPE_SPEED: f32 = 50.0;
/// The bobber, and the rod tip it is tied to (off-screen, by the viewer).
const BOBBER: (i32, i32) = (30, 23);
const ROD_TIP: (f32, f32) = (82.0, 33.0);
/// The duck: when it comes, where it flies in from, where it lands, and how
/// fast it paddles away (to the left) after.
const DUCK_START: f32 = 8.0;
const DUCK_GLIDE: f32 = 2.4;
const DUCK_FROM: (f32, f32) = (86.0, 12.0);
const DUCK_LAND: (f32, f32) = (52.0, 19.0);
const DUCK_PADDLE: f32 = 1.2;
/// No fish jumps from a little before the duck comes until it has settled.
const DUCK_QUIET: (f32, f32) = (DUCK_START - 1.5, DUCK_START + DUCK_GLIDE + 2.5);

/// Page text stays in x = TEXT_X..TEXT_X + TEXT_W; the sun and moon stay to
/// the right of that, so text never covers them.
const TEXT_X: i32 = 2;
const TEXT_W: i32 = 54;

/// The moon hangs here at night; its reflection runs below it. Page text
/// stays left of it.
const MOON_AT: (f32, f32) = (72.0, 6.0);

#[derive(Deserialize, Default)]
#[serde(default)]
struct Feed {
    /// The lake has iced over (set it at ice-in, clear it at ice-out).
    ice: bool,
    /// YYYY-MM-DD, when it iced over.
    ice_since: String,
    pressure: Pressure,
    windows: Vec<Window>,
    moon: Moon,
    sunrise_today: String,
    sunset_today: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Pressure {
    now_inhg: Option<f32>,
    change_past_6h_inhg: Option<f32>,
    trend_past_6h: String,
    change_next_6h_inhg: Option<f32>,
    trend_next_6h: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Window {
    kind: String,
    day: String,
    start: String,
    wind_mph: Option<f32>,
    gust_mph: Option<f32>,
    wind_dir: String,
    cloud_pct: Option<f32>,
    temp_f: Option<f32>,
    precip_chance_pct: Option<f32>,
    icon: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Moon {
    phase: String,
    illum_pct: Option<f32>,
    days_to_full: Option<f32>,
    days_to_new: Option<f32>,
}

/// The weather right now, from the weather feed: for wind and clouds.
#[derive(Deserialize, Default)]
#[serde(default)]
struct WeatherNow {
    now: NowPart,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct NowPart {
    temp_f: Option<f32>,
    feels_f: Option<f32>,
    icon: String,
    wind_mph: Option<f32>,
    wind_dir: String,
}

struct Page {
    lines: [String; 2],
    /// Draw a wind arrow after the first line, for this compass direction.
    arrow: Option<String>,
}

struct Jump {
    start: f32,
    x: f32,
    y: f32,
    dir: f32,
}

pub struct Lake {
    name: String,
    title: String,
    feed_name: String,
    weather_name: String,
    waiting: Option<String>,
    error: String,
    now: DateTime<Local>,
    feed: Option<Feed>,
    weather: WeatherNow,
    pages: Vec<Page>,
    jump: Option<Jump>,
    next_jump: f32,
    drops: Vec<(f32, f32, f32, f32)>,
    rng: Rng,
    duck_landed: bool,
    /// Duck sprites, facing left: wings up, wings down, swimming.
    duck: [Sprite; 3],
}

impl Lake {
    pub fn new() -> Self {
        Self {
            name: "lake".into(),
            title: "THE LAKE".into(),
            feed_name: "lake.json".into(),
            weather_name: "weather.json".into(),
            waiting: None,
            error: String::new(),
            now: Local::now(),
            feed: None,
            weather: WeatherNow::default(),
            pages: Vec::new(),
            jump: None,
            next_jump: 3.0,
            drops: Vec::new(),
            rng: Rng(0x0f15_4a7e),
            duck_landed: false,
            duck: [Sprite::from_rows(&DUCK_UP), Sprite::from_rows(&DUCK_DOWN), Sprite::from_rows(&DUCK_SWIM)],
        }
    }

    /// Call the scene something else (for logs and choosing it).
    pub fn named(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    /// The lake's name, on the first page ("CRYSTAL LAKE").
    pub fn title(mut self, title: &str) -> Self {
        self.title = clean(title);
        self
    }

    /// Read other feeds than `lake.json` and `weather.json`.
    pub fn feeds(mut self, lake: &str, weather: &str) -> Self {
        self.feed_name = lake.into();
        self.weather_name = weather.into();
        self
    }

    /// What to say under the title when the feed is missing or bad. By
    /// default it says what is wrong ("NO FEED YET", "BAD FEED: LINE 3").
    pub fn waiting(mut self, text: &str) -> Self {
        self.waiting = Some(text.into());
        self
    }

    fn build_pages(&mut self) {
        let Some(f) = &self.feed else { return };
        let num = |v: Option<f32>| v.map_or("--".into(), |v| (v.round() as i32).to_string());
        let mut pages = Vec::new();

        if f.ice {
            pages.extend(self.winter_pages(f));
        } else if let Some(t) = self.weather.now.temp_f {
            pages.push(page(&self.title, &fit(&[format!("REPORT · {}° NOW", t.round() as i32), format!("{}° NOW", t.round() as i32)])));
        } else {
            pages.push(page(&self.title, "REPORT"));
        }
        for w in f.windows.iter().filter(|_| !f.ice) {
            let kind = clean(&w.kind);
            // Dawn is AM and dusk PM, so the AM/PM can go when there is no room.
            let start = clean(&w.start);
            let short = start.trim_end_matches(" AM").trim_end_matches(" PM").to_string();
            let today = w.day.eq_ignore_ascii_case("today") || w.day.is_empty();
            let day = if today { String::new() } else { "TMRW ".into() };
            let sky = format!("{}° {}", num(w.temp_f), sky_word(&w.icon));
            // Last resort: "TMRW" moves down to the second line.
            let (when, sky) = match fit(&[format!("{kind} {day}{start}"), format!("{kind} {day}{short}"), String::new()]) {
                w if !w.is_empty() => (w, sky),
                _ => (format!("{kind} {short}"), fit(&[format!("{day}{sky}"), sky.clone()])),
            };
            pages.push(page(&when, &sky));
            let gust = match (w.wind_mph, w.gust_mph) {
                (Some(a), Some(g)) if g >= a + 4.0 => format!(" GUST {}", g.round() as i32),
                _ => String::new(),
            };
            let speed = fit(&[format!("{} MPH{gust}", num(w.wind_mph)), format!("{}{gust}", num(w.wind_mph))]);
            pages.push(Page { lines: [format!("WIND {}", clean(&w.wind_dir)), speed], arrow: Some(w.wind_dir.clone()) });
            pages.push(page(&format!("CLOUD {}%", num(w.cloud_pct)), &format!("RAIN {}%", num(w.precip_chance_pct))));
        }
        let p = &f.pressure;
        if let Some(inhg) = p.now_inhg {
            let change = |c: Option<f32>| {
                c.map_or(String::new(), |c| format!(" {}{:.2}", if c >= 0.0 { "+" } else { "-" }, c.abs()).replace("0.", "."))
            };
            let trend = |words: &str, c: Option<f32>| fit(&[format!("{}{}", clean(words), change(c)), clean(words)]);
            pages.push(page(&format!("PRESSURE {inhg:.2}"), &trend(&p.trend_past_6h, p.change_past_6h_inhg)));
            if !p.trend_next_6h.is_empty() {
                pages.push(page("NEXT 6 HOURS", &trend(&p.trend_next_6h, p.change_next_6h_inhg)));
            }
        }
        let m = &f.moon;
        if let Some(lit) = m.illum_pct {
            let lit = lit.round() as i32;
            // "WANING CRESCENT" is too wide for one line: split the phase.
            let phase = clean(&m.phase);
            let (a, b) = match phase.split_once(' ') {
                Some((first, "MOON")) => (format!("{first} MOON"), format!("{lit}% LIT")),
                Some((first, rest)) => (first.to_string(), format!("{rest} {lit}%")),
                None => ("MOON".to_string(), fit(&[format!("{phase} {lit}%"), format!("{lit}% LIT")])),
            };
            pages.push(page(&a, &b));
            let days = |d: f32| match d.round() as i32 {
                1 => "TOMORROW".to_string(),
                d => format!("IN {d} DAYS"),
            };
            match (m.days_to_new, m.days_to_full) {
                (Some(n), Some(full)) if n <= full && n >= 1.0 => pages.push(page("NEW MOON", &days(n))),
                (_, Some(full)) if full >= 1.0 => pages.push(page("FULL MOON", &days(full))),
                _ => {}
            }
        }
        self.pages = pages;
    }

    /// Where the light in the sky is: (x, y, is it the sun?).
    fn light(&self) -> (f32, f32, bool) {
        let f = self.feed.as_ref();
        let rise = f.and_then(|f| today_at(self.now, &f.sunrise_today));
        let set = f.and_then(|f| today_at(self.now, &f.sunset_today));
        if let (Some(rise), Some(set)) = (rise, set) {
            let n = self.now;
            if n > rise && n < set {
                let p = (n - rise).num_seconds() as f32 / (set - rise).num_seconds() as f32;
                // Its arc stays in the right of the sky, clear of the text.
                return (64.0 + p * 12.0, 11.0 - (PI * p).sin() * 6.0, true);
            }
        }
        (MOON_AT.0, MOON_AT.1, false)
    }

    /// Minutes to the nearer of sunrise and sunset (for the horizon glow).
    fn twilight(&self) -> Option<f32> {
        let f = self.feed.as_ref()?;
        let n = self.now;
        [today_at(n, &f.sunrise_today), today_at(n, &f.sunset_today)]
            .into_iter()
            .flatten()
            .map(|t| (t - n).num_minutes().abs() as f32)
            .reduce(f32::min)
    }

    /// The moon from the feed (lit fraction, waxing?), or worked out from
    /// the date if the feed has no moon.
    fn moon_lit(&self) -> (f32, bool) {
        match &self.feed {
            Some(f) if f.moon.illum_pct.is_some() => (
                f.moon.illum_pct.unwrap_or(0.0) / 100.0,
                if f.moon.phase.is_empty() { moon_phase(self.now).1 } else { !f.moon.phase.to_lowercase().contains("waning") },
            ),
            _ => moon_phase(self.now),
        }
    }

    fn wind(&self) -> (f32, f32) {
        let mph = self.weather.now.wind_mph.or_else(|| self.feed.as_ref()?.windows.first()?.wind_mph).unwrap_or(4.0);
        let deg = art::compass(&self.weather.now.wind_dir).unwrap_or(270.0);
        (mph, -deg.to_radians().sin())
    }

    // ---- The picture -------------------------------------------------

    fn sky(&self, t: f32, out: &mut Frame) {
        let (lx, ly, is_sun) = self.light();
        // A glow along the horizon near sunrise and sunset, strongest
        // nearest the sun.
        if let Some(mins) = self.twilight().filter(|m| *m < 50.0) {
            let strength = 1.0 - mins / 50.0;
            for y in 4..HORIZON {
                for x in 0..W {
                    let near_sun = 1.0 - ((x as f32 - lx).abs() / 60.0).min(1.0);
                    let v = strength * (y - 4) as f32 / (HORIZON - 4) as f32 * (0.25 + 0.35 * near_sun);
                    art::shade(out, x, y, v);
                }
            }
        }
        if !is_sun {
            art::stars(out, Rect::new(0, 0, W, HORIZON - 3), t, 0.03);
        }
        let sky = Rect::new(0, 0, W, HORIZON);
        if is_sun {
            art::sun(out, lx, ly, 3.0, t, sky);
        } else {
            let (lit, waxing) = self.moon_lit();
            art::moon(out, lx, ly, 4.5, lit, waxing, sky);
        }
        // Clouds when the weather has them.
        let count = match self.weather.now.icon.as_str() {
            "partly_cloudy" => 2,
            "cloudy" | "drizzle" | "rain" | "thunder" | "snow" | "sleet" => 4,
            _ => 0,
        };
        let (mph, dir) = self.wind();
        for k in 0..count {
            let x = (10.0 + k as f32 * 23.0 + t * dir * (0.4 + mph * 0.08)).rem_euclid(W as f32 + 24.0) - 12.0;
            art::cloud(out, x, 4.0 + (k % 2) as f32 * 2.0, 0.6, 1, 2, sky);
        }
    }

    /// The far shore: a row of pines, dim.
    fn treeline(&self, out: &mut Frame) {
        for x in 0..W {
            out.set(x, HORIZON - 1, 1);
            out.set(x, HORIZON, 1);
        }
        let mut x = -2;
        while x < W + 2 {
            // Mostly low pines, now and then a taller one.
            let h = 1 + (hash(x as u32, 5) * 3.0) as i32 + if hash(x as u32, 6) > 0.8 { 1 } else { 0 };
            for row in 0..h {
                let half = row / 2;
                for dx in -half..=half {
                    out.set(x + dx, HORIZON - 1 - h + row, 1);
                }
            }
            x += 3 + (hash(x as u32, 9) * 3.0) as i32;
        }
    }

    fn water(&self, t: f32, out: &mut Frame) {
        let (mph, dir) = self.wind();
        // Calm water has a few long, slow ripple lines; wind makes many,
        // short, quick ones.
        let busy = (mph / 15.0).clamp(0.15, 1.0);
        for y in HORIZON + 1..H {
            let depth = (y - HORIZON) as f32 / (H - HORIZON) as f32; // 0 far .. 1 near
            let freq = 0.9 - depth * 0.6;
            let speed = dir * (1.0 + mph * 0.25) * (0.5 + depth);
            for x in 0..W {
                let v = (x as f32 * freq + t * speed + y as f32 * 2.3).sin() * (y as f32 * 0.7 + t * 0.6).cos();
                if v > 1.0 - busy * 0.55 {
                    out.set(x, y, 1);
                }
            }
        }
        // The light's reflection: a broken, shimmering column that widens
        // toward you. A thin moon gives a faint, sparse one.
        let (lx, _, is_sun) = self.light();
        let strength = if is_sun { 1.0 } else { self.moon_lit().0.clamp(0.1, 1.0) };
        for y in HORIZON + 1..H {
            let half = 0.5 + (y - HORIZON) as f32 * (0.08 + 0.12 * strength);
            for x in (lx - half).round() as i32..=(lx + half).round() as i32 {
                let flicker = hash(x as u32 + 1000, y as u32 * 31 + (t * 7.0) as u32);
                if flicker > 1.0 - 0.45 * strength.max(0.4) {
                    let bright = is_sun || flicker > 1.0 - 0.15 * strength;
                    out.set(
                        x,
                        y,
                        if bright {
                            MAX
                        } else if strength > 0.4 {
                            2
                        } else {
                            1
                        },
                    );
                }
            }
        }
    }

    fn bobber(&self, t: f32, out: &mut Frame) {
        let (bx, by) = BOBBER;
        // Rings spread from the bobber every few seconds.
        let age = t % 2.6;
        if age < 2.0 {
            let r = age * 3.0;
            let n = (12.0 + r * 10.0) as i32;
            for k in 0..n {
                let a = TAU * k as f32 / n as f32;
                let (x, y) = (bx as f32 + a.cos() * (1.5 + r * 2.2), by as f32 + 0.5 + a.sin() * (0.4 + r * 0.45));
                out.set(x.round() as i32, y.round() as i32, if age < 1.0 { 2 } else { 1 });
            }
        }
        // The line runs back to the rod, sagging a little.
        let (sx, sy) = (bx as f32, by as f32 - 1.0);
        let (cx, cy) = (56.0, 33.0);
        for k in 1..60 {
            let u = k as f32 / 60.0;
            let x = (1.0 - u) * (1.0 - u) * sx + 2.0 * (1.0 - u) * u * cx + u * u * ROD_TIP.0;
            let y = (1.0 - u) * (1.0 - u) * sy + 2.0 * (1.0 - u) * u * cy + u * u * ROD_TIP.1;
            if out.get(x.round() as i32, y.round() as i32) == 0 {
                out.set(x.round() as i32, y.round() as i32, 1);
            }
        }
        let bob = if (t * 1.7).sin() > 0.55 { 1 } else { 0 };
        out.set(bx, by - 1 + bob, MAX);
        out.set(bx, by + bob, 2);
    }

    /// Now and then a fish jumps: an arc out of the water, and splashes.
    fn fish(&mut self, dt: f32, t: f32, out: &mut Frame) {
        let quiet = (DUCK_QUIET.0..DUCK_QUIET.1).contains(&t);
        if self.jump.is_none() && t >= self.next_jump && !quiet {
            let mut x = self.rng.range(6.0, W as f32 - 10.0);
            if (x - BOBBER.0 as f32).abs() < 8.0 {
                x += 16.0;
            }
            if let Some((dx, _, _)) = duck_at(t) {
                if (x - dx).abs() < 10.0 {
                    x = (x + 24.0).rem_euclid(W as f32 - 16.0) + 6.0;
                }
            }
            let dir = if self.rng.next_f32() < 0.5 { -1.0 } else { 1.0 };
            let y = self.rng.range(19.0, 27.0);
            self.jump = Some(Jump { start: t, x, y, dir });
            self.splash(x, y);
        }
        if let Some(j) = &self.jump {
            let u = (t - j.start) / 0.9;
            if u >= 1.0 {
                let (x, y) = (j.x + j.dir * 7.0, j.y);
                self.jump = None;
                self.next_jump = t + self.rng.range(5.0, 10.0);
                self.splash(x, y);
            } else {
                let x = j.x + j.dir * 7.0 * u;
                let y = j.y - (PI * u).sin() * 6.0;
                // A fish: a short bright body angled along its arc.
                let tilt = if u < 0.5 { -1 } else { 1 };
                out.set(x.round() as i32, y.round() as i32, MAX);
                out.set((x - j.dir).round() as i32, y.round() as i32 - tilt, MAX);
                out.set((x - 2.0 * j.dir).round() as i32, y.round() as i32 - tilt, 2);
            }
        }
        for d in &mut self.drops {
            d.0 += d.2 * dt;
            d.1 += d.3 * dt;
            d.3 += 30.0 * dt;
        }
        self.drops.retain(|d| d.1 < 31.0 && d.3 < 20.0);
        for d in &self.drops {
            out.set(d.0.round() as i32, d.1.round() as i32, 2);
        }
    }

    /// The duck: flaps in, glides down, flares, splashes down, and paddles
    /// off with a V of wake behind it.
    fn duck(&mut self, t: f32, out: &mut Frame) {
        let Some((x, y, flying)) = duck_at(t) else { return };
        if flying {
            let u = (t - DUCK_START) / DUCK_GLIDE;
            // Flapping, then wings held up as it flares to land.
            let k = if u > 0.7 { 0 } else { (t * 8.0) as usize % 2 };
            let s = &self.duck[k];
            out.blit(s, x.round() as i32, y.round() as i32 - 2);
            return;
        }
        if !self.duck_landed {
            self.duck_landed = true;
            self.splash(DUCK_LAND.0 + 2.0, DUCK_LAND.1);
        }
        let since = t - DUCK_START - DUCK_GLIDE;
        let (xi, yi) = (x.round() as i32, y.round() as i32);
        // A ring spreads from where it landed.
        if since < 1.6 {
            let r = since * 4.0;
            let n = (10.0 + r * 8.0) as i32;
            for k in 0..n {
                let a = TAU * k as f32 / n as f32;
                let (rx, ry) = (DUCK_LAND.0 + 2.0 + a.cos() * (2.0 + r * 2.0), DUCK_LAND.1 + 1.0 + a.sin() * (0.5 + r * 0.4));
                out.set(rx.round() as i32, ry.round() as i32, if since < 0.8 { 2 } else { 1 });
            }
        }
        // The wake: two lines spreading back from its tail, fading.
        let length = ((since * 4.0) as i32).min(10);
        for k in 1..=length {
            let (wx, level) = (xi + 7 + k, if k < 5 { 2 } else { 1 });
            out.set(wx, yi + 1 - k / 3, level);
            out.set(wx, yi + 1 + k / 3, level);
        }
        // The bottom of the body sits on the waterline; the reflection below.
        blit_between(out, &self.duck[2], x, yi - 3);
    }

    fn splash(&mut self, x: f32, y: f32) {
        for _ in 0..6 {
            let vx = self.rng.range(-6.0, 6.0);
            let vy = self.rng.range(-12.0, -6.0);
            self.drops.push((x, y, vx, vy));
        }
    }

    fn page(&self, t: f32, out: &mut Frame) {
        if self.pages.is_empty() {
            return;
        }
        let page = &self.pages[(t / PAGE_SECONDS) as usize % self.pages.len()];
        let shown = ((t % PAGE_SECONDS) * TYPE_SPEED) as usize;
        let mut layer = out.blank();
        let first: String = page.lines[0].chars().take(shown).collect();
        let second: String = page.lines[1].chars().take(shown.saturating_sub(page.lines[0].chars().count())).collect();
        // Clipped to the text area, just in case, so the sky is never covered.
        let area = Rect::new(TEXT_X, 0, TEXT_X + TEXT_W, HORIZON);
        SMALL.draw_clip(&mut layer, &first, TEXT_X, 1, MAX, area);
        SMALL.draw_clip(&mut layer, &second, TEXT_X, 7, 2, area);
        if let Some(dir) = &page.arrow {
            if first.len() == page.lines[0].len() {
                art::wind_arrow(&mut layer, TEXT_X + SMALL.width(&first) + 4, 0, dir, MAX);
            }
        }
        art::overlay_outlined(out, &layer);
    }
}

/// Draw a sprite at a fractional x, so slow movement is smooth: each dot's
/// brightness is split between the two dots it falls between, in thirds.
fn blit_between(out: &mut Frame, s: &Sprite, x: f32, y: i32) {
    let x0 = x.floor();
    let f = ((x - x0) * 3.0).round() / 3.0;
    let mut row = vec![0.0f32; s.w + 1];
    for sy in 0..s.h {
        row.fill(0.0);
        for sx in 0..s.w {
            let v = s.dots[sy * s.w + sx] as f32;
            row[sx] += v * (1.0 - f);
            row[sx + 1] += v * f;
        }
        for (k, &v) in row.iter().enumerate() {
            let level = v.round().min(MAX as f32) as u8;
            if level > 0 {
                out.set(x0 as i32 + k as i32, y + sy as i32, level);
            }
        }
    }
}

/// Where the duck is at time t: (x, y of its waterline, flying?).
fn duck_at(t: f32) -> Option<(f32, f32, bool)> {
    if t < DUCK_START {
        return None;
    }
    let u = (t - DUCK_START) / DUCK_GLIDE;
    if u < 1.0 {
        let x = DUCK_FROM.0 + (DUCK_LAND.0 - DUCK_FROM.0) * u;
        // It drops slowly at first, then faster as it comes down.
        let y = DUCK_FROM.1 + (DUCK_LAND.1 - DUCK_FROM.1) * u * u;
        return Some((x, y, true));
    }
    let x = DUCK_LAND.0 - (t - DUCK_START - DUCK_GLIDE) * DUCK_PADDLE;
    (x > -6.0).then_some((x, DUCK_LAND.1, false))
}

// The duck, facing left: '#' bright, '=' mid, ' ' off.
const DUCK_UP: [&str; 4] = ["   #  ", "#  #  ", "######", "      "];
const DUCK_DOWN: [&str; 4] = ["      ", "#     ", "######", "  #   "];
// Swimming, the same size as in flight: head, dim bill, body with a
// tipped-up tail, and a faint reflection under the waterline (last row).
const DUCK_SWIM: [&str; 5] = [" ##    ", "=##   #", " ######", "  ==== ", "   ... "];

/// A page with no wind arrow.
fn page(a: &str, b: &str) -> Page {
    Page { lines: [a.to_string(), b.to_string()], arrow: None }
}

/// The first of the options that fits the text area (the last, if none do).
fn fit(options: &[String]) -> String {
    options.iter().find(|o| SMALL.width(o) <= TEXT_W).unwrap_or(&options[options.len() - 1]).clone()
}

/// A short word for the sky in a window.
fn sky_word(icon: &str) -> &'static str {
    match icon {
        "clear" => "CLEAR",
        "partly_cloudy" => "PT CLOUDY",
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

impl Scene for Lake {
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
        self.weather = cx.json(&self.weather_name).unwrap_or_default();
        self.jump = None;
        self.next_jump = 3.0;
        self.drops.clear();
        self.duck_landed = false;
        self.build_pages();
    }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        let (dt, t) = (cx.dt, cx.t);
        self.now = cx.now;
        if self.feed.as_ref().is_some_and(|f| f.ice) {
            self.draw_winter(t, out);
        } else {
            self.sky(t, out);
            self.treeline(out);
            self.water(t, out);
            self.bobber(t, out);
            self.fish(dt, t, out);
            self.duck(t, out);
        }
        if self.feed.is_none() {
            let mut layer = out.blank();
            SMALL.draw(&mut layer, &self.title, 2, 1, MAX);
            SMALL.draw(&mut layer, &self.error, 2, 7, 2);
            art::overlay_outlined(out, &layer);
        } else {
            self.page(t, out);
        }
    }

    fn duration(&self) -> f32 {
        (self.pages.len().max(3) as f32 * PAGE_SECONDS).max(16.0)
    }

    fn transition(&self) -> Transition {
        Transition::Wipe
    }

    fn size(&self) -> Option<Size> {
        Some(crate::SIZE)
    }
}

impl Default for Lake {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wind arrow after a line: a gap, then 7 dots.
    const ARROW_W: i32 = 4 + 7;

    /// Every page line (with its wind arrow) must stay left of the sun and
    /// moon, in summer and winter, even for long values.
    #[test]
    fn pages_stay_clear_of_the_sky() {
        let window = |kind: &str, day: &str, start: &str| {
            serde_json::json!({
                "kind": kind, "day": day, "start": start, "wind_mph": 28, "gust_mph": 44, "wind_dir": "WNW",
                "cloud_pct": 100, "temp_f": -10, "precip_chance_pct": 100, "icon": "partly_cloudy"
            })
        };
        for ice in [false, true] {
            for phase in ["Waning crescent", "Waxing gibbous", "First quarter", "Full moon", "New moon"] {
                let feed = serde_json::json!({
                    "ice": ice, "ice_since": "2026-12-13",
                    "pressure": {"now_inhg": 29.79, "change_past_6h_inhg": -0.12, "trend_past_6h": "falling fast",
                                 "change_next_6h_inhg": 0.09, "trend_next_6h": "rising slowly"},
                    "windows": [window("Dawn", "Tomorrow", "8:48 AM"), window("Dusk", "Tomorrow", "9:11 PM")],
                    "moon": {"phase": phase, "illum_pct": 100, "days_to_full": 19, "days_to_new": 14}
                });
                let mut lake = Lake::new().title("CRYSTAL LAKE");
                lake.feed = Some(serde_json::from_value(feed).unwrap());
                lake.weather = serde_json::from_value(serde_json::json!({
                    "now": {"temp_f": -10, "feels_f": -28, "icon": "snow", "wind_mph": 28, "wind_dir": "WNW"}
                }))
                .unwrap();
                lake.build_pages();
                for p in &lake.pages {
                    let arrow = if p.arrow.is_some() { ARROW_W } else { 0 };
                    assert!(SMALL.width(&p.lines[0]) + arrow <= TEXT_W, "{:?} too wide", p.lines[0]);
                    assert!(SMALL.width(&p.lines[1]) <= TEXT_W, "{:?} too wide", p.lines[1]);
                }
            }
        }
    }
}
