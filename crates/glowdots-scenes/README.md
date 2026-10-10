# glowdots-scenes

Example scenes for [glowdots](https://crates.io/crates/glowdots), a 1990s pinball dot matrix display engine. Each one is a pattern you can take for your own, and each reads a JSON feed with a sample built in.

| Scene | Shows |
|---|---|
| `clock` | Big anti-aliased digits that roll when they change. |
| `weather` | A live sky (sun, moon, clouds, rain, snow, lightning) beside big digits and pages that type in. |
| `lake` | A landscape drawn dot by dot from a few numbers, with a winter version. |
| `news` | Headlines landing on a split-flap board, with a TV-style lower third. |
| `events`, `sports` | Cards from a JSON list, built on `cards`. |
| `status` | A status board with a heart monitor and a pinball lamp test. |
| `demo` | A demoscene tribute: starfield, plasma, a dot tunnel, and a spinning torus. |

```rust
let deck = glowdots::Deck::new(glowdots_scenes::SIZE, glowdots_scenes::showcase()?)?
    .with_feeds(glowdots_scenes::samples());
```

The scenes are composed for an 80x32 panel and are drawn in the middle of bigger ones. See them all in the [live demo](https://ztaylor.github.io/glowdots/).

<img src="https://raw.githubusercontent.com/ztaylor/glowdots/main/docs/media/news.gif" width="480" alt="A headline landing letter by letter on a split-flap board">

MIT licensed.
