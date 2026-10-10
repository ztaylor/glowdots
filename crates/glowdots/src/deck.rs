//! The deck: the list of scenes. It moves to the next scene on a timer,
//! goes forward or back on a tap, and runs the transitions.

use crate::frame::{Frame, Size};
use crate::math::Rng;
use crate::scene::{Look, Scene};
use crate::transition::Transition;
use anyhow::{Result, bail};

/// A function the deck calls with a scene's name when that scene starts.
pub type SceneHook = Box<dyn FnMut(&str)>;

pub struct Deck {
    size: Size,
    scenes: Vec<Box<dyn Scene>>,
    current: usize,
    /// Seconds since the current scene started.
    t: f32,
    transition: Option<Transition>,
    /// The last frame of the previous scene, for the transition.
    prev: Frame,
    prev_look: Look,
    last: Frame,
    rng: Rng,
    /// Replay the current scene instead of moving on (for previews).
    pub hold: bool,
    /// Called when a scene starts, with its name. For logs.
    pub on_scene: Option<SceneHook>,
}

impl Deck {
    /// A deck of `scenes` on a panel of `size`, starting at scene `start`.
    /// Fails if there are no scenes, or a scene was composed for another size.
    pub fn new(size: Size, scenes: Vec<Box<dyn Scene>>, start: usize) -> Result<Self> {
        if scenes.is_empty() {
            bail!("a deck needs at least one scene");
        }
        if start >= scenes.len() {
            bail!("start scene {start} is out of range (there are {} scenes)", scenes.len());
        }
        for s in &scenes {
            if let Some(need) = s.size() {
                if need != size {
                    bail!("scene {} is composed for a {need} panel, but the panel is {size}", s.name());
                }
            }
        }
        let mut deck = Self {
            size,
            scenes,
            current: start,
            t: 0.0,
            transition: None,
            prev: Frame::new(size),
            prev_look: Look::WILLIAMS,
            last: Frame::new(size),
            rng: Rng(0x2545_f491),
            hold: false,
            on_scene: None,
        };
        deck.scenes[start].enter();
        Ok(deck)
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn names(&self) -> Vec<&str> {
        self.scenes.iter().map(|s| s.name()).collect()
    }

    /// The scene showing now.
    pub fn current(&self) -> &dyn Scene {
        self.scenes[self.current].as_ref()
    }

    pub fn next(&mut self) {
        let i = (self.current + 1) % self.scenes.len();
        self.go(i, Some(1));
    }

    pub fn back(&mut self) {
        let i = (self.current + self.scenes.len() - 1) % self.scenes.len();
        self.go(i, Some(-1));
    }

    /// Switch scenes. A tap pushes in its direction; the timer uses the
    /// new scene's own transition.
    fn go(&mut self, index: usize, push: Option<i32>) {
        self.prev.clone_from(&self.last);
        self.prev_look = self.look();
        self.current = index;
        self.t = 0.0;
        let scene = &mut self.scenes[index];
        scene.enter();
        self.transition = Some(match (push, scene.transition()) {
            (Some(dir), _) => Transition::Push(dir),
            (None, Transition::InkBlot { .. }) => Transition::ink_blot(&mut self.rng, self.size),
            (None, t) => t,
        });
        if let Some(f) = &mut self.on_scene {
            f(scene.name());
        }
    }

    /// Advance by `dt` seconds and draw the frame into `out`.
    pub fn update(&mut self, dt: f32, out: &mut Frame) {
        self.t += dt;
        if self.t >= self.scenes[self.current].duration() {
            let i = if self.hold { self.current } else { (self.current + 1) % self.scenes.len() };
            self.go(i, None);
        }

        out.clear();
        self.scenes[self.current].update(dt, self.t, out);

        if let Some(tr) = self.transition {
            let p = self.t / tr.seconds();
            if p >= 1.0 {
                self.transition = None;
            } else {
                tr.apply(p, &self.prev, out);
            }
        }
        self.last.clone_from(out);
    }

    /// 0..1 through the current scene, for the progress line.
    pub fn progress(&self) -> f32 {
        (self.t / self.scenes[self.current].duration()).clamp(0.0, 1.0)
    }

    /// The dot look now; it blends during a transition.
    pub fn look(&self) -> Look {
        let look = self.scenes[self.current].look();
        match self.transition {
            Some(tr) => Look::mix(self.prev_look, look, (self.t / tr.seconds()).clamp(0.0, 1.0)),
            None => look,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Blink(u8);
    impl Scene for Blink {
        fn name(&self) -> &str {
            "blink"
        }
        fn update(&mut self, _dt: f32, _t: f32, out: &mut Frame) {
            out.set(0, 0, self.0);
        }
        fn duration(&self) -> f32 {
            1.0
        }
        fn transition(&self) -> Transition {
            Transition::Dissolve
        }
        fn size(&self) -> Option<Size> {
            Some(Size::new(8, 4))
        }
    }

    #[test]
    fn deck_runs_the_timer() {
        let size = Size::new(8, 4);
        let mut deck = Deck::new(size, vec![Box::new(Blink(1)), Box::new(Blink(3))], 0).unwrap();
        let mut out = Frame::new(size);
        deck.update(0.5, &mut out);
        assert_eq!(deck.current().name(), "blink");
        assert_eq!(out.get(0, 0), 1);
        deck.update(0.6, &mut out); // past 1.0 s: the second scene starts its dissolve
        deck.update(0.3, &mut out); // half way through it, dot (0, 0) has changed over
        assert_eq!(deck.current().name(), "blink");
        assert_eq!(out.get(0, 0), 3);
        deck.update(0.8, &mut out); // past 1.0 s again: back to the first scene
        deck.update(0.7, &mut out);
        assert_eq!(out.get(0, 0), 1);
    }

    #[test]
    fn deck_refuses_the_wrong_size() {
        assert!(Deck::new(Size::WPC, vec![Box::new(Blink(1))], 0).is_err());
    }
}
