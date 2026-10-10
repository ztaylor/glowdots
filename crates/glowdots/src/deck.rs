//! The deck: the list of scenes. It moves to the next scene on a timer,
//! goes forward or back on a tap, and runs the transitions.

use crate::feed::{Feeds, NoFeeds};
use crate::frame::{CLASSIC_SHADES, Frame, MAX_SHADES, Size};
use crate::math::Rng;
use crate::scene::{Context, Input, Key, Look, Scene};
use crate::time::Clock;
use crate::transition::Transition;
use anyhow::{Result, bail};
use chrono::{DateTime, Local};

/// A function the deck calls with a scene's name when that scene starts.
pub type SceneHook = Box<dyn FnMut(&str)>;

pub struct Deck {
    size: Size,
    shades: u8,
    scenes: Vec<Box<dyn Scene>>,
    current: usize,
    entered: bool,
    /// Seconds since the current scene started.
    t: f32,
    /// The transition running now, and its random seed.
    transition: Option<(Transition, u32)>,
    /// The last frame of the previous scene, for the transition.
    prev: Frame,
    prev_look: Look,
    last: Frame,
    /// A frame in the current scene's own size and shades, when they are
    /// not the panel's.
    scratch: Frame,
    rng: Rng,
    clock: Clock,
    feeds: Box<dyn Feeds>,
    /// Replay the current scene instead of moving on (for previews).
    pub hold: bool,
    /// Called when a scene starts, with its name. For logs.
    pub on_scene: Option<SceneHook>,
}

impl Deck {
    /// A deck of `scenes` on a panel of `size`, with classic shades, the
    /// real clock, and no feeds. Fails if there are no scenes, or a scene
    /// needs a bigger panel.
    pub fn new(size: Size, scenes: Vec<Box<dyn Scene>>) -> Result<Self> {
        if scenes.is_empty() {
            bail!("a deck needs at least one scene");
        }
        for s in &scenes {
            if let Some(need) = s.size() {
                if !need.fits_in(size) {
                    bail!("scene {} is composed for a {need} panel, which is bigger than the {size} panel", s.name());
                }
            }
            if !(2..=MAX_SHADES).contains(&s.shades()) {
                bail!("scene {} asks for {} shades; a scene can have 2 to {MAX_SHADES}", s.name(), s.shades());
            }
        }
        Ok(Self {
            size,
            shades: CLASSIC_SHADES,
            scenes,
            current: 0,
            entered: false,
            t: 0.0,
            transition: None,
            prev: Frame::new(size),
            prev_look: Look::WILLIAMS,
            last: Frame::new(size),
            scratch: Frame::new(size),
            rng: Rng(0x2545_f491),
            clock: Clock::system(),
            feeds: Box::new(NoFeeds),
            hold: false,
            on_scene: None,
        })
    }

    /// Give the panel `shades` shades (2..=16), counting off.
    pub fn with_shades(mut self, shades: u8) -> Self {
        self.shades = shades.clamp(2, MAX_SHADES);
        self.prev = self.new_frame();
        self.last = self.new_frame();
        self
    }

    /// Where the scenes read their feeds.
    pub fn with_feeds(mut self, feeds: impl Feeds + 'static) -> Self {
        self.feeds = Box::new(feeds);
        self
    }

    /// Where the scenes get the time. See [`Clock`].
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    /// Start at the scene called `name` (before the deck runs).
    pub fn select(&mut self, name: &str) -> Result<()> {
        let Some(i) = self.scenes.iter().position(|s| s.name() == name) else {
            bail!("there is no scene {name}; the scenes are {}", self.names().join(", "));
        };
        self.current = i;
        self.entered = false;
        self.t = 0.0;
        self.transition = None;
        Ok(())
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn shades(&self) -> u8 {
        self.shades
    }

    /// An empty frame the size and shades of the panel.
    pub fn new_frame(&self) -> Frame {
        Frame::with_shades(self.size, self.shades)
    }

    pub fn names(&self) -> Vec<&str> {
        self.scenes.iter().map(|s| s.name()).collect()
    }

    /// The scene showing now.
    pub fn current(&self) -> &dyn Scene {
        self.scenes[self.current].as_ref()
    }

    /// The deck's time now.
    pub fn now(&self) -> DateTime<Local> {
        self.clock.now()
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
        let kind = match push {
            Some(dir) => Transition::Push(dir),
            None => self.scenes[index].transition(),
        };
        self.transition = (kind.seconds() > 0.0).then(|| (kind, self.rng.next_u32()));
        self.enter();
    }

    fn enter(&mut self) {
        let cx = Context::new(self.clock.now(), self.feeds.as_ref());
        let scene = &mut self.scenes[self.current];
        scene.enter(&cx);
        self.entered = true;
        if let Some(f) = &mut self.on_scene {
            f(scene.name());
        }
    }

    /// Where the current scene sits on the panel: its top-left, and its size.
    fn placement(&self) -> ((i32, i32), Size) {
        let native = self.scenes[self.current].size().unwrap_or(self.size);
        let x = (self.size.w - native.w) as i32 / 2;
        let y = (self.size.h - native.h) as i32 / 2;
        ((x, y), native)
    }

    /// Advance by `dt` seconds and draw the frame into `out`, which must
    /// be the panel's size and shades (see [`Deck::new_frame`]).
    pub fn update(&mut self, dt: f32, out: &mut Frame) {
        debug_assert_eq!((out.size(), out.shades()), (self.size, self.shades), "the frame must match the panel");
        self.clock.advance(dt);
        if !self.entered {
            self.enter();
        }
        self.t += dt;
        if self.t >= self.scenes[self.current].duration() {
            let i = if self.hold { self.current } else { (self.current + 1) % self.scenes.len() };
            self.go(i, None);
        }

        let ((x, y), native) = self.placement();
        let cx = Context { dt, t: self.t, now: self.clock.now(), feeds: self.feeds.as_ref() };
        let scene = &mut self.scenes[self.current];
        let shades = scene.shades();
        out.clear();
        if native == self.size && shades == out.shades() {
            scene.update(&cx, out);
        } else {
            if self.scratch.size() != native || self.scratch.shades() != shades {
                self.scratch = Frame::with_shades(native, shades);
            }
            self.scratch.clear();
            scene.update(&cx, &mut self.scratch);
            out.draw_frame(&self.scratch, x, y);
        }

        if let Some((tr, seed)) = self.transition {
            let p = self.t / tr.seconds();
            if p >= 1.0 {
                self.transition = None;
            } else {
                tr.apply(p, &self.prev, out, seed);
            }
        }
        self.last.clone_from(out);
    }

    /// The viewer did something. A tap is in dots on the panel. The scene
    /// gets it first; if it does not use it, a tap on the right half or the
    /// Right key goes to the next scene, the left half or Left goes back.
    pub fn input(&mut self, input: Input) {
        let ((ox, oy), _) = self.placement();
        let local = match input {
            Input::Tap { x, y } => Input::Tap { x: x - ox as f32, y: y - oy as f32 },
            other => other,
        };
        let cx = Context { dt: 0.0, t: self.t, now: self.clock.now(), feeds: self.feeds.as_ref() };
        if self.scenes[self.current].input(&cx, &local) {
            return;
        }
        match input {
            Input::Tap { x, .. } if x < self.size.w as f32 / 2.0 => self.back(),
            Input::Tap { .. } | Input::Key(Key::Right) => self.next(),
            Input::Key(Key::Left) => self.back(),
            _ => {}
        }
    }

    /// 0..1 through the current scene, for the progress line.
    pub fn progress(&self) -> f32 {
        (self.t / self.scenes[self.current].duration()).clamp(0.0, 1.0)
    }

    /// The dot look now; it blends during a transition.
    pub fn look(&self) -> Look {
        let look = self.scenes[self.current].look();
        match self.transition {
            Some((tr, _)) => Look::mix(self.prev_look, look, (self.t / tr.seconds()).clamp(0.0, 1.0)),
            None => look,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Blink(u8, Option<Size>);
    impl Scene for Blink {
        fn name(&self) -> &str {
            "blink"
        }
        fn update(&mut self, _cx: &Context, out: &mut Frame) {
            out.set(0, 0, self.0);
        }
        fn duration(&self) -> f32 {
            1.0
        }
        fn transition(&self) -> Transition {
            Transition::Dissolve
        }
        fn size(&self) -> Option<Size> {
            self.1
        }
    }

    #[test]
    fn deck_runs_the_timer() {
        let size = Size::new(8, 4);
        let mut deck = Deck::new(size, vec![Box::new(Blink(1, None)), Box::new(Blink(3, None))]).unwrap();
        let mut out = deck.new_frame();
        deck.update(0.5, &mut out);
        assert_eq!(out.get(0, 0), 1);
        deck.update(0.6, &mut out); // past 1.0 s: the second scene starts its dissolve
        deck.update(0.3, &mut out); // half way through it, dot (0, 0) has changed over
        assert_eq!(out.get(0, 0), 3);
        deck.update(0.8, &mut out); // past 1.0 s again: back to the first scene
        deck.update(0.7, &mut out);
        assert_eq!(out.get(0, 0), 1);
    }

    #[test]
    fn small_scenes_sit_in_the_middle() {
        let mut deck = Deck::new(Size::new(8, 4), vec![Box::new(Blink(3, Some(Size::new(4, 2))))]).unwrap().with_shades(16);
        let mut out = deck.new_frame();
        deck.update(0.1, &mut out);
        assert_eq!(out.get(2, 1), 15);
        assert_eq!(out.dots.iter().filter(|&&d| d > 0).count(), 1);
    }

    #[test]
    fn big_scenes_are_refused() {
        assert!(Deck::new(Size::new(8, 4), vec![Box::new(Blink(1, Some(Size::WPC)))]).is_err());
    }
}
