//! A tribute to the 1990s PC demoscene (Future Crew's Second Reality and
//! friends): a short demo of classic effects, cut together with flashes.
//!
//! Parts: starfield with title cards, plasma, dot tunnel, and a 3D dot
//! torus. (Fire under a sine-wave greetings scroller
//! is written but out of the sequence for now; see `parts`.)

use glowdots::art::{self, overlay_outlined};
use glowdots::font::REGULAR;
use glowdots::math::Rng;
use glowdots::{Frame, MAX, Scene, Size, Transition};
use std::f32::consts::{PI, TAU};

const W: usize = crate::SIZE.w as usize;
const H: usize = crate::SIZE.h as usize;
const FLASH_SECONDS: f32 = 0.3;
const SCROLL_SPEED: f32 = 32.0; // dots per second
const GREETINGS: &str = "GREETINGS TO FUTURE CREW · PURPLE MOTION · SKAVEN · TRITON · FARBRAUSCH · AND ALL DEMOSCENERS";

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Stars,
    Plasma,
    Tunnel,
    Torus,
    #[allow(dead_code)] // out of the sequence for now
    Fire,
}

/// Set a dot from a brightness 0..1, dithered.
fn shade(out: &mut Frame, x: usize, y: usize, v: f32) {
    art::shade(out, x as i32, y as i32, v);
}

struct Star {
    x: f32,
    y: f32,
    z: f32,
}

pub struct Demo {
    stars: Vec<Star>,
    torus: Vec<[f32; 3]>,
    fire: Vec<f32>,
    fire_clock: f32,
    scroller: Vec<u16>,
    rng: Rng,
}

impl Default for Demo {
    fn default() -> Self {
        Self::new()
    }
}

impl Demo {
    pub fn new() -> Self {
        let mut torus = Vec::new();
        for i in 0..34 {
            for j in 0..10 {
                let (u, v) = (TAU * i as f32 / 34.0, TAU * j as f32 / 10.0);
                let r = 1.0 + 0.42 * v.cos();
                torus.push([r * u.cos(), r * u.sin(), 0.42 * v.sin()]);
            }
        }
        Self {
            stars: Vec::new(),
            torus,
            fire: vec![0.0; W * (H + 2)],
            fire_clock: 0.0,
            scroller: REGULAR.columns(GREETINGS),
            rng: Rng(0xdead_beef),
        }
    }

    /// The demo, in order. To bring the fire back, add
    /// `(Part::Fire, self.fire_seconds())`.
    fn parts(&self) -> Vec<(Part, f32)> {
        vec![(Part::Stars, 8.0), (Part::Plasma, 7.0), (Part::Tunnel, 7.0), (Part::Torus, 7.0)]
    }

    #[allow(dead_code)]
    fn fire_seconds(&self) -> f32 {
        (W + self.scroller.len()) as f32 / SCROLL_SPEED
    }

    fn new_star(&mut self, z: f32) -> Star {
        Star { x: self.rng.range(-1.0, 1.0), y: self.rng.range(-1.0, 1.0), z }
    }

    /// Stars fly at you; near ones are bright and leave a short streak.
    fn stars(&mut self, dt: f32, t: f32, out: &mut Frame) {
        if self.stars.is_empty() {
            for _ in 0..90 {
                let z = self.rng.range(0.05, 1.0);
                let s = self.new_star(z);
                self.stars.push(s);
            }
        }
        let speed = 0.35 + 0.25 * (t / 8.0);
        let project = |s: &Star, z: f32| (40.0 + s.x / z * 30.0, 16.0 + s.y / z * 30.0);
        for i in 0..self.stars.len() {
            let old_z = self.stars[i].z;
            self.stars[i].z -= speed * dt;
            let (x, y) = project(&self.stars[i], self.stars[i].z);
            if self.stars[i].z <= 0.02 || !(0.0..W as f32).contains(&x) || !(0.0..H as f32).contains(&y) {
                self.stars[i] = self.new_star(1.0);
                continue;
            }
            let z = self.stars[i].z;
            let level = if z < 0.3 {
                MAX
            } else if z < 0.6 {
                2
            } else {
                1
            };
            if z < 0.45 {
                let (tx, ty) = project(&self.stars[i], (old_z + 0.04).min(1.0));
                out.set(tx.round() as i32, ty.round() as i32, 1);
            }
            out.set(x.round() as i32, y.round() as i32, level);
        }

        // Title cards.
        let card = match t {
            t if (1.0..3.0).contains(&t) => Some("GLOWDOTS"),
            t if (3.3..5.3).contains(&t) => Some("PRESENTS"),
            t if t >= 5.6 => Some("DOTSCENE"),
            _ => None,
        };
        if let Some(text) = card {
            let mut layer = out.blank();
            REGULAR.draw_centered(&mut layer, text, 12, MAX);
            overlay_outlined(out, &layer);
        }
    }

    /// Sums of sines, folded into bands: the classic plasma.
    fn plasma(t: f32, out: &mut Frame) {
        let (cx, cy) = (40.0 + 25.0 * (t * 0.7).sin(), 16.0 + 10.0 * (t * 0.9).cos());
        for y in 0..H {
            for x in 0..W {
                let (fx, fy) = (x as f32, y as f32);
                let v = (fx * 0.16 + t).sin()
                    + (fy * 0.23 + t * 1.3).sin()
                    + ((fx + fy) * 0.11 + t * 0.7).sin()
                    + ((fx - cx).hypot(fy - cy) * 0.28 - t * 2.0).sin();
                shade(out, x, y, 0.5 + 0.5 * (v * PI * 0.5).sin());
            }
        }
    }

    /// The dot tunnel: rings of dots rush at you along a winding path.
    /// Far rings are dim, near rings bright.
    fn tunnel(t: f32, out: &mut Frame) {
        const RINGS: usize = 14;
        const SPACING: f32 = 0.35;
        // Where the tunnel's centre line is at depth z (it winds).
        let path = |z: f32| (40.0 + 14.0 * ((t + z) * 0.9).sin(), 16.0 + 6.0 * ((t + z) * 1.3).cos());
        let travel = (t * 1.6) % SPACING;
        let (vx, vy) = path(0.0);
        for k in (0..RINGS).rev() {
            let z = 0.25 + k as f32 * SPACING - travel;
            let (cx, cy) = path(z);
            // Rings are drawn relative to the view centre, so the path
            // bends the tunnel instead of moving it.
            let (ox, oy) = ((cx - vx) / z * 0.6, (cy - vy) / z * 0.6);
            let r = 11.0 / z;
            let level = if z < 1.2 {
                MAX
            } else if z < 2.6 {
                2
            } else {
                1
            };
            let n = (r * 1.4).clamp(8.0, 40.0) as usize;
            for i in 0..n {
                let a = TAU * i as f32 / n as f32 + t * 0.7 + k as f32 * 0.2;
                out.set((40.0 + ox + r * a.cos()).round() as i32, (16.0 + oy + r * a.sin()).round() as i32, level);
            }
        }
    }

    /// A torus of dots spinning in 3D. Near dots are bright, far ones dim.
    fn torus(&self, t: f32, out: &mut Frame) {
        let (a, b, c) = (t * 0.9, t * 0.6, t * 0.35);
        let mut pts: Vec<(f32, f32, f32)> = self
            .torus
            .iter()
            .map(|&[x, y, z]| {
                let (y, z) = (y * a.cos() - z * a.sin(), y * a.sin() + z * a.cos());
                let (x, z) = (x * b.cos() + z * b.sin(), -x * b.sin() + z * b.cos());
                let (x, y) = (x * c.cos() - y * c.sin(), x * c.sin() + y * c.cos());
                let p = 40.0 / (z + 3.6);
                (40.0 + 10.0 * (t * 0.5).sin() + x * p, 16.0 + y * p, z)
            })
            .collect();
        pts.sort_by(|p, q| q.2.total_cmp(&p.2)); // far first, near on top
        for (x, y, z) in pts {
            let level = if z < -0.45 {
                MAX
            } else if z < 0.35 {
                2
            } else {
                1
            };
            out.set(x.round() as i32, y.round() as i32, level);
        }
    }

    /// Classic fire: heat rises from the bottom and cools as it goes.
    fn fire(&mut self, dt: f32, t: f32, out: &mut Frame) {
        let rows = H + 2;
        self.fire_clock += dt;
        while self.fire_clock > 1.0 / 30.0 {
            self.fire_clock -= 1.0 / 30.0;
            // Embers: hot clumps that come and go along the bottom.
            let mut x = 0;
            while x < W {
                let (len, hot) = (self.rng.range(2.0, 7.0) as usize, self.rng.next_f32() < 0.5);
                for xx in x..(x + len).min(W) {
                    for y in rows - 2..rows {
                        self.fire[y * W + xx] = if hot { 1.0 } else { 0.0 };
                    }
                }
                x += len;
            }
            // Heat rises and cools by a random amount, so the flames lick.
            for y in 0..rows - 2 {
                for x in 0..W {
                    let at = |xx: usize, yy: usize| self.fire[yy * W + xx.min(W - 1)];
                    let sum = at(x.saturating_sub(1), y + 1) + at(x, y + 1) + at(x + 1, y + 1) + at(x, y + 2);
                    self.fire[y * W + x] = (sum / 4.0 - self.rng.range(0.0, 0.06)).max(0.0);
                }
            }
        }
        for y in 0..H {
            for x in 0..W {
                shade(out, x, y, self.fire[y * W + x].powf(1.3) * 1.15);
            }
        }

        // Greetings, riding a sine wave across the flames.
        let mut layer = out.blank();
        let start = W as f32 - t * SCROLL_SPEED;
        for (i, &bits) in self.scroller.iter().enumerate() {
            let x = (start + i as f32).round() as i32;
            if !(0..W as i32).contains(&x) {
                continue;
            }
            let y0 = 5.0 + 4.0 * (x as f32 * 0.12 + t * 3.0).sin();
            for row in 0..7 {
                if bits & 1 << row != 0 {
                    layer.set(x, y0.round() as i32 + row, MAX);
                }
            }
        }
        overlay_outlined(out, &layer);
    }
}

impl Scene for Demo {
    fn name(&self) -> &str {
        "demo"
    }

    fn enter(&mut self) {
        self.stars.clear();
        self.fire.fill(0.0);
    }

    fn update(&mut self, dt: f32, t: f32, out: &mut Frame) {
        let mut local = t;
        let mut index = 0;
        let parts = self.parts();
        for (i, &(_, secs)) in parts.iter().enumerate() {
            index = i;
            if local < secs || i + 1 == parts.len() {
                break;
            }
            local -= secs;
        }
        match parts[index].0 {
            Part::Stars => self.stars(dt, local, out),
            Part::Plasma => Self::plasma(local, out),
            Part::Tunnel => Self::tunnel(local, out),
            Part::Torus => self.torus(local, out),
            Part::Fire => self.fire(dt, local, out),
        }

        // Demo-style cut: each new part opens with a white flash.
        if index > 0 && local < FLASH_SECONDS {
            let level = ((1.0 - local / FLASH_SECONDS) * MAX as f32).round() as u8;
            for y in 0..H as i32 {
                for x in 0..W as i32 {
                    let v = out.get(x, y).max(level);
                    out.set(x, y, v);
                }
            }
        }
    }

    fn duration(&self) -> f32 {
        self.parts().iter().map(|p| p.1).sum()
    }

    fn transition(&self) -> Transition {
        Transition::Dissolve
    }

    fn size(&self) -> Option<Size> {
        Some(crate::SIZE)
    }
}
