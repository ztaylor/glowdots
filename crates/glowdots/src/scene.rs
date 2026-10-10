//! Scenes: the things the display shows, one at a time.
//!
//! Each scene draws its own frames and chooses its own look (dot colour)
//! and its own way in (transition). So a scene can have its own style
//! without the others changing.

use crate::frame::{Frame, Size};
use crate::transition::Transition;

/// How the dots look while a scene shows. The renderer reads this.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Look {
    /// Colour of a fully lit dot (linear RGB, 0..1).
    pub color: [f32; 3],
}

impl Look {
    /// Williams WPC red-orange neon plasma. The classic.
    pub const WILLIAMS: Look = Look { color: [1.0, 0.26, 0.02] };
    /// A warmer amber, like a worn panel.
    pub const AMBER: Look = Look { color: [1.0, 0.5, 0.08] };
    /// Green phosphor, like an old terminal.
    pub const GREEN: Look = Look { color: [0.25, 1.0, 0.3] };
    /// Ice blue.
    pub const BLUE: Look = Look { color: [0.3, 0.55, 1.0] };
    /// Warm white, like modern LED replacement panels.
    pub const WHITE: Look = Look { color: [1.0, 0.92, 0.8] };

    pub const fn rgb(r: f32, g: f32, b: f32) -> Look {
        Look { color: [r, g, b] }
    }

    pub fn mix(a: Look, b: Look, p: f32) -> Look {
        let c = |i: usize| a.color[i] + (b.color[i] - a.color[i]) * p;
        Look { color: [c(0), c(1), c(2)] }
    }
}

pub trait Scene {
    /// Short name, for logs and for choosing a scene from the command line.
    fn name(&self) -> &str;

    /// The scene is about to show. Reload data and reset animation here.
    fn enter(&mut self) {}

    /// Draw the frame at `t` seconds after the scene started. `dt` is the
    /// time since the last frame. `out` is cleared before each call.
    fn update(&mut self, dt: f32, t: f32, out: &mut Frame);

    /// How long the scene stays before the next one, in seconds.
    fn duration(&self) -> f32;

    /// How the scene comes in when the timer moves to it.
    fn transition(&self) -> Transition {
        Transition::Wipe
    }

    fn look(&self) -> Look {
        Look::WILLIAMS
    }

    /// The panel size this scene was composed for, if it needs one. The
    /// deck refuses a scene whose size does not match the panel's.
    fn size(&self) -> Option<Size> {
        None
    }
}
