//! Looks: shades, palettes, colours of their own, and glass.
//!
//! ```sh
//! cargo run --example looks              # classic plasma glass
//! cargo run --example looks -- led       # an LED panel
//! cargo run --example looks -- worn      # a tired plasma panel
//! ```
//!
//! The panel has 16 shades. The first scene still draws with the classic
//! 4, and the deck spreads them over the 16.

use glowdots::font::{REGULAR, SMALL};
use glowdots::{Context, Deck, Frame, Glass, Look, Player, Rect, Rgb, Scene, Size, Transition, art};

/// Rolling waves, as smooth as the scene's shades allow, under a title.
struct Waves {
    name: &'static str,
    title: &'static str,
    look: Look,
    shades: u8,
}

impl Scene for Waves {
    fn name(&self) -> &str {
        self.name
    }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        for y in 0..out.h() {
            for x in 0..out.w() {
                let (fx, fy) = (x as f32, y as f32);
                let v = 0.5 + 0.25 * (fx * 0.09 + cx.t * 1.3).sin() + 0.25 * (fx * 0.05 - fy * 0.2 + cx.t * 0.7).sin();
                // `level` turns a brightness 0..1 into this frame's shades.
                out.set(x, y, out.level(v * 0.75));
            }
        }
        let mut title = out.blank();
        let bright = title.max();
        REGULAR.draw_centered(&mut title, self.title, 12, bright);
        art::overlay_outlined(out, &title);
    }

    fn duration(&self) -> f32 {
        7.0
    }

    fn transition(&self) -> Transition {
        Transition::Dissolve
    }

    fn look(&self) -> Look {
        self.look
    }

    fn shades(&self) -> u8 {
        self.shades
    }
}

/// Dots with colours of their own: draw, then tint; or set a colour per dot.
struct Colours;

impl Scene for Colours {
    fn name(&self) -> &str {
        "colours"
    }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        let bright = out.max();
        let mut x = 16;
        for (word, color) in [("RED", 0xff2010), ("GREEN", 0x30ff40), ("BLUE", 0x3070ff)] {
            REGULAR.draw(out, word, x, 4, bright);
            let w = REGULAR.width(word);
            out.tint(Rect::new(x, 4, x + w, 11), Rgb::hex(color));
            x += w + 9;
        }
        for x in 0..out.w() {
            let hue = (x as f32 / out.w() as f32 + cx.t * 0.25).fract();
            for y in 15..21 {
                out.set_rgb(x, y, bright, rainbow(hue));
            }
        }
        SMALL.draw_centered(out, "ANY DOT, ANY COLOUR", 25, bright);
    }

    fn duration(&self) -> f32 {
        7.0
    }

    fn transition(&self) -> Transition {
        Transition::Ink
    }

    fn shades(&self) -> u8 {
        16
    }
}

/// A fully saturated colour around the colour wheel (hue 0..1).
fn rainbow(hue: f32) -> Rgb {
    let c = |shift: f32| {
        let k = (hue * 6.0 + shift) % 6.0;
        (255.0 * (1.0 - (k.min(4.0 - k)).clamp(0.0, 1.0))) as u8
    };
    Rgb(c(5.0), c(3.0), c(1.0))
}

fn main() -> anyhow::Result<()> {
    let glass = match std::env::args().nth(1).as_deref() {
        Some("led") => Glass::LED,
        Some("worn") => Glass::WORN,
        _ => Glass::CLASSIC,
    };
    let scenes: Vec<Box<dyn Scene>> = vec![
        Box::new(Waves { name: "classic", title: "4 SHADES", look: Look::WILLIAMS, shades: 4 }),
        Box::new(Waves { name: "heat", title: "16 SHADES", look: Look::heat(), shades: 16 }),
        Box::new(Colours),
        Box::new(Waves { name: "ocean", title: "PALETTES", look: Look::ocean(), shades: 16 }),
    ];
    let deck = Deck::new(Size::WPC, scenes)?.with_shades(16);
    glowdots::app::run(Player::new(deck).with_glass(glass), Default::default())
}
