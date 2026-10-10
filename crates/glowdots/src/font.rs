//! Pixel fonts in the style of 1990s pinball displays: `SMALL` (3x5) and
//! `REGULAR` (5x7).
//!
//! Glyphs are drawn as rows of '#' (lit) and '.' (off). They are
//! proportional: empty columns at the sides are trimmed.

use crate::frame::{Frame, Rect};

/// A pixel font. Each glyph is rows of '#' (lit) and '.' (off), all rows
/// the same length; empty columns at the sides are trimmed, so the font is
/// proportional. A character with no glyph is drawn as a space.
///
/// ```
/// use glowdots::font::Font;
/// static TINY: Font = Font::new(1, &[
///     ('I', &["#", "#", "#"]),
///     ('O', &["###", "#.#", "###"]),
/// ]);
/// assert_eq!(TINY.width("OI"), 5);
/// ```
pub struct Font {
    glyphs: &'static [(char, &'static [&'static str])],
    space: i32,
}

/// 3x5: labels, times, and small print.
#[rustfmt::skip]
pub static SMALL: Font = Font { space: 2, glyphs: &[
    ('A', &[".#.", "#.#", "###", "#.#", "#.#"]),
    ('B', &["##.", "#.#", "##.", "#.#", "##."]),
    ('C', &[".##", "#..", "#..", "#..", ".##"]),
    ('D', &["##.", "#.#", "#.#", "#.#", "##."]),
    ('E', &["###", "#..", "##.", "#..", "###"]),
    ('F', &["###", "#..", "##.", "#..", "#.."]),
    ('G', &[".##", "#..", "#.#", "#.#", ".##"]),
    ('H', &["#.#", "#.#", "###", "#.#", "#.#"]),
    ('I', &["###", ".#.", ".#.", ".#.", "###"]),
    ('J', &["..#", "..#", "..#", "#.#", ".#."]),
    ('K', &["#.#", "#.#", "##.", "#.#", "#.#"]),
    ('L', &["#..", "#..", "#..", "#..", "###"]),
    ('M', &["#...#", "##.##", "#.#.#", "#...#", "#...#"]),
    ('N', &["#..#", "##.#", "#.##", "#..#", "#..#"]),
    ('O', &[".#.", "#.#", "#.#", "#.#", ".#."]),
    ('P', &["##.", "#.#", "##.", "#..", "#.."]),
    ('Q', &[".#.", "#.#", "#.#", "##.", ".##"]),
    ('R', &["##.", "#.#", "##.", "#.#", "#.#"]),
    ('S', &[".##", "#..", ".#.", "..#", "##."]),
    ('T', &["###", ".#.", ".#.", ".#.", ".#."]),
    ('U', &["#.#", "#.#", "#.#", "#.#", "###"]),
    ('V', &["#.#", "#.#", "#.#", "#.#", ".#."]),
    ('W', &["#...#", "#...#", "#.#.#", "##.##", "#...#"]),
    ('X', &["#.#", "#.#", ".#.", "#.#", "#.#"]),
    ('Y', &["#.#", "#.#", ".#.", ".#.", ".#."]),
    ('Z', &["###", "..#", ".#.", "#..", "###"]),
    ('0', &["###", "#.#", "#.#", "#.#", "###"]),
    ('1', &[".#.", "##.", ".#.", ".#.", "###"]),
    ('2', &["##.", "..#", ".#.", "#..", "###"]),
    ('3', &["##.", "..#", ".#.", "..#", "##."]),
    ('4', &["#.#", "#.#", "###", "..#", "..#"]),
    ('5', &["###", "#..", "##.", "..#", "##."]),
    ('6', &[".##", "#..", "###", "#.#", "###"]),
    ('7', &["###", "..#", ".#.", ".#.", ".#."]),
    ('8', &["###", "#.#", "###", "#.#", "###"]),
    ('9', &["###", "#.#", "###", "..#", "##."]),
    (':', &[".", "#", ".", "#", "."]),
    ('.', &[".", ".", ".", ".", "#"]),
    (',', &["..", "..", "..", ".#", "#."]),
    ('\'', &["#", "#", ".", ".", "."]),
    ('!', &["#", "#", "#", ".", "#"]),
    ('?', &["##.", "..#", ".#.", "...", ".#."]),
    ('-', &["...", "...", "###", "...", "..."]),
    ('+', &["...", ".#.", "###", ".#.", "..."]),
    ('/', &["..#", "..#", ".#.", "#..", "#.."]),
    ('&', &[".#.", "#.#", ".#.", "#.#", ".##"]),
    ('@', &[".##.", "#..#", "#.##", "#...", ".##."]),
    ('#', &["#.#", "###", "#.#", "###", "#.#"]),
    ('(', &[".#", "#.", "#.", "#.", ".#"]),
    (')', &["#.", ".#", ".#", ".#", "#."]),
    ('$', &[".##", "##.", ".#.", ".##", "##."]),
    ('%', &["#.#", "..#", ".#.", "#..", "#.#"]),
    ('·', &[".", ".", "#", ".", "."]),
    ('°', &[".#.", "#.#", ".#.", "...", "..."]),
]};

/// 5x7: titles and names.
#[rustfmt::skip]
pub static REGULAR: Font = Font { space: 3, glyphs: &[
    ('A', &[".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"]),
    ('B', &["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."]),
    ('C', &[".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."]),
    ('D', &["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."]),
    ('E', &["#####", "#....", "#....", "####.", "#....", "#....", "#####"]),
    ('F', &["#####", "#....", "#....", "####.", "#....", "#....", "#...."]),
    ('G', &[".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####"]),
    ('H', &["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"]),
    ('I', &["###", ".#.", ".#.", ".#.", ".#.", ".#.", "###"]),
    ('J', &["..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."]),
    ('K', &["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"]),
    ('L', &["#....", "#....", "#....", "#....", "#....", "#....", "#####"]),
    ('M', &["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"]),
    ('N', &["#...#", "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#"]),
    ('O', &[".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."]),
    ('P', &["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."]),
    ('Q', &[".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"]),
    ('R', &["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"]),
    ('S', &[".####", "#....", "#....", ".###.", "....#", "....#", "####."]),
    ('T', &["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."]),
    ('U', &["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."]),
    ('V', &["#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#.."]),
    ('W', &["#...#", "#...#", "#...#", "#.#.#", "#.#.#", "#.#.#", ".#.#."]),
    ('X', &["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"]),
    ('Y', &["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."]),
    ('Z', &["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"]),
    ('0', &[".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###."]),
    ('1', &["..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###."]),
    ('2', &[".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"]),
    ('3', &["####.", "....#", "....#", ".###.", "....#", "....#", "####."]),
    ('4', &["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."]),
    ('5', &["#####", "#....", "####.", "....#", "....#", "#...#", ".###."]),
    ('6', &[".###.", "#....", "#....", "####.", "#...#", "#...#", ".###."]),
    ('7', &["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."]),
    ('8', &[".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."]),
    ('9', &[".###.", "#...#", "#...#", ".####", "....#", "....#", ".###."]),
    ('!', &["#", "#", "#", "#", "#", ".", "#"]),
    ('?', &[".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#.."]),
    ('.', &[".", ".", ".", ".", ".", ".", "#"]),
    (',', &["..", "..", "..", "..", "..", ".#", "#."]),
    ('\'', &["#", "#", ".", ".", ".", ".", "."]),
    ('"', &["#.#", "#.#", "...", "...", "...", "...", "..."]),
    ('-', &["....", "....", "....", "####", "....", "....", "...."]),
    (':', &[".", ".", "#", ".", ".", "#", "."]),
    ('/', &["....#", "...#.", "...#.", "..#..", ".#...", ".#...", "#...."]),
    ('&', &[".##..", "#..#.", "#.#..", ".#...", "#.#.#", "#..#.", ".##.#"]),
    ('@', &[".###.", "#...#", "#.###", "#.#.#", "#.###", "#....", ".###."]),
    ('#', &[".#.#.", ".#.#.", "#####", ".#.#.", "#####", ".#.#.", ".#.#."]),
    ('+', &[".....", "..#..", "..#..", "#####", "..#..", "..#..", "....."]),
    ('(', &[".#", "#.", "#.", "#.", "#.", "#.", ".#"]),
    (')', &["#.", ".#", ".#", ".#", ".#", ".#", "#."]),
    ('$', &["..#..", ".####", "#.#..", ".###.", "..#.#", "####.", "..#.."]),
    ('%', &["##..#", "##..#", "...#.", "..#..", ".#...", "#..##", "#..##"]),
    ('·', &[".", ".", ".", "#", ".", ".", "."]),
    ('°', &[".#.", "#.#", ".#.", "...", "...", "...", "..."]),
]};

impl Font {
    /// A font from its glyphs, and the width of a space in dots.
    pub const fn new(space: i32, glyphs: &'static [(char, &'static [&'static str])]) -> Self {
        Self { glyphs, space }
    }

    /// Height of the tallest glyph, in dots.
    pub fn height(&self) -> i32 {
        self.glyphs.iter().map(|(_, rows)| rows.len() as i32).max().unwrap_or(0)
    }

    /// A glyph's rows, its first lit column, and its width.
    fn glyph(&self, c: char) -> Option<(&'static [&'static str], usize, usize)> {
        let rows = self.glyphs.iter().find(|(g, _)| *g == c)?.1;
        let cols = rows[0].len();
        let used = |col: usize| rows.iter().any(|r| r.as_bytes()[col] == b'#');
        let first = (0..cols).find(|&c| used(c))?;
        let last = (0..cols).rev().find(|&c| used(c))?;
        Some((rows, first, last - first + 1))
    }

    fn advance(&self, c: char) -> i32 {
        self.glyph(c).map_or(self.space, |g| g.2 as i32)
    }

    /// Width of one character in dots, without the gap after it.
    pub fn char_width(&self, c: char) -> i32 {
        self.advance(c)
    }

    /// Break text into lines no wider than `width` dots, at spaces or
    /// after a '/'. A word too long for a line is split.
    pub fn wrap(&self, text: &str, width: i32) -> Vec<String> {
        // Pieces of text, and whether a space comes before each one.
        let mut pieces: Vec<(String, bool)> = Vec::new();
        for word in text.split_whitespace() {
            for (i, part) in word.split_inclusive('/').enumerate() {
                pieces.push((part.to_string(), i == 0));
            }
        }
        let mut lines: Vec<String> = Vec::new();
        let mut line = String::new();
        for (piece, spaced) in pieces {
            let joined = if line.is_empty() {
                piece.clone()
            } else if spaced {
                format!("{line} {piece}")
            } else {
                format!("{line}{piece}")
            };
            if self.width(&joined) <= width {
                line = joined;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            let mut rest = piece;
            while self.width(&rest) > width {
                let mut cut = rest.len();
                while cut > 1 && self.width(&rest[..cut]) > width {
                    cut = rest[..cut].char_indices().last().map_or(1, |(i, _)| i.max(1));
                }
                lines.push(rest[..cut].to_string());
                rest = rest[cut..].to_string();
            }
            line = rest;
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }

    /// Width of the text in dots (one-dot gap between letters).
    pub fn width(&self, text: &str) -> i32 {
        let n = text.chars().count() as i32;
        text.chars().map(|c| self.advance(c)).sum::<i32>() + (n - 1).max(0)
    }

    /// Draw text with its top-left at (x, y).
    pub fn draw(&self, f: &mut Frame, text: &str, x: i32, y: i32, level: u8) {
        let clip = f.rect();
        self.draw_clip(f, text, x, y, level, clip);
    }

    /// Draw text, but only inside `clip`.
    pub fn draw_clip(&self, f: &mut Frame, text: &str, x: i32, y: i32, level: u8, clip: Rect) {
        let mut cx = x;
        for c in text.chars() {
            if let Some((rows, first, w)) = self.glyph(c) {
                for (ry, row) in rows.iter().enumerate() {
                    for col in 0..w {
                        let (px, py) = (cx + col as i32, y + ry as i32);
                        if row.as_bytes()[first + col] == b'#' && clip.contains(px, py) {
                            f.set(px, py, level);
                        }
                    }
                }
            }
            cx += self.advance(c) + 1;
        }
    }

    /// The text as columns of dots, left to right, with the one-dot gaps.
    /// Bit 0 is the top row. A sine scroller moves each column on its own.
    pub fn columns(&self, text: &str) -> Vec<u16> {
        let mut cols = Vec::new();
        for c in text.chars() {
            match self.glyph(c) {
                Some((rows, first, w)) => cols.extend(
                    (first..first + w)
                        .map(|col| rows.iter().enumerate().filter(|(_, r)| r.as_bytes()[col] == b'#').fold(0u16, |m, (y, _)| m | 1 << y)),
                ),
                None => cols.extend(std::iter::repeat_n(0, self.advance(c) as usize)),
            }
            cols.push(0);
        }
        cols
    }

    pub fn draw_right(&self, f: &mut Frame, text: &str, right: i32, y: i32, level: u8) {
        self.draw(f, text, right - self.width(text), y, level);
    }

    pub fn draw_centered(&self, f: &mut Frame, text: &str, y: i32, level: u8) {
        self.draw(f, text, (f.w() - self.width(text)) / 2, y, level);
    }
}

/// Make text safe for the fonts: upper case, and plain ASCII stand-ins
/// for curly quotes, dashes, and accents.
pub fn clean(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '‘' | '’' | '`' | '´' => '\'',
            '“' | '”' => '"',
            '–' | '—' | '−' => '-',
            '…' => '.',
            '•' => '·',
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'Á' | 'À' | 'Â' | 'Ä' => 'A',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' => 'E',
            'í' | 'ì' | 'î' | 'ï' => 'I',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'Ö' => 'O',
            'ú' | 'ù' | 'û' | 'ü' | 'Ü' => 'U',
            'ñ' | 'Ñ' => 'N',
            'ç' => 'C',
            c => c.to_ascii_uppercase(),
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
