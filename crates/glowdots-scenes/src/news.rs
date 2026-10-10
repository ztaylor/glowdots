//! "The Wire": a news board. Headlines land on a split-flap board.
//!
//! Each visit shows one story from each section (for example AI news,
//! GitHub, Hacker News, papers), and the next story on the next visit. A
//! story is:
//!
//! 1. The headline on a split-flap board: every letter flips through
//!    random characters and lands in a wave, so the whole headline shows
//!    at once, word-wrapped (up to 4 lines).
//! 2. The board flips again, to the blurb. A blurb longer than 4 lines
//!    rolls up one line at a time, so the text never moves while you
//!    read it. (Small text that crawls smears on a plasma display.)
//!
//! A "lower third" bar, like TV news, shows the section and the story's
//! place in it ("2/5") with the headline, and its tag and source with the
//! blurb.
//!
//! It reads `news.json` (see `samples/news.json`): sections, each with an
//! id, a title, and items with a headline, blurb, tag, and source. Write
//! headlines of 60 characters or less so they fit the board.

use glowdots::font::{REGULAR, SMALL, clean};
use glowdots::math::{ease_in_out, hash};
use glowdots::{Context, Frame, MAX, Rect, Scene, Size, Sprite, Transition, ui};
use serde::Deserialize;

const WIDTH: usize = crate::SIZE.w as usize;

/// Split-flap timing: each letter starts a little after the one before.
const FLIP_START: f32 = 0.15;
const FLIP_SECONDS: f32 = 0.3;
const HEADLINE_STAGGER: f32 = 0.022;
const BLURB_STAGGER: f32 = 0.012;
const FLAP_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#%&@$";
/// Time to read, after the letters land.
const HEADLINE_READ: f32 = 3.5;
const BLURB_READ: f32 = 4.0;
/// A long blurb rolls up one line this often, taking ROLL_SECONDS to move.
const LINE_SECONDS: f32 = 1.8;
const ROLL_SECONDS: f32 = 0.25;
/// The board: up to 4 lines in rows 0..BOARD_BOTTOM.
const BOARD_LINES: usize = 4;
const BOARD_BOTTOM: i32 = 24;
const LINE_HEIGHT: i32 = 6;
/// The lower-third bar: rows BAR_TOP..32. Text sits one row down.
const BAR_TOP: i32 = 25;
const BAR_TEXT_X: i32 = 7;

#[derive(Deserialize)]
struct FeedFile {
    sections: Vec<FeedSection>,
}

#[derive(Deserialize)]
struct FeedSection {
    id: Option<String>,
    title: Option<String>,
    items: Vec<FeedItem>,
}

#[derive(Deserialize)]
struct FeedItem {
    headline: String,
    blurb: Option<String>,
    tag: Option<String>,
    source: Option<String>,
}

/// Text laid out on the board as letters at fixed places.
struct Board {
    /// (letter, x, y), in reading order.
    cells: Vec<(char, i32, i32)>,
    lines: usize,
}

impl Board {
    /// Lay out lines from the top of the board (y = row of the first line).
    fn new(lines: &[String], top: i32) -> Self {
        let mut cells = Vec::new();
        for (n, line) in lines.iter().enumerate() {
            let mut x = 2;
            for c in line.chars() {
                if c != ' ' {
                    cells.push((c, x, top + n as i32 * LINE_HEIGHT));
                }
                x += SMALL.char_width(c) + 1;
            }
        }
        Self { cells, lines: lines.len() }
    }

    /// Lay out lines centred in the board's height.
    fn centred(lines: &[String]) -> Self {
        let height = lines.len() as i32 * LINE_HEIGHT - 1;
        Self::new(lines, (BOARD_BOTTOM - height) / 2)
    }

    /// Seconds until the last letter lands.
    fn landed(&self, stagger: f32) -> f32 {
        FLIP_START + self.cells.len() as f32 * stagger + FLIP_SECONDS
    }

    /// Draw the board `t` seconds after it starts to flip in. Letters
    /// land in reading order; until then they show random flaps.
    fn draw(&self, out: &mut Frame, t: f32, stagger: f32, dy: i32) {
        let clip = Rect::new(0, 0, WIDTH as i32, BOARD_BOTTOM);
        for (i, &(c, x, y)) in self.cells.iter().enumerate() {
            let start = FLIP_START + i as f32 * stagger;
            if t < start {
                continue;
            }
            let (glyph, level) = if t < start + FLIP_SECONDS {
                let k = (hash(i as u32, (t * 30.0) as u32) * FLAP_CHARS.len() as f32) as usize;
                (FLAP_CHARS[k.min(FLAP_CHARS.len() - 1)] as char, 2)
            } else {
                (c, MAX)
            };
            SMALL.draw_clip(out, &glyph.to_string(), x, y - dy, level, clip);
        }
    }
}

struct Story {
    headline: Board,
    /// All lines of the blurb; the first 4 flip in, the rest roll up.
    blurb: Board,
    tag: String,
    source: String,
}

impl Story {
    fn new(item: &FeedItem) -> Self {
        let width = WIDTH as i32 - 4;
        let text = |s: &Option<String>| clean(s.as_deref().unwrap_or(""));
        let head = SMALL.wrap(&clean(&item.headline), width);
        let blurb = SMALL.wrap(&text(&item.blurb), width);
        // A blurb that fits is centred; a long one starts at the top and rolls.
        let blurb = if blurb.len() <= BOARD_LINES { Board::centred(&blurb) } else { Board::new(&blurb, 0) };
        Self { headline: Board::centred(&head[..head.len().min(BOARD_LINES)]), blurb, tag: text(&item.tag), source: text(&item.source) }
    }

    /// When the blurb starts to flip in.
    fn blurb_at(&self) -> f32 {
        self.headline.landed(HEADLINE_STAGGER) + HEADLINE_READ
    }

    /// Lines the blurb has beyond the board, which roll up.
    fn extra_lines(&self) -> usize {
        self.blurb.lines.saturating_sub(BOARD_LINES)
    }

    fn seconds(&self) -> f32 {
        self.blurb_at() + self.blurb.landed(BLURB_STAGGER) + BLURB_READ + self.extra_lines() as f32 * LINE_SECONDS
    }
}

struct Section {
    /// Short name for the bar.
    label: String,
    icon: Sprite,
}

enum Step {
    /// A story: its section, and its place there ("2/5").
    Story {
        section: usize,
        place: String,
        story: Story,
    },
    Message(String, String),
}

impl Step {
    fn seconds(&self) -> f32 {
        match self {
            Step::Story { story, .. } => story.seconds(),
            Step::Message(..) => 6.0,
        }
    }
}

pub struct News {
    name: String,
    title: String,
    feed_name: String,
    waiting: Option<String>,
    /// Section ids, with the label and icon for the bar.
    known: Vec<(String, String, Sprite)>,
    visit: usize,
    sections: Vec<Section>,
    steps: Vec<Step>,
}

impl News {
    pub fn new() -> Self {
        let known = [
            ("ai_news", "AI NEWS", &SPARK[..]),
            ("github", "GITHUB", &BRANCH[..]),
            ("hacker_news", "HACKER NEWS", &Y_BOX[..]),
            ("papers", "PAPERS", &PAPER[..]),
        ]
        .map(|(id, label, icon)| (id.to_string(), label.to_string(), Sprite::from_rows(icon)))
        .into();
        Self {
            name: "news".into(),
            title: "THE WIRE".into(),
            feed_name: "news.json".into(),
            waiting: None,
            known,
            visit: 0,
            sections: Vec::new(),
            steps: Vec::new(),
        }
    }

    /// Call the scene something else (for logs and choosing it).
    pub fn named(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    /// The board's name, shown when there is no news ("THE WIRE").
    pub fn title(mut self, title: &str) -> Self {
        self.title = clean(title);
        self
    }

    /// Read another feed than `news.json`.
    pub fn feed(mut self, name: &str) -> Self {
        self.feed_name = name.into();
        self
    }

    /// What to say under "NO NEWS YET" when the feed is missing or bad. By
    /// default it says what is wrong ("NO FEED YET", "BAD FEED: LINE 3").
    pub fn waiting(mut self, text: &str) -> Self {
        self.waiting = Some(text.into());
        self
    }

    /// The label and 5-dot-tall icon for a section id in the bar. The
    /// icon is a dot map ('#' full, '=' mid, '.' dim). Sections with no
    /// entry use their title and a spark.
    pub fn section(mut self, id: &str, label: &str, icon: &[&str]) -> Self {
        self.known.retain(|k| k.0 != id);
        self.known.push((id.into(), clean(label), Sprite::from_rows(icon)));
        self
    }

    fn load(&mut self, cx: &Context) -> Result<(), String> {
        let feed: FeedFile = cx.json(&self.feed_name).map_err(|e| self.waiting.clone().unwrap_or_else(|| e.short()))?;

        self.sections.clear();
        self.steps.clear();
        for s in feed.sections.iter().filter(|s| !s.items.is_empty()) {
            let id = s.id.clone().unwrap_or_default();
            let title = clean(s.title.as_deref().unwrap_or(&id));
            let index = self.sections.len();
            let section = match self.known.iter().find(|k| k.0 == id) {
                Some((_, label, icon)) => Section { label: label.clone(), icon: icon.clone() },
                None => Section { label: title, icon: Sprite::from_rows(&SPARK) },
            };
            self.sections.push(section);
            // The next story in this section on each visit.
            let n = self.visit % s.items.len();
            let place = format!("{}/{}", n + 1, s.items.len());
            self.steps.push(Step::Story { section: index, place, story: Story::new(&s.items[n]) });
        }
        if self.steps.is_empty() {
            return Err(self.waiting.clone().unwrap_or_else(|| "NO STORIES IN THE FEED".into()));
        }
        Ok(())
    }

    fn story(&self, s: &Section, place: &str, story: &Story, t: f32, out: &mut Frame) {
        let w = WIDTH as i32;
        let blurb_t = t - story.blurb_at();

        if blurb_t < 0.0 {
            story.headline.draw(out, t, HEADLINE_STAGGER, 0);
        } else {
            // Then the blurb. Once it has landed and been read, extra lines
            // roll up one at a time.
            let landed = story.blurb.landed(BLURB_STAGGER);
            let rolling = blurb_t - landed - BLURB_READ * 0.6;
            let mut dy = 0;
            if rolling > 0.0 && story.extra_lines() > 0 {
                let step = (rolling / LINE_SECONDS).min(story.extra_lines() as f32);
                let whole = step.floor();
                let part = ease_in_out(((step - whole) * LINE_SECONDS / ROLL_SECONDS).min(1.0));
                let rolled = if whole as usize >= story.extra_lines() { whole } else { whole + part };
                dy = (rolled * LINE_HEIGHT as f32).round() as i32;
            }
            if dy == 0 {
                // Only the lines on the board flip in.
                let on_board = story.blurb.cells.iter().filter(|c| c.2 < BOARD_BOTTOM).count();
                let board = Board { cells: story.blurb.cells[..on_board].to_vec(), lines: 0 };
                board.draw(out, blurb_t, BLURB_STAGGER, 0);
            } else {
                story.blurb.draw(out, f32::MAX, BLURB_STAGGER, dy);
            }
        }

        // The lower third: a dim bar with the section's icon. With the
        // headline: the section and the story's place in it. With the
        // blurb: the story's tag and source.
        out.fill(Rect::new(0, BAR_TOP, w, 32), 1);
        out.blit(&s.icon, 1, BAR_TOP + 1);
        let y = BAR_TOP + 1;
        let (left, right) = if blurb_t < 0.0 || story.tag.is_empty() {
            (s.label.as_str(), place)
        } else {
            (story.tag.as_str(), if story.source == s.label { "" } else { story.source.as_str() })
        };
        SMALL.draw(out, left, BAR_TEXT_X, y, MAX);
        let room = w - 2 - (BAR_TEXT_X + SMALL.width(left) + 4);
        if SMALL.width(right) <= room {
            SMALL.draw_right(out, right, w - 2, y, 2);
        }
    }
}

impl Scene for News {
    fn name(&self) -> &str {
        &self.name
    }

    fn enter(&mut self, cx: &Context) {
        if let Err(e) = self.load(cx) {
            self.steps = vec![Step::Message("NO NEWS YET".into(), e)];
        }
        self.visit += 1;
    }

    fn update(&mut self, cx: &Context, out: &mut Frame) {
        let t = cx.t;
        let mut local = t;
        let mut current = self.steps.len() - 1;
        for (i, step) in self.steps.iter().enumerate() {
            if local < step.seconds() {
                current = i;
                break;
            }
            local -= step.seconds();
        }
        match &self.steps[current] {
            Step::Story { section, place, story } => self.story(&self.sections[*section], place, story, local, out),
            Step::Message(a, b) => {
                ui::header(out, &self.title, "");
                REGULAR.draw_centered(out, a, 12, MAX);
                SMALL.draw_centered(out, b, 23, 2);
            }
        }
    }

    fn duration(&self) -> f32 {
        self.steps.iter().map(|s| s.seconds()).sum()
    }

    fn transition(&self) -> Transition {
        Transition::Wipe
    }

    fn size(&self) -> Option<Size> {
        Some(crate::SIZE)
    }
}

impl Default for News {
    fn default() -> Self {
        Self::new()
    }
}

// Bar icons, 5 dots tall: '#' full, '=' mid, '.' dim, ' ' off.
// A spark (AI news), a git branch, a Y in a box (Hacker News), a page.

const SPARK: [&str; 5] = ["  #  ", " ### ", "#####", " ### ", "  #  "];

const BRANCH: [&str; 5] = ["#  #", "#  #", "# # ", "##  ", "#   "];

const Y_BOX: [&str; 5] = ["#####", "# # #", "## ##", "## ##", "#####"];

const PAPER: [&str; 5] = ["### ", "# ##", "#  #", "#  #", "####"];

#[cfg(test)]
mod tests {
    use super::*;

    /// Real headlines from the digest, up to its 60-character limit, must
    /// fit the 4-line board.
    #[test]
    fn headlines_fit_the_board() {
        for h in [
            "cathrynlavery/diagram-design: SVG diagrams for agents",
            "pbakaus/impeccable: design language for AI harness",
            "LoGRA: low-rank gradient sketches for RL scaling",
            "Websites are blocking AI shopping/booking agents",
            "OpenTPU: open-source AI accelerator built by AI",
            "thedotmack/claude-mem: persistent agent memory",
            "A sixty character headline with many medium sized words here",
        ] {
            let lines = SMALL.wrap(&clean(h), WIDTH as i32 - 4);
            assert!(lines.len() <= BOARD_LINES, "{h:?} -> {lines:?}");
        }
    }
}
