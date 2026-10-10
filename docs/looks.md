# Looks and glass

Two things decide how the display looks. The **look** is the color of a scene's dots. The **glass** is the display itself: the size and shape of its dots, and how they glow, shimmer, and fade. The defaults are a 1990s Williams plasma panel.

## Glass

```rust
let player = Player::new(deck).with_glass(Glass::LED);
let mut glass = Glass::CLASSIC;
glass.bloom = 1.2;
glass.roundness = 0.5;
```

| Setting | Classic | What it does |
|---|---|---|
| `dot_size` | 0.43 | Dot radius in dots; 0.5 makes neighbors touch. |
| `roundness` | 1.0 | 1 is a round plasma dot, 0 a square LED pixel. |
| `halo` | 0.12 | The tight glow around each lit dot. |
| `halo_width` | 0.45 | How far that glow reaches, in dots. |
| `unlit` | 0.065 | How visible unlit dots are. |
| `variation` | 0.18 | Dot-to-dot difference in brightness, like an old panel. |
| `flicker` | 0.05 | The fast shimmer of the plasma. Set it to 0 for smaller GIFs. |
| `gamma` | 1.6 | Spreads the shades apart, so dim text still reads. |
| `hot_core` | 0.6 | How far bright dot centers lean to white. |
| `bloom` | 0.8 | The wide, soft glow around everything. |
| `afterglow` | 0.05 | Seconds a dot takes to go dark after it goes off. |

The presets are `Glass::CLASSIC`, `Glass::LED` (crisp rounded squares, steady, almost no afterglow), and `Glass::WORN` (uneven dots, more shimmer, a big glow). The [live demo](https://ztaylor.github.io/glowdots/) has a slider for each setting.

## Looks

A scene picks its look with `fn look()`, and the deck blends from one look to the next during a transition.

- **A color**: `Look::WILLIAMS` (the default), `AMBER`, `RED`, `GREEN`, `BLUE`, `WHITE`, or any `Look::rgb(r, g, b)` or `Look::hex(0xff4205)`.
- **A palette**: `Look::palette(&[Rgb(0, 0, 0), Rgb(0x60, 0x08, 0), Rgb(0xff, 0x90, 0x10), Rgb(0xff, 0xf0, 0xb0)])` gives each shade its own color, from off to full, the way color kits colorize a classic display. `Look::heat()` and `Look::ocean()` are ready-made ones.
- **One look for everything**: `player.look = Some(Look::GREEN)` overrides every scene's own.

## Shades

Williams games had 4 shades: off and 3 lit levels. That is the default and what `MAX` means. A panel can have up to 16, like later Stern games:

```rust
let deck = Deck::new(Size::WPC, scenes)?.with_shades(16);
```

A scene draws with the shades it asks for (`fn shades()`, 4 unless it says otherwise), and the deck spreads them over the panel's. So classic scenes look the same on a 16-shade panel, and a 16-shade scene can sit next to them.

## Color per dot

Any dot can have its own color, over the look:

```rust
out.set_rgb(x, y, MAX, Rgb::hex(0x30ff40));     // one dot
out.tint(Rect::new(0, 0, 40, 8), Rgb(255, 0, 0)); // the lit dots in a box, after drawing
```

## Panel size

The panel can be any size. `Size::WPC` is 128x32, the Williams and Bally size, and `Size::SEGA_WIDE` is the 192x64 display of some late-1990s Sega games. The example scenes are composed for 80x32, which reads well on small screens. On a bigger panel they are drawn in the middle.

The renderer fits the panel to the screen at a whole number of pixels per dot, so every dot is the same shape, and centers it.

## Pictures and GIFs

```sh
cargo run --release -- --scene lake --screenshot lake.png --time 5 --size 1200x480
cargo run --release -- --scene news --gif news.gif --seconds 6 --fps 25 --steady
```

From code, `glowdots::export::png` and `glowdots::export::gif` take a player and an offscreen renderer (`glowdots::app::offscreen`). A GIF keeps one palette for the whole clip and stores only the pixels that change, so turn the shimmer off (`--steady`) and a few seconds of a calm scene is a few hundred kilobytes. `cargo run --release --bin media` makes every picture in the README this way.
