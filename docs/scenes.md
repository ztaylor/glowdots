# Writing scenes

A scene is a type that implements [`Scene`](https://docs.rs/glowdots/latest/glowdots/scene/trait.Scene.html). The deck calls it once when it comes around (`enter`) and then once a frame (`update`), and asks how long it stays (`duration`). Everything else has a default.

```rust
use glowdots::{Context, Frame, Scene, Transition, MAX};

struct Countdown { from: u32 }

impl Scene for Countdown {
    fn name(&self) -> &str { "countdown" }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        let left = self.from.saturating_sub(cx.t as u32);
        glowdots::font::REGULAR.draw_centered(out, &left.to_string(), 12, MAX);
    }

    fn duration(&self) -> f32 { self.from as f32 + 1.0 }
    fn transition(&self) -> Transition { Transition::Dissolve }
}
```

## The frame

A `Frame` is a grid of dots, top-left at (0, 0). Each dot has a shade: 0 is off, and `MAX` (3) is fully lit in a classic 4-shade frame. `out` is cleared before each `update`, so draw the whole picture every time.

- `out.set(x, y, shade)` and `out.get(x, y)`. Dots off the grid are ignored, so you can draw past the edges.
- `out.fill(rect, shade)` fills a `Rect` (the end is not included).
- `out.w()` and `out.h()` are the size. Example scenes say they need 80x32 with `fn size()`; a scene without one fills any panel.
- `out.blank()` makes an empty frame of the same size, for drawing a layer to combine.

The shades are the look. Text and outlines read best at `MAX`, backgrounds and dividers at 1, and secondary text at 2. Dark text cut out of a lit area does not read on plasma: the glow fills it in. Put bright text on a dim box instead (`ui::chip`).

## Time

`cx.t` is seconds since the scene started, and `cx.dt` is seconds since the last frame. `cx.now` is the local date and time. Use `cx.now`, not the system clock: the deck may run a simulated clock for previews, screenshots, and GIFs (`--now`), and then your scene follows it.

Animate from `cx.t` when you can. A position worked out from the time stays smooth at any frame rate and is the same every run.

## Text

`glowdots::font` has two pixel fonts in the style of the machines: `SMALL` (3x5) for labels and small print, and `REGULAR` (5x7) for titles and names. Both are capitals only; `font::clean(text)` turns any text into what they can draw (upper case, plain quotes and dashes, accents removed).

- `draw`, `draw_right`, `draw_centered`, and `draw_clip` (inside a rectangle).
- `width(text)` in dots, and `wrap(text, width)` into lines.
- `ui::marquee` scrolls text that is too wide for its box, after a pause, and `ui::marquee_seconds` says how long that takes.
- Make your own font with `Font::new(space, glyphs)`, each glyph a list of rows of `#` and `.`.

## Sprites and art

A `Sprite` is a small picture of shades.

- `Sprite::from_rows(&["  #  ", " #=# ", "#...#"])`: `#` full, `=` mid, `.` dim, anything else off (transparent when drawn).
- `Sprite::load(path, w, h)` fits any image into w x h dots.
- `Sprite::load_art(path)` and `art_from_bytes(bytes)` read pen-and-ink art drawn at 8 pixels per dot. Each 8x8 block becomes one dot, shaded by how much ink covers it, so curves come out smooth. `tools/big_digits.py` draws the clock digits this way from SVG paths.
- `out.blit(&sprite, x, y)` draws one; off dots are transparent.

`glowdots::art` has the drawing the example scenes share:

- `shade(out, x, y, brightness)` sets a dot from 0..1 with ordered (Bayer) dithering, which gives 4 shades many more steps and holds still frame to frame. The plasma and fire in the demo scene are drawn with it.
- `overlay_outlined(out, &layer)` puts the lit dots of a layer on top with a dark edge around them, so text stays readable over a busy picture.
- `sun`, `moon` (at any phase), `stars` (that twinkle in place), `cloud`, and `wind_arrow`.

`glowdots::math` has a small random number generator (`Rng`), `hash(x, y)` for patterns that hold still, and easing curves.

## Layout

`glowdots::ui` has the parts the data scenes share, so they look like one design:

- `header(out, left, right)`: a dim bar with bright labels across the top. Content starts at `ui::CONTENT_TOP`.
- `chip(out, text, x, y)`: bright text on a dim box, like a badge.
- `Pager`: pages that each stay their own time, and `slide_up` to move from one to the next.

## Patterns in the example scenes

The scenes in `crates/glowdots-scenes` are meant to be read and taken apart.

- **Typing text** (`weather`, `lake`): show `(t * letters_per_second) as usize` characters of a line.
- **Split-flap** (`news`): before a letter lands, draw a random letter from `hash(i, (t * 30.0) as u32)`; each letter lands a little after the one before.
- **Rolling digits** (`clock`): when a digit changes, draw the old one moving down and the new one coming in from above, clipped to the digit's box.
- **A living picture** (`lake`): the sky, the treeline, the water, and the animals are each a function of the time and a few numbers from the feed. Fixed randomness comes from `hash`, so the trees stay where they are.
- **Cards** (`events`, `sports`): `cards::CardScene` pages through cards made from a list feed. Write a `Card` and a function that reads the feed, and the paging, header, counts, and empty and error messages come with it.
- **Demo effects** (`demo`): a starfield, a sine plasma, a tunnel of dot rings, and a torus of points in 3D, each in a few lines.

## Transitions, looks, and input

- `fn transition()` picks how the scene comes in when the timer moves to it: `Wipe` (a bright bar sweeps across), `Dissolve`, `Push`, `Ink` (a ragged blot spreads), or `Cut`. A tap always pushes the next scene in from the side tapped.
- `fn look()` picks the dot color: `Look::WILLIAMS` by default, another color, or a palette. See [Looks and glass](looks.md).
- `fn shades()` draws with more shades, up to 16. Use `out.max()` and `out.level(brightness)` instead of `MAX`.
- `fn input()` gets taps (in dots) and keys. Return true to keep one; otherwise the deck uses it to move between scenes.

## Playing scenes

```rust
let deck = Deck::new(Size::WPC, vec![Box::new(Countdown { from: 10 })])?
    .with_feeds(Folder::new("feeds"));
glowdots::app::run(Player::new(deck), Default::default())?;
```

To check a scene without a window, step it yourself and look at the dots: `deck.update(1.0 / 60.0, &mut frame)` with `Clock::stepped(...)` gives the same frames every run. `glowdots-scenes` tests every example scene this way.
