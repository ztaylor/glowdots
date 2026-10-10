//! Shared drawing for the art scenes: dithered shading, text with a dark
//! outline, and sky objects (sun, moon, stars, clouds) and a wind arrow.

use crate::frame::{Frame, MAX, Rect};
use crate::math::hash;
use std::f32::consts::TAU;

/// A 4x4 Bayer matrix: ordered dithering, the way 90s demos stretched a
/// small palette. It gives the 4 dot shades many more in-between steps.
const BAYER: [[f32; 4]; 4] = [[0.0, 8.0, 2.0, 10.0], [12.0, 4.0, 14.0, 6.0], [3.0, 11.0, 1.0, 9.0], [15.0, 7.0, 13.0, 5.0]];

/// Set a dot from a brightness 0..1, dithered.
pub fn shade(out: &mut Frame, x: i32, y: i32, v: f32) {
    let b = (BAYER[y.rem_euclid(4) as usize][x.rem_euclid(4) as usize] + 0.5) / 16.0;
    let level = (v.clamp(0.0, 1.0) * MAX as f32 + b - 0.5).round().clamp(0.0, MAX as f32);
    out.set(x, y, level as u8);
}

/// Copy the lit dots of `layer` onto `out` with a dark edge around them,
/// so text stays readable over a busy picture.
pub fn overlay_outlined(out: &mut Frame, layer: &Frame) {
    for y in 0..out.h() {
        for x in 0..out.w() {
            if layer.get(x, y) > 0 {
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (-1, 1), (1, -1)] {
                    if layer.get(x + dx, y + dy) == 0 {
                        out.set(x + dx, y + dy, 0);
                    }
                }
            }
        }
    }
    for y in 0..out.h() {
        for x in 0..out.w() {
            let v = layer.get(x, y);
            if v > 0 {
                out.set(x, y, v);
            }
        }
    }
}

fn set_in(out: &mut Frame, clip: Rect, x: i32, y: i32, v: u8) {
    if clip.contains(x, y) {
        out.set(x, y, v);
    }
}

/// A sun: a bright disc with rays that turn slowly and breathe.
pub fn sun(out: &mut Frame, cx: f32, cy: f32, r: f32, t: f32, clip: Rect) {
    for y in (cy - r - 6.0) as i32..=(cy + r + 6.0) as i32 {
        for x in (cx - r - 6.0) as i32..=(cx + r + 6.0) as i32 {
            let (dx, dy) = (x as f32 - cx, y as f32 - cy);
            let d = dx.hypot(dy);
            if d <= r {
                set_in(out, clip, x, y, MAX);
            } else {
                // 8 rays, each a narrow wedge; the ring of rays turns and pulses.
                let a = dy.atan2(dx) + t * 0.4;
                let wedge = ((a * 8.0 / TAU).fract() - 0.5).abs();
                let reach = r + 3.0 + 1.2 * (t * 2.0).sin();
                if d > r + 1.5 && d < reach && wedge > 0.38 {
                    set_in(out, clip, x, y, 2);
                }
            }
        }
    }
}

/// A moon at its phase. `lit` is the lit fraction (0 new .. 1 full).
/// Waxing moons are lit on the right (as seen from the north). A thin
/// moon shows the rest of its disc faintly (earthshine).
pub fn moon(out: &mut Frame, cx: f32, cy: f32, r: f32, lit: f32, waxing: bool, clip: Rect) {
    for y in (cy - r) as i32 - 1..=(cy + r) as i32 + 1 {
        for x in (cx - r) as i32 - 1..=(cx + r) as i32 + 1 {
            let (dx, dy) = (x as f32 - cx, y as f32 - cy);
            if dx.hypot(dy) > r + 0.3 {
                continue;
            }
            let half = (r * r - dy * dy).max(0.0).sqrt();
            let side = if waxing { dx } else { -dx };
            if side >= half * (1.0 - 2.0 * lit) - 0.3 {
                set_in(out, clip, x, y, MAX);
            } else if lit < 0.35 {
                set_in(out, clip, x, y, 1);
            }
        }
    }
}

/// Stars in `area` that twinkle. Fixed places, so they don't move.
pub fn stars(out: &mut Frame, area: Rect, t: f32, density: f32) {
    for y in area.y0..area.y1 {
        for x in area.x0..area.x1 {
            let h = hash(x as u32 * 7 + 3, y as u32 * 13 + 1);
            if h < density {
                let twinkle = hash(x as u32, y as u32 + (t * 3.0 + h * 50.0) as u32);
                let level = if twinkle > 0.85 {
                    MAX
                } else if twinkle > 0.3 {
                    2
                } else {
                    1
                };
                out.set(x, y, level);
            }
        }
    }
}

/// A puffy cloud centred at (cx, cy). Its body is `fill`, its sunlit top
/// edge `edge`; its base is flat.
pub fn cloud(out: &mut Frame, cx: f32, cy: f32, scale: f32, fill: u8, edge: u8, clip: Rect) {
    const PUFFS: [(f32, f32, f32); 5] = [(-6.0, 1.0, 3.6), (-2.0, -1.5, 4.4), (3.0, -0.5, 4.2), (6.5, 1.5, 3.2), (0.0, 1.5, 4.0)];
    let inside = |x: f32, y: f32| {
        y <= cy + 3.2 * scale && PUFFS.iter().any(|&(px, py, pr)| (x - (cx + px * scale)).hypot(y - (cy + py * scale)) <= pr * scale)
    };
    let (w, h) = (11.0 * scale, 7.0 * scale);
    for y in (cy - h) as i32..=(cy + h) as i32 {
        for x in (cx - w) as i32..=(cx + w) as i32 {
            if inside(x as f32, y as f32) {
                let top = !inside(x as f32, y as f32 - 1.0);
                set_in(out, clip, x, y, if top { edge } else { fill });
            }
        }
    }
}

/// An arrow (7x7) pointing where the wind blows *to*. `from` is the
/// compass direction the wind comes from ("WNW").
pub fn wind_arrow(out: &mut Frame, x: i32, y: i32, from: &str, level: u8) {
    let Some(deg) = compass(from) else { return };
    let a = (deg + 180.0).to_radians(); // blowing towards
    let (dx, dy) = (a.sin(), -a.cos()); // screen: north is up
    let (cx, cy) = (x as f32 + 3.0, y as f32 + 3.0);
    for i in -3..=3 {
        out.set((cx + dx * i as f32).round() as i32, (cy + dy * i as f32).round() as i32, level);
    }
    // The head: two short strokes back from the tip.
    let (tx, ty) = (cx + dx * 3.0, cy + dy * 3.0);
    for side in [-1.0f32, 1.0] {
        let b = a + std::f32::consts::PI + side * 0.6;
        for i in 1..=2 {
            out.set((tx + b.sin() * i as f32).round() as i32, (ty - b.cos() * i as f32).round() as i32, level);
        }
    }
}

/// Compass point -> degrees (N = 0, E = 90).
pub fn compass(dir: &str) -> Option<f32> {
    const POINTS: [&str; 16] = ["N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW"];
    POINTS.iter().position(|p| p.eq_ignore_ascii_case(dir.trim())).map(|i| i as f32 * 22.5)
}
