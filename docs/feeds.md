# Feeding data

Scenes read JSON feeds through [`Feeds`](https://docs.rs/glowdots/latest/glowdots/feed/trait.Feeds.html). Something else writes the data; a scene reads its feed again each time it comes around.

## Where feeds come from

```rust
use glowdots::{Bundled, Folder, Layered};

// Files in a folder: "events.json" is feeds/events.json.
let feeds = Folder::new("feeds");

// Or in memory: samples built into the program, or data from anywhere.
let feeds = Bundled::new().with("events.json", r#"{"items": []}"#);

// Or several, in order: each feed comes from the first that has it.
let feeds = Layered::new().then(Folder::new("feeds")).then(glowdots_scenes::samples());

let deck = deck.with_feeds(feeds);
```

The showcase does the last one: `--data DIR` reads your folder first, then the samples.

## Reading a feed in a scene

```rust
#[derive(serde::Deserialize)]
struct Score { home: u32, away: u32 }

fn enter(&mut self, cx: &Context) {
    match cx.json::<Score>("score.json") {
        Ok(score) => self.score = Some(score),
        Err(e) => self.message = e.short(), // "NO FEED YET", or "BAD FEED: LINE 3"
    }
}
```

`cx.json` reads any JSON into your type. `cx.items` reads a list feed:

```json
{"date": "2026-10-09", "items": [ ... ]}
```

`date` is optional. If it is there and it is not today, the feed is `Feed::Stale`, and the card scenes say "NOT UPDATED" instead of showing old items. Leave it out for feeds that do not go out of date.

When a feed is missing or broken, show it on the display. People will edit these files by hand, and "BAD FEED: LINE 3" on the panel saves them a trip to a log.

## Writing feeds

Anything that can write a file can feed the display. Write to a temporary file and rename it, so a scene never reads half a file.

```sh
# Every 10 minutes from cron: the weather from your own service, reshaped with jq.
curl -s https://example.com/my-weather | jq '{now: {temp_f: .temp, icon: .icon}}' > feeds/weather.json.tmp \
  && mv feeds/weather.json.tmp feeds/weather.json
```

```python
import json, os
items = [{"title": "Pinball league night", "venue": "The Silverball Arcade", "time": "7:30 PM"}]
with open("feeds/events.json.tmp", "w") as f:
    json.dump({"items": items}, f)
os.replace("feeds/events.json.tmp", "feeds/events.json")
```

## The example scenes' feeds

Each scene says what it reads in its docs, and [`crates/glowdots-scenes/samples`](../crates/glowdots-scenes/samples) has a full example of each. Every field is optional unless it says otherwise.

| Feed | Scene | Shape |
|---|---|---|
| `events.json` | events | List feed. Items: `title` (needed), `venue`, `time`, `category`. |
| `sports.json` | sports | List feed. Items: `league`, `away` and `home`, or `title` for a race or a match; `time`, `channel`, `note`. |
| `news.json` | news | `sections`: each an `id`, a `title`, and `items` with `headline`, `blurb`, `tag`, `source`. Headlines up to 60 characters fit the board. |
| `status.json` | status | `generated_at` (RFC 3339), `status` (ok, warn, or error), and `checks`, each a `group`, `name`, `status`, and `detail`. Older than 30 minutes shows as silent. |
| `weather.json` | weather, lake | `now`, `today`, `tomorrow`, `next_12h`, and `alerts`. `icon` is clear, partly_cloudy, cloudy, fog, drizzle, rain, sleet, snow, or thunder. |
| `lake.json` | lake | `windows` (dawn and dusk), `pressure`, `moon`, `sunrise_today`, `sunset_today`, and `ice` (true for the winter scene). |

Every example scene takes other feed names and titles, so you can run two of them on different data, the way the showcase runs the lake twice:

```rust
lake::Lake::new().named("winter").title("MIRROR LAKE").feeds("lake_winter.json", "weather_winter.json")
```
