//! The lake in winter: iced over and under snow.
//!
//! Snow on the ice with a couple of pressure cracks, and a soft glow from
//! the moon or sun. Snow blows across it with the real wind (and falls,
//! when it is snowing). An ice-fishing hole with a tip-up whose flag springs
//! up for a bite; a fish house on the far ice with a lit window and chimney
//! smoke; and once a visit, a crow that walks across and stops to peck.

use super::{BOBBER, Feed, H, HORIZON, Lake, Page, W, blit_between, fit, page};
use chrono::NaiveDate;
use glowdots::art;
use glowdots::font::clean;
use glowdots::math::hash;
use glowdots::{Frame, MAX, Sprite};

/// The tip-up's flag springs up for a bite this often, and stays up a while.
const BITE_FIRST: f32 = 5.0;
const BITE_EVERY: f32 = 11.0;
const FLAG_UP: f32 = 3.5;
/// The fish house: top-left corner, on the far ice.
const SHACK: (i32, i32) = (56, 13);
/// The crow: when it comes, how fast it walks, where it stops to peck, and
/// the row its feet are on.
const CROW_START: f32 = 9.0;
const CROW_SPEED: f32 = 3.0;
const CROW_PECK_AT: f32 = 46.0;
const CROW_PECK: f32 = 2.5;
const CROW_FEET: i32 = 29;

// '#' bright, '=' mid, '.' dim, ' ' off.
/// The fish house, drawn as an outline (a solid shape just glows into a
/// blob): peaked roof, chimney, dim walls. Its window (row 5, column 4) is
/// drawn on top and flickers.
const SHACK_ART: [&str; 7] = ["    #  # ", "   # # # ", "  #   ## ", "#########", " =     = ", " =     = ", " ======= "];
/// The crow, facing left: walking (two leg frames) and pecking.
const CROW: [[&str; 5]; 3] = [
    [" ##     ", "=###    ", " #######", "  ####  ", "  #  #  "],
    [" ##     ", "=###    ", " #######", "  ####  ", "   ##   "],
    ["        ", "   #### ", "  ######", " ####   ", "=#  #   "],
];

impl Lake {
    pub(super) fn draw_winter(&self, t: f32, out: &mut Frame) {
        self.sky(t, out);
        self.treeline(out);
        self.ice(t, out);
        self.shack(t, out);
        self.tip_up(t, out);
        self.crow(t, out);
        self.snow(t, out);
    }

    /// Snow on the ice: mostly dark, with sparse glints that sparkle now
    /// and then, a soft glow from the moon or sun, and two pressure cracks.
    /// Snow is brighter by day.
    fn ice(&self, t: f32, out: &mut Frame) {
        let (lx, _, is_sun) = self.light();
        let glow = if is_sun { 0.3 } else { 0.2 * self.moon_lit().0.max(0.25) };
        let density = if is_sun { 0.14 } else { 0.07 };
        for y in HORIZON + 1..H {
            for x in 0..W {
                let (fx, fy) = (x as f32, y as f32);
                let d = ((fx - lx) / 14.0).hypot((fy - HORIZON as f32 - 3.0) / 4.0);
                if d < 1.0 {
                    art::shade(out, x, y, glow * (1.0 - d));
                }
                let h = hash(x as u32 + 500, y as u32);
                if h < density {
                    let glint = hash(x as u32, y as u32 * 7 + (t * 2.0 + h * 40.0) as u32) > 0.93;
                    out.set(x, y, if glint { MAX } else { 1 });
                }
            }
        }
        // Pressure cracks: jagged dim lines across the ice.
        for (start, end, y0) in [(4, 24, 28.0f32), (42, 54, 19.0)] {
            let mut y = y0;
            for x in start..end {
                y += (hash(x as u32, y0 as u32) - 0.5) * 1.4;
                y = y.clamp(HORIZON as f32 + 2.0, H as f32 - 1.0);
                out.set(x, y.round() as i32, 1);
            }
        }
    }

    /// A fish house with a flickering lit window, and smoke from its
    /// chimney that drifts with the wind.
    fn shack(&self, t: f32, out: &mut Frame) {
        let (x0, y0) = SHACK;
        out.blit(&Sprite::from_rows(&SHACK_ART), x0, y0);
        let window = if hash((t * 4.0) as u32, 3) > 0.15 { MAX } else { 2 };
        out.set(x0 + 4, y0 + 5, window);
        // Puffs of smoke: born every 0.45 s, rising and drifting for 3 s.
        let (mph, dir) = self.wind();
        let drift = dir * (0.5 + mph * 0.35);
        let (cx, cy) = (x0 as f32 + 7.0, y0 as f32 - 1.0);
        let newest = (t / 0.45) as i32;
        for i in (newest - 7).max(0)..=newest {
            let age = t - i as f32 * 0.45;
            if !(0.0..3.0).contains(&age) {
                continue;
            }
            let x = cx + drift * age + (age * 3.0 + i as f32).sin() * 0.6;
            let y = cy - age * 2.4;
            if y >= 0.0 {
                out.set(x.round() as i32, y.round() as i32, if age < 1.2 { 2 } else { 1 });
            }
        }
    }

    /// An ice hole with a tip-up across it. For a bite, the flag springs up
    /// on its pole and waves, and the water in the hole stirs.
    fn tip_up(&self, t: f32, out: &mut Frame) {
        let (hx, hy) = BOBBER;
        // The hole: dark water inside a bright rim of snow.
        for y in hy - 2..=hy + 2 {
            for x in hx - 5..=hx + 5 {
                let d = ((x - hx) as f32 / 3.5).hypot((y - hy) as f32 / 1.5);
                if d <= 0.75 {
                    out.set(x, y, 0);
                } else if d <= 1.15 {
                    out.set(x, y, 2);
                }
            }
        }
        let cycle = (t - BITE_FIRST).rem_euclid(BITE_EVERY);
        let biting = t >= BITE_FIRST && cycle < FLAG_UP;
        if biting && hash((t * 8.0) as u32, 9) > 0.5 {
            out.set(hx - 1 + (hash((t * 8.0) as u32, 1) * 3.0) as i32, hy, 1);
        }
        // The cross stick lying over the hole.
        for x in hx - 6..=hx + 6 {
            out.set(x, hy, 2);
        }
        if !biting {
            // The flag folded down along the stick.
            out.set(hx + 1, hy - 1, MAX);
            out.set(hx + 2, hy - 1, MAX);
            return;
        }
        // The pole springs up, then the flag waves at its top.
        let height = ((cycle / 0.2).min(1.0) * 6.0).round() as i32;
        for k in 1..=height {
            out.set(hx, hy - k, 2);
        }
        if height == 6 {
            let top = hy - 6;
            let wide = (t * 6.0) as i32 % 2 == 0;
            for (dx, dy) in [(1, 0), (2, 0), (1, 1), (2, 1)] {
                out.set(hx + dx, top + dy, MAX);
            }
            out.set(hx + 3, top + if wide { 0 } else { 1 }, MAX);
        }
    }

    /// Once a visit, a crow walks across from the right, stops to peck at
    /// the snow, and walks on.
    fn crow(&self, t: f32, out: &mut Frame) {
        let s = t - CROW_START;
        if s < 0.0 {
            return;
        }
        let from = W as f32 + 2.0;
        let reach = (from - CROW_PECK_AT) / CROW_SPEED;
        let (x, frame) = if s < reach {
            (from - s * CROW_SPEED, ((t * 6.0) as usize) % 2)
        } else if s < reach + CROW_PECK {
            // Peck, look up, peck.
            (CROW_PECK_AT, if ((s - reach) * 3.0) as i32 % 2 == 0 { 2 } else { 0 })
        } else {
            (CROW_PECK_AT - (s - reach - CROW_PECK) * CROW_SPEED, ((t * 6.0) as usize) % 2)
        };
        if x < -8.0 {
            return;
        }
        blit_between(out, &Sprite::from_rows(&CROW[frame]), x, CROW_FEET - 4);
    }

    /// Snow blowing low across the ice with the wind; and falling snow
    /// when the weather says it is snowing.
    fn snow(&self, t: f32, out: &mut Frame) {
        let (mph, dir) = self.wind();
        let speed = dir * (3.0 + mph * 0.7);
        let count = if mph < 3.0 { 4 } else { (6.0 + mph * 0.6) as u32 };
        let span = W as f32 + 6.0;
        for i in 0..count {
            let h = hash(i, 11);
            let x = (h * span + speed * (0.7 + 0.6 * hash(i, 12)) * t).rem_euclid(span) - 3.0;
            let y = HORIZON as f32 + 3.0 + hash(i, 13) * (H - HORIZON - 4) as f32 + (t * 2.0 + i as f32).sin();
            let (xi, yi) = (x.round() as i32, y.round() as i32);
            out.set(xi, yi, 2);
            out.set(xi - dir.signum() as i32, yi, 1); // a short streak behind
        }
        if self.weather.now.icon == "snow" {
            for i in 0..22 {
                let fall = 4.0 + hash(i, 21) * 3.0;
                let y = (hash(i, 22) * H as f32 + fall * t).rem_euclid(H as f32);
                let x = (hash(i, 23) * span + speed * 0.3 * t + (t * 1.5 + i as f32).sin() * 1.5).rem_euclid(span) - 3.0;
                out.set(x.round() as i32, y.round() as i32, if fall > 5.5 { MAX } else { 2 });
            }
        }
    }

    /// The pages while the lake is iced: since when, temperature and wind
    /// chill, wind, and dawn and dusk.
    pub(super) fn winter_pages(&self, f: &Feed) -> Vec<Page> {
        let mut pages = Vec::new();
        let since = NaiveDate::parse_from_str(&f.ice_since, "%Y-%m-%d")
            .map(|d| d.format("%b %-d").to_string().to_uppercase())
            .map(|d| fit(&[format!("ICED SINCE {d}"), format!("ICED {d}")]))
            .unwrap_or_else(|_| "ICED OVER".into());
        pages.push(page(&self.title, &since));
        let now = &self.weather.now;
        if let Some(t) = now.temp_f {
            let feels = match now.feels_f {
                Some(f) if (f - t).abs() >= 3.0 => format!("FEELS {}°", f.round() as i32),
                _ => String::new(),
            };
            pages.push(page(&format!("NOW {}°", t.round() as i32), &feels));
        }
        if let Some(mph) = now.wind_mph {
            pages.push(Page {
                lines: [format!("WIND {}", clean(&now.wind_dir)), format!("{} MPH", mph.round() as i32)],
                arrow: Some(now.wind_dir.clone()),
            });
        }
        let times: Vec<String> = f
            .windows
            .iter()
            .map(|w| {
                let start = clean(&w.start);
                let short = start.trim_end_matches(" AM").trim_end_matches(" PM").to_string();
                fit(&[format!("{} {start}", clean(&w.kind)), format!("{} {short}", clean(&w.kind))])
            })
            .collect();
        if !times.is_empty() {
            pages.push(page(&times[0], times.get(1).map_or("", |s| s.as_str())));
        }
        pages
    }
}
