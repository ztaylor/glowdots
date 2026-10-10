# Contributing

Thanks for wanting to help. New scenes, new looks, fixes, and better docs are all welcome.

## Build and test

```sh
cargo run --release                      # the showcase
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

The web demo and the Raspberry Pi player have their own steps: see [docs/web.md](docs/web.md) and [docs/raspberry-pi.md](docs/raspberry-pi.md).

## Pictures

The pictures in the README come from the real scenes. After a change that shows, make them again:

```sh
cargo run --release --bin media
```

The launch video is made the same way, with a soundtrack synthesized to its cuts (it needs ffmpeg):

```sh
cargo run --release --bin promo -- glowdots-promo.mp4
```

## A new example scene

Example scenes live in `crates/glowdots-scenes`. A good one shows a pattern other people can take: say in its doc comment what it shows and what feed it reads, add a sample feed to `samples/`, add it to `showcase()`, and check that the test that runs every scene still passes. Keep text inside its area at the panel size; the existing scenes have tests that check long values fit.

## Pull requests

Keep a pull request to one change, say what it changes and why, and include a picture or GIF when it changes how something looks.
