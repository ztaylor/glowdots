//! Scenes: the things the display shows, one at a time.
//!
//! Each scene draws its own frames and chooses its own look (dot colour)
//! and its own way in (transition). So a scene can have its own style
//! without the others changing.

use crate::feed::{self, Feed, FeedError, Feeds, NoFeeds};
use crate::frame::{CLASSIC_SHADES, Frame, Rgb, Size};
use crate::transition::Transition;
use chrono::{DateTime, Local};
use serde::de::DeserializeOwned;

const NO_PALETTE: [[f32; 3]; 16] = [[0.0; 3]; 16];

/// How the dots look while a scene shows: a colour, or a palette that
/// gives each shade its own colour. The renderer reads this.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Look {
    color: [f32; 3],
    /// The palette, resampled to 16 steps from off to full brightness.
    palette: [[f32; 3]; 16],
    /// 0 uses the colour, 1 the palette. Between the two while a
    /// transition blends looks.
    palette_weight: f32,
}

impl Look {
    /// Williams WPC red-orange neon plasma. The classic.
    pub const WILLIAMS: Look = Look::rgb(1.0, 0.26, 0.02);
    /// A warmer amber, like a worn panel.
    pub const AMBER: Look = Look::rgb(1.0, 0.5, 0.08);
    /// Red neon.
    pub const RED: Look = Look::rgb(1.0, 0.08, 0.04);
    /// Green phosphor, like an old terminal.
    pub const GREEN: Look = Look::rgb(0.25, 1.0, 0.3);
    /// Ice blue.
    pub const BLUE: Look = Look::rgb(0.3, 0.55, 1.0);
    /// Warm white, like LED replacement panels.
    pub const WHITE: Look = Look::rgb(1.0, 0.92, 0.8);

    /// Dots of one colour (0..1 per channel). Brighter shades are brighter
    /// versions of it, with white-hot centres.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Look {
        Look { color: [r, g, b], palette: NO_PALETTE, palette_weight: 0.0 }
    }

    /// Dots of one colour, from a web hex number like `0xff4205`.
    pub fn hex(v: u32) -> Look {
        let [r, g, b] = Rgb::hex(v).to_f32();
        Look::rgb(r, g, b)
    }

    /// A palette: the colour of each shade, from off (the first, usually
    /// black) to full brightness (the last). 2 to 16 colours, spread evenly
    /// over the shades. Colour DMD kits colourise the classic 4 shades
    /// this way.
    pub fn palette(colors: &[Rgb]) -> Look {
        let colors: Vec<[f32; 3]> = match colors.len() {
            0 => return Look::WILLIAMS,
            1 => vec![[0.0; 3], colors[0].to_f32()],
            _ => colors.iter().take(16).map(|c| c.to_f32()).collect(),
        };
        let n = colors.len();
        let mut palette = NO_PALETTE;
        for (k, entry) in palette.iter_mut().enumerate() {
            let x = k as f32 / 15.0 * (n - 1) as f32;
            let (i, f) = (x.floor() as usize, x.fract());
            let j = (i + 1).min(n - 1);
            *entry = std::array::from_fn(|c| colors[i][c] + (colors[j][c] - colors[i][c]) * f);
        }
        // The glass glows in the hue of the brightest colour.
        let top = colors[n - 1];
        let m = top.iter().copied().fold(0.0, f32::max);
        let color = if m > 0.0 { top.map(|c| c / m) } else { Look::WILLIAMS.color };
        Look { color, palette, palette_weight: 1.0 }
    }

    /// Heat: dark red, orange, yellow, white. Best with 16 shades.
    pub fn heat() -> Look {
        Look::palette(&[Rgb(0, 0, 0), Rgb(0x60, 0x08, 0x00), Rgb(0xd0, 0x30, 0x00), Rgb(0xff, 0x90, 0x10), Rgb(0xff, 0xf0, 0xb0)])
    }

    /// Ocean: deep blue through teal to white.
    pub fn ocean() -> Look {
        Look::palette(&[Rgb(0, 0, 0), Rgb(0x00, 0x18, 0x60), Rgb(0x00, 0x80, 0xc0), Rgb(0x40, 0xf0, 0xd0), Rgb(0xf0, 0xff, 0xff)])
    }

    /// The colour of a fully lit dot, and the hue of the glass's glow.
    pub fn color(&self) -> [f32; 3] {
        self.color
    }

    /// The palette, in 16 steps from off to full, if this look has one.
    pub fn palette_steps(&self) -> Option<&[[f32; 3]; 16]> {
        (self.palette_weight > 0.0).then_some(&self.palette)
    }

    /// How much the palette counts (0..1). Between 0 and 1 only while a
    /// transition blends two looks.
    pub fn palette_weight(&self) -> f32 {
        self.palette_weight
    }

    /// Blend two looks: `a` at p = 0, `b` at p = 1.
    pub fn mix(a: Look, b: Look, p: f32) -> Look {
        let lerp = |x: f32, y: f32| x + (y - x) * p;
        let palette = if a.palette_weight == 0.0 {
            b.palette
        } else if b.palette_weight == 0.0 {
            a.palette
        } else {
            std::array::from_fn(|k| std::array::from_fn(|c| lerp(a.palette[k][c], b.palette[k][c])))
        };
        Look {
            color: std::array::from_fn(|c| lerp(a.color[c], b.color[c])),
            palette,
            palette_weight: lerp(a.palette_weight, b.palette_weight),
        }
    }
}

impl Default for Look {
    fn default() -> Self {
        Look::WILLIAMS
    }
}

/// What a scene gets each time it is called: the time, and the feeds.
pub struct Context<'a> {
    /// Seconds since the last frame (0 in [`Scene::enter`]).
    pub dt: f32,
    /// Seconds since the scene started.
    pub t: f32,
    /// The local time now. It is the real time unless the deck runs a
    /// simulated clock (for previews, screenshots, and tests).
    pub now: DateTime<Local>,
    /// Where the scene reads its data.
    pub feeds: &'a dyn Feeds,
}

static NO_FEEDS: NoFeeds = NoFeeds;

impl<'a> Context<'a> {
    pub fn new(now: DateTime<Local>, feeds: &'a dyn Feeds) -> Self {
        Self { dt: 0.0, t: 0.0, now, feeds }
    }

    /// A context at `now` with no feeds. For tests and tools.
    pub fn at(now: DateTime<Local>) -> Context<'static> {
        Context::new(now, &NO_FEEDS)
    }

    /// The same context at another time into the scene.
    pub fn with_time(&self, dt: f32, t: f32) -> Context<'a> {
        Context { dt, t, now: self.now, feeds: self.feeds }
    }

    /// Read a list feed: `{"date": "YYYY-MM-DD", "items": [...]}`. See
    /// [`feed::items`].
    pub fn items<T: DeserializeOwned>(&self, name: &str) -> Feed<T> {
        feed::items(self.feeds, self.now, name)
    }

    /// Read any JSON feed into `T`. See [`feed::json`].
    pub fn json<T: DeserializeOwned>(&self, name: &str) -> Result<T, FeedError> {
        feed::json(self.feeds, name)
    }
}

/// Something the viewer did.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Input {
    /// A tap or click, in dots on the scene (x right, y down). It can be
    /// off the panel.
    Tap {
        x: f32,
        y: f32,
    },
    Key(Key),
}

/// A key, from a keyboard or buttons.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    /// Enter, or a "start" button.
    Select,
    Char(char),
}

pub trait Scene {
    /// Short name, for logs and for choosing a scene from the command line.
    fn name(&self) -> &str;

    /// The scene is about to show. Read feeds and reset animation here.
    fn enter(&mut self, cx: &Context) {
        let _ = cx;
    }

    /// Draw the frame at `cx.t` seconds after the scene started. `out` is
    /// cleared before each call.
    fn update(&mut self, cx: &Context, out: &mut Frame);

    /// How long the scene stays before the next one, in seconds.
    fn duration(&self) -> f32;

    /// How the scene comes in when the timer moves to it.
    fn transition(&self) -> Transition {
        Transition::Wipe
    }

    fn look(&self) -> Look {
        Look::WILLIAMS
    }

    /// The size this scene is composed for, if it needs one. On a bigger
    /// panel it is drawn in the middle. `None` fills any panel; read the
    /// size from the frame.
    fn size(&self) -> Option<Size> {
        None
    }

    /// How many shades the scene draws with (2..=16). The deck rescales
    /// them to the panel's. Classic scenes use 4 and [`crate::MAX`].
    fn shades(&self) -> u8 {
        CLASSIC_SHADES
    }

    /// The viewer did something. Return true if the scene used it; if not,
    /// the deck does (a tap or arrow key moves to the next or last scene).
    fn input(&mut self, cx: &Context, input: &Input) -> bool {
        let _ = (cx, input);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_land_on_their_colours() {
        let look = Look::palette(&[Rgb(0, 0, 0), Rgb(255, 0, 0), Rgb(0, 255, 0), Rgb(0, 0, 255)]);
        let p = look.palette_steps().unwrap();
        assert_eq!(p[0], [0.0, 0.0, 0.0]);
        assert_eq!(p[5], [1.0, 0.0, 0.0]);
        assert_eq!(p[10], [0.0, 1.0, 0.0]);
        assert_eq!(p[15], [0.0, 0.0, 1.0]);
        assert!(Look::WILLIAMS.palette_steps().is_none());
    }

    #[test]
    fn looks_blend_colour_into_palette() {
        let heat = Look::heat();
        let half = Look::mix(Look::WILLIAMS, heat, 0.5);
        assert_eq!(half.palette_weight(), 0.5);
        assert_eq!(half.palette_steps(), heat.palette_steps());
        assert_eq!(Look::mix(Look::WILLIAMS, Look::GREEN, 0.0), Look::WILLIAMS);
    }
}
