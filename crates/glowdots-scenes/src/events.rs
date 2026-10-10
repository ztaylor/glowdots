//! Today's events, one card each: the title, the venue, the time, and the
//! kind of event.
//!
//! It reads `events.json` (see `samples/events.json`), a list feed:
//!
//! ```json
//! {"date": "2026-10-06", "items": [
//!   {"title": "The Midnight Owls", "venue": "The Orpheum", "time": "8:00 PM", "category": "Music"}
//! ]}
//! ```
//!
//! Only `title` is necessary.

use crate::cards::{Card, CardScene};
use glowdots::font::{REGULAR, SMALL, clean};
use glowdots::{Context, Feed, Frame, MAX, Transition, ui};
use serde::Deserialize;

const WIDTH: i32 = crate::SIZE.w as i32;

/// One event in the feed.
#[derive(Deserialize, Clone)]
pub struct Event {
    pub title: String,
    pub venue: Option<String>,
    pub time: Option<String>,
    pub category: Option<String>,
}

/// The events scene, titled "TODAY".
pub fn scene() -> CardScene {
    CardScene::new("events", "TODAY", "events.json", load).empty("QUIET NIGHT", "NO EVENTS ON THE LIST TODAY").transition(Transition::Wipe)
}

fn load(cx: &Context, name: &str) -> Feed<Box<dyn Card>> {
    cx.items::<Event>(name).map(|e| Box::new(EventCard::new(e)) as Box<dyn Card>)
}

struct EventCard {
    title: String,
    venue: String,
    time: String,
    category: String,
}

impl EventCard {
    fn new(e: Event) -> Self {
        let c = |s: Option<String>| clean(&s.unwrap_or_default());
        Self { title: clean(&e.title), venue: c(e.venue), time: c(e.time), category: c(e.category) }
    }
}

impl Card for EventCard {
    fn seconds(&self) -> f32 {
        let w = WIDTH - 4;
        let scroll = ui::marquee_seconds(&REGULAR, &self.title, w).max(ui::marquee_seconds(&SMALL, &self.venue, w));
        (scroll + 1.5).max(5.0)
    }

    fn draw(&self, out: &mut Frame, t: f32) {
        let right = WIDTH - 2;
        ui::marquee(out, &REGULAR, &self.title, 2, right, 10, MAX, t);
        ui::marquee(out, &SMALL, &self.venue, 2, right, 19, 2, t);
        if !self.time.is_empty() {
            ui::chip(out, &self.time, 1, 25);
        }
        SMALL.draw_right(out, &self.category, right, 26, 1);
    }
}
