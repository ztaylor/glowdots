//! The smallest glowdots program: one scene, in a window.
//!
//! ```sh
//! cargo run --example hello
//! ```

use glowdots::font::{REGULAR, SMALL};
use glowdots::{Context, Deck, Frame, MAX, Player, Scene, Size};

/// A scene draws one frame at a time. This one says hello, shows the
/// time, and runs a dot along the bottom.
struct Hello;

impl Scene for Hello {
    fn name(&self) -> &str {
        "hello"
    }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        REGULAR.draw_centered(out, "HELLO, WORLD", 6, MAX);
        SMALL.draw_centered(out, &cx.now.format("%-I:%M:%S %p").to_string(), 17, 2);

        // A bright dot runs along the bottom with a dim tail behind it.
        // `cx.t` is seconds since the scene started.
        let x = (cx.t * 40.0) as i32 % out.w();
        for k in 0..8 {
            out.set(x - k, 27, if k == 0 { MAX } else { 1 });
        }
    }

    /// How long the scene stays before the next one.
    fn duration(&self) -> f32 {
        60.0
    }
}

fn main() -> anyhow::Result<()> {
    // A 128x32 panel, like a Williams machine, with one scene.
    let deck = Deck::new(Size::WPC, vec![Box::new(Hello)])?;
    glowdots::app::run(Player::new(deck), Default::default())
}
