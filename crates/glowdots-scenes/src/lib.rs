//! Example scenes for glowdots, composed for an 80x32 panel.
//!
//! Each scene shows a pattern you can take for your own: big rolling
//! digits, a demoscene part, and (in later modules) a live sky, a
//! split-flap board, pages that type in, and cards from a JSON feed.

pub mod clock;
pub mod demo;

use glowdots::{Scene, Size};

/// The panel the example scenes are composed for.
pub const SIZE: Size = Size::new(80, 32);

/// The showcase deck: every example scene, in order.
pub fn showcase() -> anyhow::Result<Vec<Box<dyn Scene>>> {
    Ok(vec![Box::new(clock::Clock::new()?), Box::new(demo::Demo::new())])
}
