//! Shared layout parts for the pinball-style data scenes, so they all
//! look like one design: a header bar, chips, dividers, and long text that
//! scrolls.

use crate::font::{Font, SMALL};
use crate::frame::{Frame, MAX, Rect};

/// Rows 0..7 are the header bar. Content starts here.
pub const CONTENT_TOP: i32 = 8;
/// Long text pauses, then scrolls at this speed (dots per second).
pub const SCROLL_SPEED: f32 = 22.0;
const SCROLL_PAUSE: f32 = 1.0;

/// A dim bar across the top with a bright label at each end.
pub fn header(out: &mut Frame, left: &str, right: &str) {
    out.fill(Rect::new(0, 0, out.w(), 7), 1);
    SMALL.draw(out, left, 2, 1, MAX);
    SMALL.draw_right(out, right, out.w() - 2, 1, MAX);
}

/// Bright text on a dim box, like a DMD badge. (Dark text cut out of a
/// lit box does not read: the glow fills the letters in.) Returns its width.
pub fn chip(out: &mut Frame, text: &str, x: i32, y: i32) -> i32 {
    let w = SMALL.width(text) + 4;
    out.fill(Rect::new(x, y, x + w, y + 7), 1);
    SMALL.draw(out, text, x + 2, y + 1, MAX);
    w
}

pub fn chip_right(out: &mut Frame, text: &str, right: i32, y: i32) -> i32 {
    chip(out, text, right - SMALL.width(text) - 4, y)
}

/// Seconds that `marquee` needs to show all of the text.
pub fn marquee_seconds(font: &Font, text: &str, width: i32) -> f32 {
    let over = (font.width(text) - width).max(0) as f32;
    if over == 0.0 { 0.0 } else { SCROLL_PAUSE * 2.0 + over / SCROLL_SPEED }
}

/// Text in the box x0..x1. If it is too long, it waits, scrolls to its
/// end, and holds there.
#[allow(clippy::too_many_arguments)]
pub fn marquee(out: &mut Frame, font: &Font, text: &str, x0: i32, x1: i32, y: i32, level: u8, t: f32) {
    let over = font.width(text) - (x1 - x0);
    let shift = if over <= 0 { 0 } else { (((t - SCROLL_PAUSE).max(0.0) * SCROLL_SPEED) as i32).min(over) };
    font.draw_clip(out, text, x0 - shift, y, level, Rect::new(x0, 0, x1, out.h()));
}

/// Pages that show one after the other, each for its own time.
pub struct Pager {
    pub seconds: Vec<f32>,
}

/// How long the content slides up when the page changes.
pub const PAGE_SLIDE: f32 = 0.3;

impl Pager {
    pub fn total(&self) -> f32 {
        self.seconds.iter().sum()
    }

    /// Which page shows at time t, and the time into that page.
    pub fn locate(&self, mut t: f32) -> (usize, f32) {
        for (i, &s) in self.seconds.iter().enumerate() {
            if t < s || i + 1 == self.seconds.len() {
                return (i, t);
            }
            t -= s;
        }
        (0, t)
    }
}

/// Slide the content rows up from `old` to `new` (p = 0..1).
pub fn slide_up(out: &mut Frame, old: &Frame, new: &Frame, p: f32) {
    let h = out.h() - CONTENT_TOP;
    let shift = (crate::math::ease_in_out(p) * h as f32).round() as i32;
    for y in CONTENT_TOP..out.h() {
        let from_old = y + shift;
        for x in 0..out.w() {
            let v = if from_old < out.h() { old.get(x, from_old) } else { new.get(x, from_old - h) };
            out.set(x, y, v);
        }
    }
}
