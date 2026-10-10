//! glowdots: a 1990s pinball dot matrix display (DMD) engine.
//!
//! Williams, Bally, and Data East pinball machines of the 1990s had a
//! plasma dot matrix display: a grid of orange dots, each off or lit at
//! one of a few levels, behind a glowing glass. glowdots draws that look
//! on a modern screen, and gives you the parts to put something on it.
//!
//! - [`Frame`]: the grid of dots. A scene draws into it, one dot at a time
//!   or with sprites, [`font`]s, and the helpers in [`art`] and [`ui`].
//! - [`Scene`]: something to show. Each frame it gets a [`Context`] (the
//!   time, and [`Feeds`] to read data from) and draws. It says how long it
//!   stays, how it comes in, and its [`Look`].
//! - [`Deck`]: a list of scenes. It runs the timer, taps and keys, and the
//!   [`Transition`]s between scenes.
//! - [`Player`]: a deck on a [`Glass`]: the afterglow, and the GPU passes
//!   in [`render`] that draw round plasma dots with a halo and bloom.
//! - [`app`] (feature `desktop`): play in a window, or render offscreen
//!   for [`export`]ed screenshots and GIFs. [`sdl`] (feature `sdl`): play
//!   full screen on a Raspberry Pi panel with no desktop. [`web`] (feature
//!   `web`): play in a web page with WebGL 2.
//!
//! The defaults are the classic look: Williams orange, 4 shades, round
//! plasma dots. The panel can be any size, have up to 16 shades, use a
//! palette or per-dot colour, and the glass can be tuned dot by dot.
//!
//! ```no_run
//! use glowdots::{Context, Deck, Frame, Player, Scene, Size, MAX, font::REGULAR};
//!
//! struct Hello;
//!
//! impl Scene for Hello {
//!     fn name(&self) -> &str { "hello" }
//!     fn update(&mut self, cx: &Context, out: &mut Frame) {
//!         REGULAR.draw_centered(out, "HELLO", 12, MAX);
//!     }
//!     fn duration(&self) -> f32 { 10.0 }
//! }
//!
//! let deck = Deck::new(Size::WPC, vec![Box::new(Hello)])?;
//! glowdots::app::run(Player::new(deck), Default::default())?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod app;
pub mod art;
pub mod deck;
pub mod export;
pub mod feed;
pub mod font;
pub mod frame;
pub mod glass;
pub mod math;
pub mod player;
pub mod render;
pub mod scene;
pub mod sdl;
pub mod time;
pub mod transition;
pub mod ui;
pub mod web;

pub use chrono;
pub use deck::Deck;
pub use feed::{Bundled, Feed, FeedError, Feeds, Folder, Layered, NoFeeds};
pub use frame::{CLASSIC_SHADES, Frame, MAX, MAX_SHADES, Phosphor, Rect, Rgb, Size, Sprite};
pub use glass::Glass;
pub use player::Player;
pub use scene::{Context, Input, Key, Look, Scene};
pub use time::Clock;
pub use transition::Transition;
