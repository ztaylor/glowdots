//! glowdots: a 1990s pinball dot matrix display (DMD) engine.
//!
//! Williams, Bally, and Data East pinball machines of the 1990s had a
//! plasma dot matrix display: a grid of orange dots, each off or lit at
//! one of a few levels, behind a glowing glass. glowdots draws that look
//! on a modern screen, and gives you the parts to put something on it.
//!
//! - [`Frame`]: the grid of dots. A scene draws into it, one dot at a time
//!   or with sprites, fonts, and the helpers in [`art`] and [`ui`].
//! - [`Scene`]: something to show. It draws a frame for each tick and says
//!   how long it stays, how it comes in, and what colour its dots are.
//! - [`Deck`]: a list of scenes. It runs the timer, the taps, and the
//!   [`Transition`]s between scenes.
//! - [`render::Renderer`]: the GPU passes that turn a frame into round
//!   plasma dots with afterglow, a halo, and bloom.
//! - [`app`] (feature `desktop`): a window that runs a deck.
//!
//! The panel can be any size. The classic machines were 128x32; the
//! showcase scenes are composed for 80x32, which reads well on small
//! screens.

pub mod app;
pub mod art;
pub mod deck;
pub mod feed;
pub mod font;
pub mod frame;
pub mod math;
pub mod render;
pub mod scene;
pub mod time;
pub mod transition;
pub mod ui;

pub use deck::Deck;
pub use frame::{Frame, MAX, Phosphor, Rect, Size, Sprite};
pub use scene::{Look, Scene};
pub use transition::Transition;
