//! A pinball-style clock: big smooth digits that roll when they change,
//! a blinking colon, and the date.

use anyhow::Result;
use chrono::Timelike;
use glowdots::font::{REGULAR, SMALL};
use glowdots::math::ease_in_out;
use glowdots::{Context, Frame, Rect, Scene, Size, Sprite, Transition};

const DIGIT_W: i32 = 10;
const COLON_W: i32 = 4;
const GAP: i32 = 2;
const TOP: i32 = 2;
const ROLL_SECONDS: f32 = 0.35;

/// The digits, drawn by tools/big_digits.py at 8 px per dot.
const DIGITS: [&[u8]; 10] = [
    include_bytes!("../assets/fonts/digits/0.png"),
    include_bytes!("../assets/fonts/digits/1.png"),
    include_bytes!("../assets/fonts/digits/2.png"),
    include_bytes!("../assets/fonts/digits/3.png"),
    include_bytes!("../assets/fonts/digits/4.png"),
    include_bytes!("../assets/fonts/digits/5.png"),
    include_bytes!("../assets/fonts/digits/6.png"),
    include_bytes!("../assets/fonts/digits/7.png"),
    include_bytes!("../assets/fonts/digits/8.png"),
    include_bytes!("../assets/fonts/digits/9.png"),
];
const COLON: &[u8] = include_bytes!("../assets/fonts/digits/colon.png");

pub struct Clock {
    digits: Vec<Sprite>,
    colon: Sprite,
    dim_colon: Sprite,
    /// The time text now, and for each character: the one before and when it changed.
    text: Vec<char>,
    rolls: Vec<Option<(char, f32)>>,
}

impl Clock {
    pub fn new() -> Result<Self> {
        let digits = DIGITS.iter().map(|png| Sprite::art_from_bytes(png)).collect::<Result<_>>()?;
        let colon = Sprite::art_from_bytes(COLON)?;
        Ok(Self { digits, dim_colon: colon.capped(1), colon, text: Vec::new(), rolls: Vec::new() })
    }

    fn glyph(&self, c: char) -> &Sprite {
        match c.to_digit(10) {
            Some(d) => &self.digits[d as usize],
            None => &self.colon,
        }
    }

    fn width(c: char) -> i32 {
        if c == ':' { COLON_W } else { DIGIT_W }
    }
}

impl Scene for Clock {
    fn name(&self) -> &str {
        "clock"
    }

    fn enter(&mut self, _cx: &Context) {
        self.text.clear();
    }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        let (t, local) = (cx.t, cx.now);
        let text: Vec<char> = local.format("%-I:%M").to_string().chars().collect();

        // Note which digits changed, so they can roll.
        if text.len() != self.text.len() {
            self.rolls = vec![None; text.len()];
        } else {
            for (i, (&new, &old)) in text.iter().zip(&self.text).enumerate() {
                if new != old {
                    self.rolls[i] = Some((old, t));
                }
            }
        }
        self.text = text;

        // Layout: the digits, then a narrow column with seconds and AM/PM.
        let ampm = local.format("%p").to_string();
        let secs = local.format("%S").to_string();
        let column = SMALL.width(&ampm).max(SMALL.width(&secs));
        let digits_w: i32 = self.text.iter().map(|&c| Self::width(c)).sum::<i32>() + GAP * (self.text.len() as i32 - 1);
        let mut x = (out.w() - (digits_w + 3 + column)) / 2;

        let blink_on = local.nanosecond() < 500_000_000;
        for (i, &c) in self.text.iter().enumerate() {
            let w = Self::width(c);
            if c == ':' {
                out.blit(if blink_on { &self.colon } else { &self.dim_colon }, x, TOP);
            } else {
                let clip = Rect::new(x, TOP, x + w, TOP + 18);
                match self.rolls[i] {
                    Some((old, start)) if t - start < ROLL_SECONDS => {
                        // The old digit drops out; the new one drops in.
                        let p = ease_in_out((t - start) / ROLL_SECONDS);
                        let dy = (p * 19.0).round() as i32;
                        out.blit_clip(self.glyph(old), x, TOP + dy, clip);
                        out.blit_clip(self.glyph(c), x, TOP + dy - 19, clip);
                    }
                    _ => out.blit_clip(self.glyph(c), x, TOP, clip),
                }
            }
            x += w + GAP;
        }
        let col_x = x - GAP + 3;
        SMALL.draw(out, &secs, col_x, TOP + 1, 2);
        SMALL.draw(out, &ampm, col_x, TOP + 13, 2);

        let date = local.format("%a %b %-d").to_string().to_uppercase();
        REGULAR.draw_centered(out, &date, 24, 2);
    }

    fn duration(&self) -> f32 {
        12.0
    }

    fn transition(&self) -> Transition {
        Transition::Dissolve
    }

    fn size(&self) -> Option<Size> {
        Some(crate::SIZE)
    }
}
