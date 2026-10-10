//! Example scenes for glowdots, composed for an 80x32 panel.
//!
//! Each scene shows a pattern you can take for your own:
//!
//! - [`clock`]: big anti-aliased digits that roll when they change.
//! - [`weather`]: a live sky window (sun, moon, clouds, rain, snow,
//!   lightning) beside big digits and pages that type in.
//! - [`lake`]: a whole living landscape drawn dot by dot from a few
//!   numbers, with a winter version.
//! - [`news`]: a split-flap headline board with a TV-style lower third.
//! - [`events`] and [`sports`]: cards from a JSON list feed, built on
//!   [`cards`], the simplest way to put your own data on the display.
//! - [`status`]: a status board with a heart monitor and a lamp test.
//! - [`demo`]: a 1990s demoscene tribute: starfield, plasma, a dot
//!   tunnel, and a spinning torus.
//!
//! Scenes that read data say which feed and what shape in their docs, and
//! [`samples`] has an example of each.

pub mod cards;
pub mod clock;
pub mod demo;
mod digits;
pub mod events;
pub mod lake;
pub mod news;
pub mod sports;
pub mod status;
pub mod weather;

use glowdots::{Bundled, Scene, Size};

/// The panel the example scenes are composed for.
pub const SIZE: Size = Size::new(80, 32);

/// The showcase deck: every example scene, in order.
pub fn showcase() -> anyhow::Result<Vec<Box<dyn Scene>>> {
    Ok(vec![
        Box::new(clock::Clock::new()?),
        Box::new(weather::Weather::new()?),
        Box::new(lake::Lake::new().title("MIRROR LAKE")),
        Box::new(news::News::new()),
        Box::new(events::scene()),
        Box::new(sports::scene()),
        Box::new(status::Status::new()),
        Box::new(lake::Lake::new().named("winter").title("MIRROR LAKE").feeds("lake_winter.json", "weather_winter.json")),
        Box::new(demo::Demo::new()),
    ])
}

/// Sample feeds for every example scene, built in. Use them to try the
/// scenes, and as a template for your own data.
pub fn samples() -> Bundled {
    let mut b = Bundled::new();
    for (name, text) in [
        ("weather.json", include_str!("../samples/weather.json")),
        ("weather_winter.json", include_str!("../samples/weather_winter.json")),
        ("lake.json", include_str!("../samples/lake.json")),
        ("lake_winter.json", include_str!("../samples/lake_winter.json")),
        ("news.json", include_str!("../samples/news.json")),
        ("events.json", include_str!("../samples/events.json")),
        ("sports.json", include_str!("../samples/sports.json")),
        ("status.json", include_str!("../samples/status.json")),
    ] {
        b.insert(name, text);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use glowdots::{Clock, Deck, time};

    /// Every scene in the showcase runs a full visit on the sample feeds, at
    /// noon and at midnight, without panicking, and draws something.
    #[test]
    fn every_scene_runs_on_the_samples() {
        for at in ["2026-10-09 12:00", "2026-10-09 23:30"] {
            let clock = Clock::stepped(time::parse_local(at).unwrap());
            let mut deck = Deck::new(SIZE, showcase().unwrap()).unwrap().with_feeds(samples()).with_clock(clock);
            let names: Vec<String> = deck.names().iter().map(|s| s.to_string()).collect();
            for name in names {
                deck.select(&name).unwrap();
                deck.hold = true;
                let mut out = deck.new_frame();
                let mut lit = 0;
                for _ in 0..(60 * 40) {
                    deck.update(1.0 / 60.0, &mut out);
                    lit = lit.max(out.dots.iter().filter(|&&d| d > 0).count());
                }
                assert!(lit > 40, "{name} at {at} drew only {lit} dots");
            }
        }
    }

    /// The samples are all used, and all valid JSON.
    #[test]
    fn samples_parse() {
        for name in samples().names() {
            let text = glowdots::Feeds::read(&samples(), name).unwrap();
            serde_json::from_str::<serde_json::Value>(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }
}
