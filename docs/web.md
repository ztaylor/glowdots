# On the web

The `web` feature plays a deck in a `<canvas>` with WebGL 2. The shaders are the same GL ES 3.0 source the desktop and the Pi run, so the display looks the same.

## The demo

`web/` is the [live demo](https://ztaylor.github.io/glowdots/): the showcase compiled to WebAssembly, and a page with controls for the scenes, glass, color, time of day, and feeds.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # the same version as wasm-bindgen in Cargo.lock
./web/build.sh
python3 -m http.server -d web/www                  # then open http://localhost:8000
```

`build.sh` uses `wasm-opt` from binaryen to make the file smaller when it is installed. A GitHub Actions workflow builds and publishes the page on every push to `main`.

## Your own deck on a page

Make a library crate that builds to WebAssembly:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
glowdots = { version = "0.1", default-features = false, features = ["web"] }
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["Document", "Element", "HtmlCanvasElement", "Window"] }
```

```rust
use glowdots::web::WebPlayer;
use glowdots::{Deck, Player, Size};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("dmd"))
        .ok_or("no canvas with id dmd")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    let deck = Deck::new(Size::WPC, vec![Box::new(Hello)]).map_err(|e| e.to_string())?;
    WebPlayer::start(Player::new(deck), canvas)?;
    Ok(())
}
```

Size the canvas with CSS; the drawing buffer follows it at the screen's pixel density. For crisp dots, make the canvas a whole number of device pixels per dot wide, the way `web/www/main.js` does.

- A tap on the canvas works like everywhere else, and once the canvas has focus the arrow keys do too.
- Keep the `WebPlayer` (it is cheap to clone) to change the player from your page: `web.with(|p| p.deck.next())`.
- A browser has no folder of files. Give the deck `Bundled` feeds, and replace them when your page fetches new data.
