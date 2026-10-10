//! The DMD frame: a grid of dots, each with one of 4 shades.
//!
//! Williams WPC games drove a 128x32 plasma panel with 4 shades (off + 3).
//! A frame can be any size; the renderer fits it to the screen.

use anyhow::{Context, Result, bail};
use image::imageops::{self, FilterType};
use std::path::Path;

/// Brightest shade. Shades are 0 (off) ..= MAX.
pub const MAX: u8 = 3;

/// The panel size, in dots.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

impl Size {
    /// The Williams WPC panel, also used by Bally and Data East: 128x32.
    pub const WPC: Size = Size::new(128, 32);
    /// Later Stern and Sega games: 192x64.
    pub const STERN: Size = Size::new(192, 64);

    pub const fn new(w: u32, h: u32) -> Self {
        Self { w, h }
    }

    /// The whole panel as a rectangle.
    pub const fn rect(self) -> Rect {
        Rect::new(0, 0, self.w as i32, self.h as i32)
    }

    pub const fn dots(self) -> usize {
        (self.w * self.h) as usize
    }
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.w, self.h)
    }
}

/// A rectangle of dots: x0..x1, y0..y1 (the end is not included).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub const fn w(&self) -> i32 {
        self.x1 - self.x0
    }

    pub const fn h(&self) -> i32 {
        self.y1 - self.y0
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }
}

/// The dots of one picture. Row-major, top row first.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Frame {
    size: Size,
    pub dots: Vec<u8>,
}

impl Frame {
    pub fn new(size: Size) -> Self {
        Self { size, dots: vec![0; size.dots()] }
    }

    /// An empty frame the same size as this one, for layers.
    pub fn blank(&self) -> Self {
        Self::new(self.size)
    }

    pub fn size(&self) -> Size {
        self.size
    }

    /// Width in dots.
    pub fn w(&self) -> i32 {
        self.size.w as i32
    }

    /// Height in dots.
    pub fn h(&self) -> i32 {
        self.size.h as i32
    }

    /// The whole frame as a rectangle.
    pub fn rect(&self) -> Rect {
        self.size.rect()
    }

    pub fn clear(&mut self) {
        self.dots.fill(0);
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w() && y < self.h()).then(|| y as usize * self.size.w as usize + x as usize)
    }

    /// The dot at (x, y). Dots off the grid are 0.
    pub fn get(&self, x: i32, y: i32) -> u8 {
        self.index(x, y).map_or(0, |i| self.dots[i])
    }

    /// Set a dot. Dots off the grid are ignored.
    pub fn set(&mut self, x: i32, y: i32, v: u8) {
        if let Some(i) = self.index(x, y) {
            self.dots[i] = v.min(MAX);
        }
    }

    /// Draw a sprite with its top-left at (x, y). Off dots are transparent.
    pub fn blit(&mut self, s: &Sprite, x: i32, y: i32) {
        self.blit_clip(s, x, y, self.rect());
    }

    /// Draw a sprite, but only inside `clip`.
    pub fn blit_clip(&mut self, s: &Sprite, x: i32, y: i32, clip: Rect) {
        for sy in 0..s.h {
            for sx in 0..s.w {
                let v = s.dots[sy * s.w + sx];
                let (px, py) = (x + sx as i32, y + sy as i32);
                if v > 0 && clip.contains(px, py) {
                    self.set(px, py, v);
                }
            }
        }
    }

    pub fn fill(&mut self, r: Rect, v: u8) {
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                self.set(x, y, v);
            }
        }
    }

    /// Copy the rows y0..y1 from another frame, moved down by dy.
    pub fn copy_rows(&mut self, from: &Frame, y0: i32, y1: i32, dy: i32) {
        for y in y0..y1 {
            for x in 0..self.w() {
                let v = from.get(x, y - dy);
                self.set(x, y, v);
            }
        }
    }

    /// A still image, fitted and centered.
    pub fn from_image(path: &Path, size: Size) -> Result<Self> {
        let s = Sprite::load(path, size.w as usize, size.h as usize)?;
        let mut f = Self::new(size);
        f.blit(&s, (f.w() - s.w as i32) / 2, (f.h() - s.h as i32) / 2);
        Ok(f)
    }
}

/// A small picture made of DMD shades.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub dots: Vec<u8>,
}

impl Sprite {
    /// A sprite from a dot map: '#' full, '=' mid, '.' dim, anything else off.
    pub fn from_rows(rows: &[&str]) -> Self {
        let (w, h) = (rows[0].len(), rows.len());
        let dots = rows
            .iter()
            .flat_map(|r| {
                r.bytes().map(|b| match b {
                    b'#' => MAX,
                    b'=' => 2,
                    b'.' => 1,
                    _ => 0,
                })
            })
            .collect();
        Self { w, h, dots }
    }

    /// The same sprite with no dot brighter than `max`.
    pub fn capped(&self, max: u8) -> Self {
        Self { w: self.w, h: self.h, dots: self.dots.iter().map(|&v| v.min(max)).collect() }
    }

    /// Load any image, shrink it to fit inside max_w x max_h dots (keeping
    /// the aspect ratio), and quantize it to the DMD shades.
    pub fn load(path: &Path, max_w: usize, max_h: usize) -> Result<Self> {
        Ok(Self::fit(open_luma(path)?, max_w, max_h))
    }

    /// Like [`Sprite::load`], from the bytes of an image file.
    pub fn from_image_bytes(bytes: &[u8], max_w: usize, max_h: usize) -> Result<Self> {
        Ok(Self::fit(image::load_from_memory(bytes).context("decoding image")?.to_luma8(), max_w, max_h))
    }

    fn fit(img: image::GrayImage, max_w: usize, max_h: usize) -> Self {
        let scale = (max_w as f32 / img.width() as f32).min(max_h as f32 / img.height() as f32);
        let w = ((img.width() as f32 * scale).round() as u32).clamp(1, max_w as u32);
        let h = ((img.height() as f32 * scale).round() as u32).clamp(1, max_h as u32);
        let small = imageops::resize(&img, w, h, FilterType::Triangle);
        let dots = small.pixels().map(|p| shade(p.0[0] as f32 / 255.0)).collect();
        Self { w: w as usize, h: h as usize, dots }
    }

    /// Load pen-and-ink art drawn at [`ART_SCALE`] px per dot. Each block
    /// of pixels becomes one dot, shaded by how much ink covers it. This
    /// gives smooth, anti-aliased shapes (see `tools/big_digits.py`).
    pub fn load_art(path: &Path) -> Result<Self> {
        Ok(Self::art(open_luma(path)?))
    }

    /// Like [`Sprite::load_art`], from the bytes of a PNG. Use it with
    /// `include_bytes!` so a scene carries its own art.
    pub fn art_from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self::art(image::load_from_memory(bytes).context("decoding art")?.to_luma8()))
    }

    fn art(img: image::GrayImage) -> Self {
        let (w, h) = (img.width() as usize / ART_SCALE, img.height() as usize / ART_SCALE);
        let mut dots = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0u32;
                for py in 0..ART_SCALE {
                    for px in 0..ART_SCALE {
                        sum += img.get_pixel((x * ART_SCALE + px) as u32, (y * ART_SCALE + py) as u32).0[0] as u32;
                    }
                }
                let ink = sum as f32 / (255 * ART_SCALE * ART_SCALE) as f32;
                dots.push(shade(ink * INK_BOOST));
            }
        }
        Self { w, h, dots }
    }

    /// Load every PNG in a folder, in name order, as animation frames.
    pub fn load_art_dir(dir: &Path) -> Result<Vec<Self>> {
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .with_context(|| format!("reading {}", dir.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "png"))
            .collect();
        paths.sort();
        if paths.is_empty() {
            bail!("no PNG frames in {}", dir.display());
        }
        paths.iter().map(|p| Self::load_art(p)).collect()
    }
}

/// Pixels per dot in generated art.
pub const ART_SCALE: usize = 8;
/// Thin pen lines cover little of a dot. Boost them so they still show.
const INK_BOOST: f32 = 1.4;

fn open_luma(path: &Path) -> Result<image::GrayImage> {
    Ok(image::open(path).with_context(|| format!("opening {}", path.display()))?.to_luma8())
}

/// Brightness 0..1 -> nearest DMD shade.
pub fn shade(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * MAX as f32).round() as u8
}

/// Plasma dots light up at once but take a moment to go dark.
/// This keeps a brightness (0..1) per dot that trails the frame.
pub struct Phosphor {
    level: Vec<f32>,
    bytes: Vec<u8>,
    /// How long a lit dot takes to fade to about a third.
    pub decay_seconds: f32,
}

impl Phosphor {
    pub const DEFAULT_DECAY: f32 = 0.05;

    pub fn new(size: Size) -> Self {
        Self { level: vec![0.0; size.dots()], bytes: vec![0; size.dots()], decay_seconds: Self::DEFAULT_DECAY }
    }

    pub fn update(&mut self, frame: &Frame, dt: f32) {
        let keep = (-dt / self.decay_seconds).exp();
        for (l, &d) in self.level.iter_mut().zip(&frame.dots) {
            let target = d as f32 / MAX as f32;
            *l = if target >= *l { target } else { target.max(*l * keep) };
        }
    }

    /// Brightness per dot, 0..=255, row-major, for the renderer.
    pub fn bytes(&mut self) -> &[u8] {
        for (b, l) in self.bytes.iter_mut().zip(&self.level) {
            *b = (l * 255.0).round() as u8;
        }
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dots_off_the_grid_are_ignored() {
        let mut f = Frame::new(Size::new(4, 2));
        f.set(-1, 0, 3);
        f.set(4, 0, 3);
        f.set(0, 2, 3);
        assert!(f.dots.iter().all(|&d| d == 0));
        f.set(3, 1, 9);
        assert_eq!(f.get(3, 1), MAX);
        assert_eq!(f.get(3, 2), 0);
    }

    #[test]
    fn sprites_blit_with_transparency_and_clip() {
        let mut f = Frame::new(Size::new(4, 2));
        f.fill(f.rect(), 1);
        let s = Sprite::from_rows(&["# ", " ="]);
        f.blit_clip(&s, 0, 0, Rect::new(0, 0, 1, 2));
        assert_eq!(f.get(0, 0), MAX);
        assert_eq!(f.get(1, 0), 1); // transparent
        assert_eq!(f.get(1, 1), 1); // clipped
    }

    #[test]
    fn phosphor_fades_but_lights_at_once() {
        let size = Size::new(1, 1);
        let mut p = Phosphor::new(size);
        let mut on = Frame::new(size);
        on.set(0, 0, MAX);
        p.update(&on, 0.001);
        assert_eq!(p.bytes()[0], 255);
        let off = Frame::new(size);
        p.update(&off, Phosphor::DEFAULT_DECAY);
        let b = p.bytes()[0];
        assert!(b > 80 && b < 110, "one decay constant later: {b}");
    }
}
