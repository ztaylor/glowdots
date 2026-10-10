//! The glowdots showcase: a deck of example scenes in a window.
//!
//! Usage: glowdots [--scene NAME] [--hold] [--rotate 0..3] [--fullscreen]
//!                 [--now "YYYY-MM-DD HH:MM"] [--screenshot OUT.png [--time SECONDS]]
//!                 [--list]

use anyhow::{Result, anyhow, bail};
use glowdots::Deck;
use glowdots::app::{self, Options, Screenshot};
use std::path::PathBuf;

struct Args {
    scene: Option<String>,
    hold: bool,
    list: bool,
    options: Options,
    time: f32,
    screenshot: Option<PathBuf>,
}

fn parse_args() -> Result<Args> {
    let mut args = Args { scene: None, hold: false, list: false, options: Options::default(), time: 0.0, screenshot: None };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or(anyhow!("{a} needs a value"));
        match a.as_str() {
            "--scene" => args.scene = Some(value()?),
            "--hold" => args.hold = true,
            "--list" => args.list = true,
            "--rotate" => args.options.rotate = value()?.parse()?,
            "--fullscreen" => args.options.fullscreen = true,
            "--now" => glowdots::time::simulate_now(&value()?).map_err(|e| anyhow!(e))?,
            "--screenshot" => args.screenshot = Some(value()?.into()),
            "--time" => args.time = value()?.parse()?,
            "-h" | "--help" => {
                println!("{}", USAGE.trim());
                std::process::exit(0);
            }
            s => bail!("unknown option {s}\n{}", USAGE.trim()),
        }
    }
    if let Some(path) = args.screenshot.take() {
        args.options.screenshot = Some(Screenshot { path, seconds: args.time });
    }
    Ok(args)
}

const USAGE: &str = "
glowdots: a 1990s pinball dot matrix display, with example scenes.

  --scene NAME         start at this scene (see --list)
  --hold               keep replaying the start scene
  --rotate N           quarter turns of the panel (0..3)
  --fullscreen         fill the screen (F toggles it while running)
  --now \"YYYY-MM-DD HH:MM\"   pretend it is this time
  --screenshot OUT.png [--time S]   save the frame at S seconds, then stop
  --list               print the scene names

Keys: Right/Space next, Left back, F fullscreen, Esc or Q quit.
Click or tap the right half for the next scene, the left half to go back.
";

fn main() -> Result<()> {
    let args = parse_args()?;
    let scenes = glowdots_scenes::showcase()?;
    let names: Vec<&str> = scenes.iter().map(|s| s.name()).collect();
    if args.list {
        println!("{}", names.join("\n"));
        return Ok(());
    }
    let start = match &args.scene {
        Some(name) => names.iter().position(|n| n == name).ok_or(anyhow!("no scene {name}; scenes: {names:?}"))?,
        None => 0,
    };
    let mut deck = Deck::new(glowdots_scenes::SIZE, scenes, start)?;
    deck.hold = args.hold;
    deck.on_scene = Some(Box::new(|name| eprintln!("scene: {name}")));
    eprintln!("scene: {}", deck.current().name());
    app::run(deck, args.options)
}
