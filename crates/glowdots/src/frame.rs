//! The DMD frame: a grid of dots, each at one of a few shades.
//!
//! The Williams WPC games drove a 128x32 plasma panel with 4 shades: off
//! and 3 lit levels. That is the classic frame here, and what [`MAX`]
//! means. A frame can also have more shades (up to 16, like later Stern
//! panels), and its dots can carry colours of their own.

use anyhow::{Context as _, Result, bail};
use image::imageops::{self, FilterType};
use std::path::Path;

/// The brightest shade of a classic 4-shade frame.
pub const MAX: u8 = 3;
/// Shades in a classic frame: off and 3 lit levels.
pub const CLASSIC_SHADES: u8 = 4;
/// The most shades a frame can have.
pub const MAX_SHADES: u8 = 16;

/// The panel size, in dots.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

impl Size {
    /// The Williams WPC panel, also used by Bally, Data East, and Stern: 128x32.
    pub const WPC: Size = Size::new(128, 32);
    /// The larger display on a run of late-1990s Sega games: 192x64.
    pub const SEGA_WIDE: Size = Size::new(192, 64);

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

    /// Whether this size fits inside `other`.
    pub const fn fits_in(self, other: Size) -> bool {
        self.w <= other.w && self.h <= other.h
    }
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.w, self.h)
    }
}

/// A rectangle of dots: x0..x1, y0..y1 (the end is not included).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
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

/// A dot colour, as on screen (0..255 per channel).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// No colour of its own: the dot takes the scene's [`Look`](crate::Look).
    pub const NONE: Rgb = Rgb(0, 0, 0);

    /// A colour from a web hex number, like `Rgb::hex(0xff4205)`.
    pub const fn hex(v: u32) -> Rgb {
        Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    pub const fn is_none(self) -> bool {
        self.0 == 0 && self.1 == 0 && self.2 == 0
    }

    /// The colour as 0..1 floats.
    pub fn to_f32(self) -> [f32; 3] {
        [self.0 as f32 / 255.0, self.1 as f32 / 255.0, self.2 as f32 / 255.0]
    }
}

/// The dots of one picture: a shade per dot, row-major, top row first,
/// and optionally a colour per dot.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Frame {
    size: Size,
    max: u8,
    /// The shade of each dot, 0 (off) ..= [`Frame::max`].
    pub dots: Vec<u8>,
    /// Each dot's own colour, made the first time a dot gets one.
    colors: Option<Vec<Rgb>>,
}

impl Frame {
    /// A classic frame: 4 shades.
    pub fn new(size: Size) -> Self {
        Self::with_shades(size, CLASSIC_SHADES)
    }

    /// A frame with `shades` shades (2..=16), counting off.
    pub fn with_shades(size: Size, shades: u8) -> Self {
        let shades = shades.clamp(2, MAX_SHADES);
        Self { size, max: shades - 1, dots: vec![0; size.dots()], colors: None }
    }

    /// An empty frame of the same size and shades, for layers.
    pub fn blank(&self) -> Self {
        Self::with_shades(self.size, self.shades())
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

    /// The brightest shade.
    pub fn max(&self) -> u8 {
        self.max
    }

    /// How many shades, counting off.
    pub fn shades(&self) -> u8 {
        self.max + 1
    }

    /// A classic shade (0..=[`MAX`]) in this frame's shades. In a classic
    /// frame it is the same number. Helpers use it so they look the same
    /// on any frame.
    pub fn classic(&self, level: u8) -> u8 {
        rescale(level.min(MAX), MAX, self.max)
    }

    /// A brightness 0..1 as the nearest shade of this frame.
    pub fn level(&self, v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * self.max as f32).round() as u8
    }

    pub fn clear(&mut self) {
        self.dots.fill(0);
        self.colors = None;
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w() && y < self.h()).then(|| y as usize * self.size.w as usize + x as usize)
    }

    fn colors_mut(&mut self) -> &mut Vec<Rgb> {
        let n = self.dots.len();
        self.colors.get_or_insert_with(|| vec![Rgb::NONE; n])
    }

    pub(crate) fn color_at(&self, i: usize) -> Rgb {
        self.colors.as_ref().map_or(Rgb::NONE, |c| c[i])
    }

    /// The shade at (x, y). Dots off the grid are 0.
    pub fn get(&self, x: i32, y: i32) -> u8 {
        self.index(x, y).map_or(0, |i| self.dots[i])
    }

    /// Set a dot's shade. It takes the scene's look (any colour of its own
    /// is cleared). Dots off the grid are ignored.
    pub fn set(&mut self, x: i32, y: i32, v: u8) {
        if let Some(i) = self.index(x, y) {
            self.dots[i] = v.min(self.max);
            if let Some(c) = &mut self.colors {
                c[i] = Rgb::NONE;
            }
        }
    }

    /// Set a dot's shade and give it a colour of its own.
    pub fn set_rgb(&mut self, x: i32, y: i32, v: u8, color: Rgb) {
        if let Some(i) = self.index(x, y) {
            self.dots[i] = v.min(self.max);
            self.colors_mut()[i] = color;
        }
    }

    /// The dot's own colour, if it has one.
    pub fn color(&self, x: i32, y: i32) -> Option<Rgb> {
        let c = self.color_at(self.index(x, y)?);
        (!c.is_none()).then_some(c)
    }

    /// Give the lit dots inside `r` a colour of their own. Draw first, then
    /// tint, the way colour kits colourise a classic display.
    pub fn tint(&mut self, r: Rect, color: Rgb) {
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                if let Some(i) = self.index(x, y) {
                    if self.dots[i] > 0 {
                        self.colors_mut()[i] = color;
                    }
                }
            }
        }
    }

    /// Copy one dot (shade and colour) from another frame, with its shade
    /// rescaled to this frame's shades.
    pub fn copy_dot(&mut self, from: &Frame, fx: i32, fy: i32, x: i32, y: i32) {
        let Some(i) = self.index(x, y) else { return };
        self.dots[i] = rescale(from.get(fx, fy), from.max, self.max);
        let c = from.index(fx, fy).map_or(Rgb::NONE, |j| from.color_at(j));
        if !c.is_none() || self.colors.is_some() {
            self.colors_mut()[i] = c;
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
                    self.set(px, py, rescale(v, s.max, self.max));
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
                self.copy_dot(from, x, y - dy, x, y);
            }
        }
    }

    /// Draw all of `src` (any size and shades) with its top-left at (x, y).
    /// Its off dots are drawn too.
    pub fn draw_frame(&mut self, src: &Frame, x: i32, y: i32) {
        for sy in 0..src.h() {
            for sx in 0..src.w() {
                self.copy_dot(src, sx, sy, x + sx, y + sy);
            }
        }
    }

    /// A still image, fitted and centered, in classic shades.
    pub fn from_image(path: &Path, size: Size) -> Result<Self> {
        let s = Sprite::load(path, size.w as usize, size.h as usize)?;
        let mut f = Self::new(size);
        f.blit(&s, (f.w() - s.w as i32) / 2, (f.h() - s.h as i32) / 2);
        Ok(f)
    }
}

/// A shade out of `from` levels as the nearest shade out of `to`.
fn rescale(v: u8, from: u8, to: u8) -> u8 {
    if from == to || from == 0 {
        return v.min(to);
    }
    ((v.min(from) as u32 * to as u32 * 2 + from as u32) / (2 * from as u32)) as u8
}

/// A small picture made of shades. Sprites keep their own number of
/// shades and are rescaled when drawn into a frame with another number.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub dots: Vec<u8>,
    /// The brightest shade the dots use ([`MAX`] for a classic sprite).
    pub max: u8,
}

impl Sprite {
    /// A classic sprite from a dot map: '#' full, '=' mid, '.' dim,
    /// anything else off.
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
        Self { w, h, dots, max: MAX }
    }

    /// The same sprite with no dot brighter than `max`.
    pub fn capped(&self, max: u8) -> Self {
        Self { dots: self.dots.iter().map(|&v| v.min(max)).collect(), ..self.clone() }
    }

    /// Load any image, shrink it to fit inside max_w x max_h dots (keeping
    /// the aspect ratio), and quantize it to classic shades.
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
        Self { w: w as usize, h: h as usize, dots, max: MAX }
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
        Self { w, h, dots, max: MAX }
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

/// Brightness 0..1 -> nearest classic shade.
pub fn shade(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * MAX as f32).round() as u8
}

/// Plasma dots light up at once but take a moment to go dark. This keeps
/// a brightness (0..1) per dot that trails the frame, and the colour each
/// dot had when it was last lit, so it fades in that colour.
pub struct Phosphor {
    level: Vec<f32>,
    color: Vec<Rgb>,
    rgba: Vec<u8>,
    /// How long a lit dot takes to fade to about a third.
    pub decay_seconds: f32,
}

impl Phosphor {
    pub const DEFAULT_DECAY: f32 = 0.05;

    pub fn new(size: Size) -> Self {
        let n = size.dots();
        Self { level: vec![0.0; n], color: vec![Rgb::NONE; n], rgba: vec![0; n * 4], decay_seconds: Self::DEFAULT_DECAY }
    }

    pub fn update(&mut self, frame: &Frame, dt: f32) {
        let keep = (-dt / self.decay_seconds.max(1e-4)).exp();
        let max = frame.max() as f32;
        for (i, (l, &d)) in self.level.iter_mut().zip(&frame.dots).enumerate() {
            let target = d as f32 / max;
            *l = if target >= *l { target } else { target.max(*l * keep) };
            if d > 0 {
                self.color[i] = frame.color_at(i);
            }
        }
    }

    /// Brightness of each dot, 0..1.
    pub fn levels(&self) -> &[f32] {
        &self.level
    }

    /// For the renderer: per dot, its own colour (0, 0, 0 for the look's)
    /// and its brightness, as RGBA bytes, row-major.
    pub fn rgba(&mut self) -> &[u8] {
        for (i, px) in self.rgba.chunks_exact_mut(4).enumerate() {
            let c = self.color[i];
            px.copy_from_slice(&[c.0, c.1, c.2, (self.level[i] * 255.0).round() as u8]);
        }
        &self.rgba
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
    fn classic_shades_rescale_exactly() {
        let mut f = Frame::with_shades(Size::new(4, 1), 16);
        f.blit(&Sprite::from_rows(&[".=# "]), 0, 0);
        assert_eq!(&f.dots, &[5, 10, 15, 0]);
        assert_eq!((f.classic(1), f.classic(2), f.classic(MAX)), (5, 10, 15));
        let mut classic = Frame::new(Size::new(4, 1));
        classic.draw_frame(&f, 0, 0);
        assert_eq!(&classic.dots, &[1, 2, 3, 0]);
    }

    #[test]
    fn colours_follow_their_dots() {
        let mut f = Frame::new(Size::new(3, 1));
        f.set(0, 0, 3);
        f.set(1, 0, 2);
        f.tint(f.rect(), Rgb::hex(0x00ff00));
        assert_eq!(f.color(0, 0), Some(Rgb(0, 255, 0)));
        assert_eq!(f.color(2, 0), None); // off dots are not tinted
        f.set(0, 0, 3);
        assert_eq!(f.color(0, 0), None); // drawing again clears it
        let mut g = Frame::new(Size::new(3, 1));
        g.copy_dot(&f, 1, 0, 2, 0);
        assert_eq!((g.get(2, 0), g.color(2, 0)), (2, Some(Rgb(0, 255, 0))));
    }

    #[test]
    fn phosphor_fades_in_its_colour() {
        let size = Size::new(1, 1);
        let mut p = Phosphor::new(size);
        let mut on = Frame::new(size);
        on.set_rgb(0, 0, MAX, Rgb(10, 20, 30));
        p.update(&on, 0.001);
        assert_eq!(p.rgba(), &[10, 20, 30, 255]);
        p.update(&Frame::new(size), Phosphor::DEFAULT_DECAY);
        let px = p.rgba();
        assert_eq!(&px[..3], &[10, 20, 30]);
        assert!(px[3] > 80 && px[3] < 110, "one decay constant later: {}", px[3]);
    }
}
