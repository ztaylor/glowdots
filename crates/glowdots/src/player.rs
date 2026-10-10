//! The player: a deck, its afterglow, and how it is shown. Window code
//! (the desktop window, the browser, a bare Raspberry Pi panel) calls it
//! each frame and passes it the viewer's taps and keys.

use crate::deck::Deck;
use crate::frame::{Frame, Phosphor};
use crate::glass::Glass;
use crate::render::{Fit, Renderer, View};
use crate::scene::{Input, Key};
use anyhow::{Result, anyhow};

pub struct Player {
    pub deck: Deck,
    pub glass: Glass,
    /// Quarter turns of the panel on the screen (0..3). A panel mounted
    /// sideways uses 1 or 3.
    pub rotate: u32,
    /// Show a thin line under the panel that fills up until the next scene.
    pub progress_line: bool,
    frame: Frame,
    phosphor: Phosphor,
    /// Seconds played, summed in f64 so long runs do not drift.
    time: f64,
}

impl Player {
    /// Play `deck` on classic glass, the right way up, with the progress line.
    pub fn new(deck: Deck) -> Self {
        let frame = deck.new_frame();
        let phosphor = Phosphor::new(deck.size());
        Self { deck, glass: Glass::CLASSIC, rotate: 0, progress_line: true, frame, phosphor, time: 0.0 }
    }

    pub fn with_glass(mut self, glass: Glass) -> Self {
        self.glass = glass;
        self
    }

    pub fn with_rotate(mut self, rotate: u32) -> Self {
        self.rotate = rotate % 4;
        self
    }

    /// Move on `dt` seconds: the deck draws its next frame, and the
    /// afterglow follows it.
    pub fn step(&mut self, dt: f32) {
        self.time += dt as f64;
        self.deck.update(dt, &mut self.frame);
        self.phosphor.decay_seconds = self.glass.afterglow;
        self.phosphor.update(&self.frame, dt);
    }

    /// The last frame the deck drew.
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// Seconds played.
    pub fn time(&self) -> f32 {
        self.time as f32
    }

    /// How the next draw looks.
    pub fn view(&self) -> View {
        View {
            rotate: self.rotate,
            time: self.time as f32,
            look: self.deck.look(),
            glass: self.glass,
            progress: self.progress_line.then(|| self.deck.progress()),
        }
    }

    /// Draw to the screen.
    pub fn draw(&mut self, r: &Renderer) {
        r.upload(self.phosphor.rgba());
        r.draw(&self.view());
    }

    /// Draw offscreen and return the picture, turned the way the viewer
    /// sees it.
    pub fn capture(&mut self, r: &mut Renderer) -> Result<image::RgbaImage> {
        r.upload(self.phosphor.rgba());
        r.draw_offscreen(&self.view())?;
        let (w, h) = r.screen();
        let img = image::RgbaImage::from_raw(w, h, r.read_pixels()).ok_or_else(|| anyhow!("bad framebuffer size"))?;
        Ok(match self.rotate % 4 {
            1 => image::imageops::rotate90(&img),
            2 => image::imageops::rotate180(&img),
            3 => image::imageops::rotate270(&img),
            _ => img,
        })
    }

    /// A tap or click at a point on a screen of `screen` pixels (top-left
    /// origin).
    pub fn tap(&mut self, screen: (u32, u32), x: f32, y: f32) {
        let (px, py) = Fit::new(screen, self.deck.size(), self.rotate).to_panel(screen, self.rotate, x, y);
        self.deck.input(Input::Tap { x: px, y: py });
    }

    pub fn key(&mut self, key: Key) {
        self.deck.input(Input::Key(key));
    }

    /// Play `seconds` in steps of 1/60 s without drawing: to get to a moment.
    pub fn skip(&mut self, seconds: f32) {
        let dt = 1.0 / 60.0;
        for _ in 0..(seconds / dt).round() as usize {
            self.step(dt);
        }
    }
}
