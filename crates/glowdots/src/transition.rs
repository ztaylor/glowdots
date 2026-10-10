//! Ways to move from one scene to the next. Each one mixes the last frame
//! of the old scene with the live frame of the new scene.

use crate::frame::{Frame, MAX, Size};
use crate::math::{Rng, ease_in_out, hash};
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Transition {
    /// Pinball: a bright bar sweeps across, left to right.
    Wipe,
    /// Pinball: dots change over in a random order.
    Dissolve,
    /// The new scene pushes the old one out. +1 = from the right.
    Push(i32),
    /// A ragged ring of ink spreads from a point, with spatter ahead of
    /// it. Return `InkBlot` with any values from [`Scene::transition`];
    /// the deck picks a random point and edge each time.
    ///
    /// [`Scene::transition`]: crate::scene::Scene::transition
    InkBlot { x: f32, y: f32, phase: [f32; 3] },
}

impl Transition {
    /// An ink blot at a random place, with a random ragged edge.
    pub fn ink_blot(rng: &mut Rng, size: Size) -> Self {
        let (w, h) = (size.w as f32, size.h as f32);
        Transition::InkBlot {
            x: rng.range(w * 0.25, w * 0.75),
            y: rng.range(h * 0.25, h * 0.75),
            phase: [rng.range(0.0, TAU), rng.range(0.0, TAU), rng.range(0.0, TAU)],
        }
    }

    pub fn seconds(&self) -> f32 {
        match self {
            Transition::Wipe => 0.45,
            Transition::Dissolve => 0.6,
            Transition::Push(_) => 0.35,
            Transition::InkBlot { .. } => 0.55,
        }
    }

    /// Mix `old` into `new` at progress p (0..1). The result is in `new`.
    pub fn apply(&self, p: f32, old: &Frame, new: &mut Frame) {
        let (w, h) = (new.w(), new.h());
        match *self {
            Transition::Wipe => {
                // The bar is wider than one frame's step, so it leaves no gaps.
                let edge = (p * (w as f32 + 4.0)) as i32;
                for y in 0..h {
                    for x in 0..w {
                        if (edge - 4..=edge).contains(&x) {
                            new.set(x, y, MAX);
                        } else if x > edge {
                            new.set(x, y, old.get(x, y));
                        }
                    }
                }
            }
            Transition::Dissolve => {
                for y in 0..h {
                    for x in 0..w {
                        if hash(x as u32, y as u32) >= p {
                            new.set(x, y, old.get(x, y));
                        }
                    }
                }
            }
            Transition::Push(dir) => {
                let shift = ((1.0 - ease_in_out(p)) * w as f32).round() as i32 * dir.signum();
                let fresh = new.clone();
                for y in 0..h {
                    for x in 0..w {
                        let from_new = x - shift;
                        let v = if (0..w).contains(&from_new) { fresh.get(from_new, y) } else { old.get(x - shift + dir.signum() * w, y) };
                        new.set(x, y, v);
                    }
                }
            }
            Transition::InkBlot { x: bx, y: by, phase } => {
                // By the end, the ring (even at its most ragged) reaches
                // past the farthest corner, and the band behind it too.
                let corner = [(0.0, 0.0), (w as f32, 0.0), (0.0, h as f32), (w as f32, h as f32)]
                    .into_iter()
                    .map(|(cx, cy)| (cx - bx).hypot(cy - by))
                    .fold(0.0, f32::max);
                let reach = (corner + 4.0) / (1.0 - 0.16 - 0.10 - 0.07);
                let r = p * p.sqrt() * reach;
                for y in 0..h {
                    for x in 0..w {
                        let (dx, dy) = (x as f32 - bx, y as f32 - by);
                        let (d, a) = (dx.hypot(dy), dy.atan2(dx));
                        let wobble = 1.0
                            + 0.16 * (3.0 * a + phase[0]).sin()
                            + 0.10 * (5.0 * a + phase[1]).sin()
                            + 0.07 * (11.0 * a + phase[2]).sin();
                        let edge = r * wobble;
                        if d < edge - 4.0 {
                            continue; // new scene
                        }
                        // Spatter flies just ahead of the ring.
                        let spray = d < edge + 7.0 * hash(x as u32, y as u32) && hash(y as u32 + 99, x as u32) < 0.25;
                        new.set(x, y, if d < edge || spray { MAX } else { old.get(x, y) });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames() -> (Frame, Frame) {
        let size = Size::new(16, 4);
        let mut old = Frame::new(size);
        old.fill(old.rect(), 1);
        let mut new = Frame::new(size);
        new.fill(new.rect(), 2);
        (old, new)
    }

    #[test]
    fn every_transition_ends_on_the_new_scene() {
        let mut rng = Rng(7);
        for tr in [
            Transition::Wipe,
            Transition::Dissolve,
            Transition::Push(1),
            Transition::Push(-1),
            Transition::ink_blot(&mut rng, Size::new(16, 4)),
        ] {
            let (old, mut new) = frames();
            tr.apply(0.999, &old, &mut new);
            let stale = new.dots.iter().filter(|&&d| d == 1).count();
            assert_eq!(stale, 0, "{tr:?} left {stale} old dots");
        }
    }

    #[test]
    fn transitions_start_on_the_old_scene() {
        for tr in [Transition::Wipe, Transition::Dissolve, Transition::Push(1)] {
            let (old, mut new) = frames();
            tr.apply(0.0, &old, &mut new);
            let fresh = new.dots.iter().filter(|&&d| d == 2).count();
            assert!(fresh <= 1, "{tr:?} showed {fresh} new dots at the start");
        }
    }
}
