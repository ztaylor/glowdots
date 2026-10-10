//! Sports on TV today, one game per card, like a scoreboard: the league,
//! the two sides (or the event), the time, a note, and the channel.
//!
//! It reads `sports.json` (see `samples/sports.json`), a list feed:
//!
//! ```json
//! {"date": "2026-10-06", "items": [
//!   {"league": "MLB", "away": "Dodgers", "home": "Braves", "time": "5:00 PM", "channel": "FS1", "note": "NLDS Game 3"},
//!   {"league": "F1", "title": "US Grand Prix Practice", "time": "4:30 PM", "channel": "ESPN2"}
//! ]}
//! ```

use crate::cards::{Card, CardScene};
use glowdots::font::{REGULAR, SMALL, clean};
use glowdots::{Context, Feed, Frame, MAX, Transition, ui};
use serde::Deserialize;

const WIDTH: usize = crate::SIZE.w as usize;

/// One game (or event) in the feed.
#[derive(Deserialize, Clone)]
pub struct Game {
    pub league: Option<String>,
    /// The two sides.
    pub away: Option<String>,
    pub home: Option<String>,
    /// For example "NLDS Game 3".
    pub note: Option<String>,
    /// For things that are not one team against another (a race, a match).
    pub title: Option<String>,
    pub time: Option<String>,
    pub channel: Option<String>,
}

/// The sports scene, titled "SPORTS ON TV".
pub fn scene() -> CardScene {
    CardScene::new("sports", "SPORTS ON TV", "sports.json", load).empty("NO GAMES", "NOTHING ON TV TODAY").transition(Transition::Dissolve)
}

fn load(cx: &Context, name: &str) -> Feed<Box<dyn Card>> {
    cx.items::<Game>(name).map(|g| Box::new(GameCard::new(g)) as Box<dyn Card>)
}

struct GameCard {
    league: String,
    /// Top line: the away team, or the title of the event.
    top: String,
    /// Second line: the other team (shown after "VS"), or empty.
    home: String,
    note: String,
    time: String,
    channel: String,
}

impl GameCard {
    fn new(g: Game) -> Self {
        let c = |s: Option<String>| clean(&s.unwrap_or_default());
        let (top, home) = match (g.away, g.home) {
            (Some(a), Some(h)) => (clean(&a), clean(&h)),
            _ => (c(g.title), String::new()),
        };
        Self { league: c(g.league), top, home, note: c(g.note), time: c(g.time), channel: c(g.channel) }
    }

    /// Room left for the top line beside the league chip.
    fn top_right(&self) -> i32 {
        let chip = if self.league.is_empty() { 0 } else { SMALL.width(&self.league) + 4 + 2 };
        WIDTH as i32 - 2 - chip
    }
}

impl Card for GameCard {
    fn seconds(&self) -> f32 {
        let scroll = ui::marquee_seconds(&REGULAR, &self.top, self.top_right() - 2).max(ui::marquee_seconds(
            &REGULAR,
            &self.home,
            WIDTH as i32 - 14,
        ));
        (scroll + 1.5).max(5.0)
    }

    fn draw(&self, out: &mut Frame, t: f32) {
        let right = WIDTH as i32 - 2;
        if !self.league.is_empty() {
            ui::chip_right(out, &self.league, right + 1, 9);
        }
        ui::marquee(out, &REGULAR, &self.top, 2, self.top_right(), 9, MAX, t);
        if !self.home.is_empty() {
            SMALL.draw(out, "VS", 2, 18, 1);
            ui::marquee(out, &REGULAR, &self.home, 12, right, 17, MAX, t);
        }
        SMALL.draw(out, &self.time, 2, 26, MAX);
        let chip_left = if self.channel.is_empty() { right } else { right + 1 - (SMALL.width(&self.channel) + 4) };
        if !self.channel.is_empty() {
            ui::chip_right(out, &self.channel, right + 1, 25);
        }
        // The note sits between the time and the channel.
        let (x0, x1) = (2 + SMALL.width(&self.time) + 4, chip_left - 3);
        let x = x0.max((x0 + x1 - SMALL.width(&self.note)) / 2);
        ui::marquee(out, &SMALL, &self.note, x, x1, 26, 2, t);
    }
}
