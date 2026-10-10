//! A scene that pages through cards (one event, one game...) under a
//! header bar. The events and sports scenes are card scenes; make your own
//! by writing a [`Card`] and a function that reads a feed into cards.
//!
//! The feed is a list feed (see [`glowdots::feed::items`]): if it is dated
//! another day, the scene says "NOT UPDATED"; if it is missing or bad, it
//! says so.

use glowdots::font::{REGULAR, SMALL};
use glowdots::ui::{self, CONTENT_TOP, PAGE_SLIDE, Pager};
use glowdots::{Context, Feed, Frame, MAX, Scene, Size, Transition};

/// The most cards one visit to the scene shows.
const MAX_CARDS: usize = 8;

pub trait Card {
    /// How long this card stays.
    fn seconds(&self) -> f32;
    /// Draw the card below the header. `t` is the time into the card.
    fn draw(&self, out: &mut Frame, t: f32);
}

/// Words to show when there is nothing else: a big line and a small line.
pub struct Message(pub String, pub String);

impl Card for Message {
    fn seconds(&self) -> f32 {
        6.0
    }

    fn draw(&self, out: &mut Frame, t: f32) {
        let w = out.w();
        let x = ((w - REGULAR.width(&self.0)) / 2).max(2);
        ui::marquee(out, &REGULAR, &self.0, x, w - 2, 12, MAX, t);
        let x = ((w - SMALL.width(&self.1)) / 2).max(2);
        ui::marquee(out, &SMALL, &self.1, x, w - 2, 23, 2, t);
    }
}

/// Reads a feed (by name) into cards.
pub type Loader = fn(&Context, &str) -> Feed<Box<dyn Card>>;

pub struct CardScene {
    name: String,
    /// Header label, left side.
    title: String,
    feed_name: String,
    load: Loader,
    /// The message when the feed has no items today.
    empty: (String, String),
    waiting: Option<String>,
    transition: Transition,
    cards: Vec<Box<dyn Card>>,
    real: bool,
    pager: Pager,
}

impl CardScene {
    /// A card scene called `name`, with `title` in its header, that reads
    /// the feed `feed_name` with `load`.
    pub fn new(name: &str, title: &str, feed_name: &str, load: Loader) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            feed_name: feed_name.into(),
            load,
            empty: ("NOTHING TODAY".into(), "THE LIST IS EMPTY".into()),
            waiting: None,
            transition: Transition::Wipe,
            cards: Vec::new(),
            real: false,
            pager: Pager { seconds: vec![1.0] },
        }
    }

    /// Call the scene something else (for logs and choosing it).
    pub fn named(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    /// The header label.
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.into();
        self
    }

    /// Read another feed.
    pub fn feed(mut self, name: &str) -> Self {
        self.feed_name = name.into();
        self
    }

    /// What to say when the feed has no items today: a big and a small line.
    pub fn empty(mut self, big: &str, small: &str) -> Self {
        self.empty = (big.into(), small.into());
        self
    }

    /// What to say under "NO DATA YET" when the feed is missing or bad. By
    /// default it says what is wrong ("NO FEED YET", "BAD FEED: LINE 3").
    pub fn waiting(mut self, text: &str) -> Self {
        self.waiting = Some(text.into());
        self
    }

    pub fn transition(mut self, transition: Transition) -> Self {
        self.transition = transition;
        self
    }
}

impl Scene for CardScene {
    fn name(&self) -> &str {
        &self.name
    }

    fn enter(&mut self, cx: &Context) {
        let msg = |a: &str, b: &str| -> Vec<Box<dyn Card>> { vec![Box::new(Message(a.into(), b.into()))] };
        (self.cards, self.real) = match (self.load)(cx, &self.feed_name) {
            Feed::Ready(mut cards) if !cards.is_empty() => {
                cards.truncate(MAX_CARDS);
                (cards, true)
            }
            Feed::Ready(_) => (msg(&self.empty.0, &self.empty.1), false),
            Feed::Stale(date) => (msg("NOT UPDATED", &format!("LATEST LIST IS FROM {date}")), false),
            Feed::Failed(e) => (msg("NO DATA YET", &self.waiting.clone().unwrap_or_else(|| e.short())), false),
            _ => (msg("NO DATA YET", ""), false),
        };
        self.pager = Pager { seconds: self.cards.iter().map(|c| c.seconds()).collect() };
    }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        let (i, local) = self.pager.locate(cx.t);
        let mut page = out.blank();
        self.cards[i].draw(&mut page, local);
        if i > 0 && local < PAGE_SLIDE {
            let mut old = out.blank();
            self.cards[i - 1].draw(&mut old, self.pager.seconds[i - 1]);
            ui::slide_up(out, &old, &page, local / PAGE_SLIDE);
        } else {
            let h = out.h();
            out.copy_rows(&page, CONTENT_TOP, h, 0);
        }
        let count = if self.real { format!("{}/{}", i + 1, self.cards.len()) } else { String::new() };
        ui::header(out, &self.title, &count);
    }

    fn duration(&self) -> f32 {
        self.pager.total()
    }

    fn transition(&self) -> Transition {
        self.transition
    }

    fn size(&self) -> Option<Size> {
        Some(crate::SIZE)
    }
}
