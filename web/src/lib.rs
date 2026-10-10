//! The glowdots web demo: the showcase deck in a canvas, and the controls
//! the page calls: scenes, glass, colour, time of day, and the feeds.
//!
//! Build it with `web/build.sh`; it writes `web/www/pkg`.

#![cfg(target_arch = "wasm32")]

use glowdots::web::WebPlayer;
use glowdots::{Bundled, Clock, Deck, Feeds, Glass, Look, Player};
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

/// The sample feeds, editable from the page while the deck reads them.
#[derive(Clone)]
struct Shared(Rc<RefCell<Bundled>>);

impl Feeds for Shared {
    fn read(&self, name: &str) -> std::io::Result<String> {
        self.0.borrow().read(name)
    }
}

fn js(e: impl Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

#[wasm_bindgen]
pub struct Demo {
    web: WebPlayer,
    feeds: Shared,
}

#[wasm_bindgen]
impl Demo {
    /// Start the showcase in `canvas`.
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<Demo, JsValue> {
        console_error_panic_hook::set_once();
        let feeds = Shared(Rc::new(RefCell::new(glowdots_scenes::samples())));
        let deck = Deck::new(glowdots_scenes::SIZE, glowdots_scenes::showcase().map_err(js)?).map_err(js)?.with_feeds(feeds.clone());
        let mut player = Player::new(deck);
        player.progress_line = false;
        Ok(Demo { web: WebPlayer::start(player, canvas)?, feeds })
    }

    /// The scene names, in order.
    pub fn scenes(&self) -> Vec<String> {
        self.web.with(|p| p.deck.names().iter().map(|s| s.to_string()).collect())
    }

    /// The scene showing now.
    pub fn current(&self) -> String {
        self.web.with(|p| p.deck.current().name().to_string())
    }

    /// Go to a scene now.
    pub fn show(&self, name: &str) -> Result<(), JsValue> {
        self.web.with(|p| p.deck.jump(name)).map_err(js)
    }

    pub fn next(&self) {
        self.web.with(|p| p.deck.next());
    }

    pub fn back(&self) {
        self.web.with(|p| p.deck.back());
    }

    /// Stay on the current scene (replay it) instead of moving on.
    pub fn set_hold(&self, hold: bool) {
        self.web.with(|p| p.deck.hold = hold);
    }

    /// Glass from a preset: "classic", "led", or "worn".
    pub fn set_glass(&self, preset: &str) {
        let glass = match preset {
            "led" => Glass::LED,
            "worn" => Glass::WORN,
            _ => Glass::CLASSIC,
        };
        self.web.with(|p| p.glass = glass);
    }

    /// One glass setting (see `Glass`): "dot_size", "roundness", "halo",
    /// "halo_width", "unlit", "variation", "flicker", "gamma", "hot_core",
    /// "bloom", or "afterglow".
    pub fn glass_value(&self, name: &str) -> f32 {
        self.web.with(|p| field(&mut p.glass, name).map_or(f32::NAN, |f| *f))
    }

    pub fn set_glass_value(&self, name: &str, value: f32) {
        self.web.with(|p| {
            if let Some(f) = field(&mut p.glass, name) {
                *f = value;
            }
        });
    }

    /// One colour for every scene: "williams", "amber", "red", "green",
    /// "blue", "white", "heat", or "ocean". Anything else gives each scene
    /// its own look back.
    pub fn set_look(&self, name: &str) {
        let look = match name {
            "williams" => Some(Look::WILLIAMS),
            "amber" => Some(Look::AMBER),
            "red" => Some(Look::RED),
            "green" => Some(Look::GREEN),
            "blue" => Some(Look::BLUE),
            "white" => Some(Look::WHITE),
            "heat" => Some(Look::heat()),
            "ocean" => Some(Look::ocean()),
            _ => None,
        };
        self.web.with(|p| p.look = look);
    }

    /// Pretend it is `minutes` after midnight today; the clock runs on
    /// from there.
    pub fn set_time(&self, minutes: u32) {
        use glowdots::chrono::Local;
        let at = Local::now().date_naive().and_hms_opt(minutes / 60 % 24, minutes % 60, 0);
        if let Some(at) = at.and_then(|t| t.and_local_timezone(Local).single()) {
            self.web.with(|p| p.deck.set_clock(Clock::starting_at(at)));
        }
    }

    /// Back to the real time.
    pub fn set_real_time(&self) {
        self.web.with(|p| p.deck.set_clock(Clock::system()));
    }

    /// Minutes after midnight on the deck's clock.
    pub fn minutes(&self) -> u32 {
        use glowdots::chrono::Timelike;
        self.web.with(|p| {
            let now = p.deck.now();
            now.hour() * 60 + now.minute()
        })
    }

    /// The text of a feed ("events.json").
    pub fn feed(&self, name: &str) -> String {
        self.feeds.read(name).unwrap_or_default()
    }

    /// Replace a feed. Scenes read it the next time they start. Returns
    /// what is wrong with it, if it is not valid JSON (the scene will say
    /// so too).
    pub fn set_feed(&self, name: &str, text: &str) -> Option<String> {
        self.feeds.0.borrow_mut().insert(name, text);
        glowdots::feed::json::<serde_json::Value>(&self.feeds, name).err().map(|e| e.to_string())
    }
}

fn field<'a>(g: &'a mut Glass, name: &str) -> Option<&'a mut f32> {
    Some(match name {
        "dot_size" => &mut g.dot_size,
        "roundness" => &mut g.roundness,
        "halo" => &mut g.halo,
        "halo_width" => &mut g.halo_width,
        "unlit" => &mut g.unlit,
        "variation" => &mut g.variation,
        "flicker" => &mut g.flicker,
        "gamma" => &mut g.gamma,
        "hot_core" => &mut g.hot_core,
        "bloom" => &mut g.bloom,
        "afterglow" => &mut g.afterglow,
        _ => return None,
    })
}
