//! Make the pictures in the README and docs from the real scenes, so they
//! always match the code:
//!
//! ```sh
//! cargo run --release --bin media            # writes docs/media/
//! ```

use anyhow::{Result, anyhow};
use glowdots::font::{REGULAR, SMALL};
use glowdots::{Clock, Context, Deck, Frame, Glass, Look, Player, Rect, Rgb, Scene, Transition, export, time};
use glowdots_scenes::{SIZE, samples, showcase};
use std::path::Path;

/// Classic glass without the shimmer, which keeps GIFs small.
const STEADY: Glass = Glass { flicker: 0.0, ..Glass::CLASSIC };

/// A deck of the showcase on the samples, at a time of day, on `scene`.
fn player(scene: &str, at: &str) -> Result<Player> {
    let clock = Clock::stepped(time::parse_local(at).map_err(|e| anyhow!(e))?);
    let mut deck = Deck::new(SIZE, showcase()?)?.with_feeds(samples()).with_clock(clock);
    deck.select(scene)?;
    deck.hold = true;
    let mut player = Player::new(deck).with_glass(STEADY);
    player.progress_line = false;
    Ok(player)
}

/// Dots with colours of their own, for the looks picture.
struct Colours;

impl Scene for Colours {
    fn name(&self) -> &str {
        "colours"
    }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        let bright = out.max();
        let mut x = 6;
        for (word, color) in [("RED", 0xff2010), ("GREEN", 0x30ff40), ("BLUE", 0x3070ff)] {
            REGULAR.draw(out, word, x, 3, bright);
            let w = REGULAR.width(word);
            out.tint(Rect::new(x, 3, x + w, 10), Rgb::hex(color));
            x += w + 6;
        }
        for x in 0..out.w() {
            let hue = (x as f32 / out.w() as f32 + cx.t * 0.25).fract() * 6.0;
            let c = |shift: f32| (255.0 * (1.0 - ((hue + shift) % 6.0).min(4.0 - (hue + shift) % 6.0).clamp(0.0, 1.0))) as u8;
            for y in 14..19 {
                out.set_rgb(x, y, bright, Rgb(c(5.0), c(3.0), c(1.0)));
            }
        }
        SMALL.draw_centered(out, "ANY DOT, ANY COLOR", 24, bright);
    }

    fn duration(&self) -> f32 {
        60.0
    }

    fn transition(&self) -> Transition {
        Transition::Cut
    }

    fn shades(&self) -> u8 {
        16
    }
}

fn main() -> Result<()> {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/media");
    std::fs::create_dir_all(&out)?;
    #[allow(clippy::too_many_arguments)]
    let gif = |r: &mut glowdots::render::Renderer,
               name: &str,
               scene: &str,
               at: &str,
               from: f32,
               seconds: f32,
               fps: u32,
               size: (u32, u32)|
     -> Result<()> {
        r.resize(size)?;
        let mut p = player(scene, at)?;
        p.skip(from);
        let path = out.join(format!("{name}.gif"));
        export::gif(&mut p, r, seconds, fps, &path)?;
        eprintln!("{} ({} KB)", path.display(), std::fs::metadata(&path)?.len() / 1024);
        Ok(())
    };
    let png = |r: &mut glowdots::render::Renderer, name: &str, mut p: Player, at: f32, size: (u32, u32)| -> Result<()> {
        r.resize(size)?;
        let path = out.join(format!("{name}.png"));
        export::png(&mut p, r, at, &path)?;
        eprintln!("{} ({} KB)", path.display(), std::fs::metadata(&path)?.len() / 1024);
        Ok(())
    };

    glowdots::app::offscreen(SIZE, (800, 320), |r| {
        // The hero: the demo's title cards over the starfield.
        gif(r, "hero", "demo", "2026-10-09 22:08", 0.6, 5.6, 20, (720, 288))?;

        // One clip per scene.
        let small = (480, 192);
        gif(r, "clock", "clock", "2026-10-09 22:07:55", 3.0, 4.0, 20, small)?;
        gif(r, "weather", "weather", "2026-10-09 15:20", 0.0, 7.0, 20, small)?;
        gif(r, "lake", "lake", "2026-10-09 21:40", 6.5, 6.0, 15, small)?;
        gif(r, "news", "news", "2026-10-09 12:00", 0.0, 6.0, 20, small)?;
        gif(r, "sports", "sports", "2026-10-09 12:00", 0.0, 5.5, 20, small)?;
        gif(r, "status", "status", "2026-10-09 12:00", 0.0, 5.0, 20, small)?;
        gif(r, "winter", "winter", "2026-12-20 16:20", 4.0, 5.0, 15, small)?;
        gif(r, "demo", "demo", "2026-10-09 12:00", 14.5, 5.0, 15, small)?;

        // Looks: glass, palettes, and colour.
        let big = (800, 320);
        let mut led = player("sports", "2026-10-09 12:00")?.with_glass(Glass::LED);
        led.look = Some(Look::GREEN);
        png(r, "look-led", led, 2.0, big)?;
        let mut heat = player("lake", "2026-10-09 21:40")?;
        heat.look = Some(Look::heat());
        png(r, "look-heat", heat, 7.0, big)?;
        png(r, "look-worn", player("clock", "2026-10-09 22:08")?.with_glass(Glass::WORN), 2.0, big)?;
        let deck = Deck::new(glowdots::Size::WPC, vec![Box::new(Colours)])?.with_shades(16);
        let mut colour = Player::new(deck).with_glass(STEADY);
        colour.progress_line = false;
        r.set_size(glowdots::Size::WPC)?;
        png(r, "look-color", colour, 1.0, (1024, 256))?;
        r.set_size(SIZE)?;

        // The social preview GitHub shows when the repo is shared: 1280x640.
        png(r, "social", player("demo", "2026-10-09 22:08")?, 2.2, (1280, 640))?;
        Ok(())
    })
}
