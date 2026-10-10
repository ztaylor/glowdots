# glowdots

<img src="https://raw.githubusercontent.com/ztaylor/glowdots/main/docs/media/hero.gif" width="720" alt="The word GLOWDOTS lights up in orange plasma dots over a starfield">

A 1990s pinball dot matrix display engine. glowdots draws the orange plasma display of the Williams, Bally, and Data East pinball machines (round dots, white-hot centers, a soft glow, a quick afterglow) and gives you the parts to put anything on it: frames, sprites, pixel fonts, a deck of scenes with pinball transitions, and JSON feeds.

It plays in a desktop window, in a web page with WebGL 2, or full screen on a Raspberry Pi with no desktop. **[Try the live demo](https://ztaylor.github.io/glowdots/).**

```rust
use glowdots::font::REGULAR;
use glowdots::{Context, Deck, Frame, MAX, Player, Scene, Size};

struct Hello;

impl Scene for Hello {
    fn name(&self) -> &str { "hello" }
    fn update(&mut self, _cx: &Context, out: &mut Frame) {
        REGULAR.draw_centered(out, "HELLO, WORLD", 12, MAX);
    }
    fn duration(&self) -> f32 { 60.0 }
}

fn main() -> anyhow::Result<()> {
    let deck = Deck::new(Size::WPC, vec![Box::new(Hello)])?;
    glowdots::app::run(Player::new(deck), Default::default())
}
```

| Feature | Default | What it adds |
|---|---|---|
| `desktop` | yes | A window on macOS, Windows, and Linux, and offscreen rendering. |
| `gif` | yes | Animated GIF export. |
| `sdl` | no | Full screen through SDL2, for a Raspberry Pi panel with no desktop. Needs `libsdl2-dev`. |
| `web` | no | Play in a canvas with WebGL 2, on `wasm32`. |

Example scenes (a clock, weather, a living lake, a split-flap news board, cards from JSON, a status board, and a demoscene tribute) are in [`glowdots-scenes`](https://crates.io/crates/glowdots-scenes). Guides are in the [repository](https://github.com/ztaylor/glowdots).

MIT licensed. Not affiliated with Williams, Bally, Data East, Stern, or Sega.
