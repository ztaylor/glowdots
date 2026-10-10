//! The glowdots showcase: a deck of example scenes in a window.

use anyhow::{Result, anyhow, bail};
use glowdots::app::{self, Options};
use glowdots::{Clock, Deck, Folder, Glass, Layered, Player, export, time};
use std::path::PathBuf;

const USAGE: &str = "
glowdots: a 1990s pinball dot matrix display, with example scenes.

  --scene NAME            start at this scene (see --list)
  --hold                  keep replaying the start scene
  --data DIR              read feeds from DIR first (then the samples)
  --glass classic|led|worn
  --steady                no plasma shimmer (smaller GIFs)
  --shades N              panel shades, 2..16 (default 4)
  --rotate N              quarter turns of the panel (0..3)
  --fullscreen            fill the screen (F toggles it while running)
  --kiosk                 full screen through SDL, for a Raspberry Pi with
                          no desktop (build with --features sdl)
  --now \"YYYY-MM-DD HH:MM\" pretend it is this time
  --screenshot OUT.png    save the frame at --time seconds, then stop
  --gif OUT.gif           record --seconds of play from --time, then stop
  --time S  --seconds S  --fps N  --size WxH
  --list                  print the scene names

Keys: Right/Space next, Left back, F fullscreen, Esc or Q quit.
Click or tap the right half for the next scene, the left half to go back.
";

struct Args {
    scene: Option<String>,
    hold: bool,
    list: bool,
    data: Option<PathBuf>,
    glass: Glass,
    shades: u8,
    rotate: u32,
    fullscreen: bool,
    kiosk: bool,
    now: Option<String>,
    screenshot: Option<PathBuf>,
    gif: Option<PathBuf>,
    time: f32,
    seconds: f32,
    fps: u32,
    size: (u32, u32),
}

fn parse_args() -> Result<Args> {
    let mut a = Args {
        scene: None,
        hold: false,
        list: false,
        data: None,
        glass: Glass::CLASSIC,
        shades: 4,
        rotate: 0,
        fullscreen: false,
        kiosk: false,
        now: None,
        screenshot: None,
        gif: None,
        time: 0.0,
        seconds: 6.0,
        fps: 25,
        size: (1200, 480),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or(anyhow!("{arg} needs a value"));
        match arg.as_str() {
            "--scene" => a.scene = Some(value()?),
            "--hold" => a.hold = true,
            "--list" => a.list = true,
            "--data" => a.data = Some(value()?.into()),
            "--glass" => {
                a.glass = match value()?.as_str() {
                    "classic" => Glass::CLASSIC,
                    "led" => Glass::LED,
                    "worn" => Glass::WORN,
                    g => bail!("no glass {g}: use classic, led, or worn"),
                }
            }
            "--shades" => a.shades = value()?.parse()?,
            "--steady" => a.glass.flicker = 0.0,
            "--rotate" => a.rotate = value()?.parse()?,
            "--fullscreen" => a.fullscreen = true,
            "--kiosk" => a.kiosk = true,
            "--now" => a.now = Some(value()?),
            "--screenshot" => a.screenshot = Some(value()?.into()),
            "--gif" => a.gif = Some(value()?.into()),
            "--time" => a.time = value()?.parse()?,
            "--seconds" => a.seconds = value()?.parse()?,
            "--fps" => a.fps = value()?.parse()?,
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or(anyhow!("--size is WxH, like 1200x480"))?;
                a.size = (w.parse()?, h.parse()?);
            }
            "-h" | "--help" => {
                println!("{}", USAGE.trim());
                std::process::exit(0);
            }
            s => bail!("unknown option {s}\n{}", USAGE.trim()),
        }
    }
    Ok(a)
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let mut feeds = Layered::new();
    if let Some(dir) = &args.data {
        feeds = feeds.then(Folder::new(dir));
    }
    let feeds = feeds.then(glowdots_scenes::samples());

    let exporting = args.screenshot.is_some() || args.gif.is_some();
    let start = match &args.now {
        Some(s) => Some(time::parse_local(s).map_err(|e| anyhow!(e))?),
        None => None,
    };
    // Exports run on a stepped clock, so they come out the same every time.
    let clock = match (start, exporting) {
        (Some(s), true) => Clock::stepped(s),
        (None, true) => Clock::stepped(glowdots::chrono::Local::now()),
        (Some(s), false) => Clock::starting_at(s),
        (None, false) => Clock::system(),
    };

    let mut deck =
        Deck::new(glowdots_scenes::SIZE, glowdots_scenes::showcase()?)?.with_shades(args.shades).with_feeds(feeds).with_clock(clock);
    if args.list {
        println!("{}", deck.names().join("\n"));
        return Ok(());
    }
    if let Some(name) = &args.scene {
        deck.select(name)?;
    }
    deck.hold = args.hold;
    if !exporting {
        deck.on_scene = Some(Box::new(|name| eprintln!("scene: {name}")));
    }
    let mut player = Player::new(deck).with_glass(args.glass).with_rotate(args.rotate);

    if exporting {
        player.progress_line = false;
        let panel = player.deck.size();
        return app::offscreen(panel, args.size, |r| {
            if let Some(path) = &args.screenshot {
                export::png(&mut player, r, args.time, path)?;
                eprintln!("saved {}", path.display());
            }
            if let Some(path) = &args.gif {
                player.skip(args.time - player.time());
                export::gif(&mut player, r, args.seconds, args.fps, path)?;
                eprintln!("saved {}", path.display());
            }
            Ok(())
        });
    }
    if args.kiosk {
        #[cfg(feature = "sdl")]
        return glowdots::sdl::run(player, Default::default());
        #[cfg(not(feature = "sdl"))]
        bail!("--kiosk needs SDL: build with --features sdl");
    }
    app::run(player, Options { fullscreen: args.fullscreen, ..Default::default() })
}
