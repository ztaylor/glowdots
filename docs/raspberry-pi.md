# Raspberry Pi

glowdots can run full screen on a Raspberry Pi with no desktop. SDL's KMSDRM driver draws straight to the panel, so the Pi boots to a text console and the display starts by itself. It is developed on a Raspberry Pi 5 with the 5 inch Touch Display 2, and works with any screen the Pi drives.

## Build

On Raspberry Pi OS Lite (64-bit):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install git libsdl2-dev
git clone https://github.com/ztaylor/glowdots
cd glowdots
cargo build --release -p glowdots-showcase --features sdl
```

## Run

From the console or over SSH, not inside a desktop session:

```sh
./target/release/glowdots --kiosk
./target/release/glowdots --kiosk --rotate 1 --data ~/feeds
```

- `--kiosk` plays full screen through SDL. Taps on a touch screen move between scenes: the right half forward, the left half back.
- `--rotate` turns the picture a quarter turn at a time. The Touch Display 2 is a portrait panel, so mounted the long way it needs `--rotate 1` (or 3, upside down).
- `--data` reads your feeds from a folder. Write them with cron, a script, or anything else; scenes read them again each time they come around.

Only one program can drive the screen. Stop the service (below) before you run glowdots by hand.

## Start at boot

[`tools/glowdots.service`](../tools/glowdots.service) starts the display at boot and restarts it if it stops. Change the user and paths in it to yours, then:

```sh
sudo cp tools/glowdots.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now glowdots
journalctl -u glowdots -f        # its log: scenes, frame rate, feed errors
```

After a new build, `sudo systemctl restart glowdots`.

## Your own program

Use the `sdl` feature and play your deck with `glowdots::sdl::run`:

```toml
[dependencies]
glowdots = { version = "0.1", default-features = false, features = ["sdl"] }
```

```rust
let player = Player::new(deck).with_rotate(1);
glowdots::sdl::run(player, Default::default())?;
```

`glowdots::sdl::offscreen` renders to an image instead, for screenshots on the Pi itself.

## If the screen stays black

- Check the log for the video driver line. It should say `KMSDRM`; if SDL picked another driver, set `SDL_VIDEODRIVER=kmsdrm` (the service file does).
- Make sure no desktop session holds the screen: boot to the console, or stop the display manager.
- The user needs access to the GPU: the `video` and `render` groups. The default user on Raspberry Pi OS has them.
