#!/usr/bin/env python3
"""Generate smooth, bold digits for the clock scene.

Each digit is a stroked path in a 10 x 18 dot box, drawn at D px per dot.
The program averages each D x D block into one dot, so the curves get
soft, anti-aliased edges in the DMD's shades, like late pinball fonts.

Output: crates/glowdots-scenes/assets/fonts/digits/{0..9,colon}.png

Usage: python3 tools/big_digits.py   (needs rsvg-convert)
"""

import shutil
import subprocess
from pathlib import Path

D = 8
W, H = 10, 18
STROKE = 2.1  # in dots
OUT = Path(__file__).resolve().parent.parent / "crates" / "glowdots-scenes" / "assets" / "fonts" / "digits"

SIX = ("M 7.5,1.5 H 5 A 3.5,4 0 0 0 1.5,5.5 V 13 A 3.5,3.5 0 0 0 5,16.5 "
       "A 3.5,3.5 0 0 0 8.5,13 V 12 A 3.5,3.5 0 0 0 5,8.5 H 1.5")

# Centre lines of each glyph, in dot units.
GLYPHS = {
    "0": ['<path d="M 4.5,1.5 H 5.5 A 3,3 0 0 1 8.5,4.5 V 13.5 A 3,3 0 0 1 5.5,16.5 '
          'H 4.5 A 3,3 0 0 1 1.5,13.5 V 4.5 A 3,3 0 0 1 4.5,1.5 Z"/>'],
    "1": ['<path d="M 2.8,4.4 L 5.8,1.5 V 16.5"/>'],
    "2": ['<path d="M 1.5,4.5 A 3,3 0 0 1 4.5,1.5 H 5.5 A 3,3 0 0 1 8.5,4.5 V 6 '
          'C 8.5,8 7.5,9 5.5,10 L 1.5,12.5 V 16.5 H 8.5"/>'],
    "3": ['<path d="M 1.5,1.5 H 5.5 A 3,3.75 0 0 1 5.5,9 H 3.5 M 5.5,9 '
          'A 3,3.75 0 0 1 5.5,16.5 H 1.5"/>'],
    "4": ['<path d="M 6.5,16.5 V 1.5 L 1.5,12 H 8.5"/>'],
    "5": ['<path d="M 8.5,1.5 H 2 L 1.5,8.5 H 5.5 A 3,4 0 0 1 5.5,16.5 H 1.5"/>'],
    "6": [f'<path d="{SIX}"/>'],
    "7": ['<path d="M 1.5,1.5 H 8.5 L 4,16.5"/>'],
    "8": ['<ellipse cx="5" cy="5.25" rx="3.1" ry="3.75"/>',
          '<ellipse cx="5" cy="12.75" rx="3.5" ry="3.75"/>'],
    "9": [f'<path d="{SIX}" transform="rotate(180 5 9)"/>'],
}


def svg(width, body):
    w, h = width * D, H * D
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {width} {H}">'
            f'<rect width="{width}" height="{H}" fill="#000"/>'
            f'<g fill="none" stroke="#fff" stroke-width="{STROKE}" stroke-linecap="round" '
            f'stroke-linejoin="round">{body}</g></svg>')


def main():
    shutil.rmtree(OUT, ignore_errors=True)
    OUT.mkdir(parents=True)
    files = {name: svg(W, "".join(parts)) for name, parts in GLYPHS.items()}
    # The colon is two round dots in a narrow box.
    files["colon"] = svg(4, '<circle cx="2" cy="6" r="0.1" stroke-width="2.8"/>'
                            '<circle cx="2" cy="12.5" r="0.1" stroke-width="2.8"/>')
    for name, text in files.items():
        path = OUT / f"{name}.svg"
        path.write_text(text)
        subprocess.run(["rsvg-convert", str(path), "-o", str(path.with_suffix(".png"))], check=True)
        path.unlink()
    print(f"wrote {len(files)} glyphs to {OUT}")


if __name__ == "__main__":
    main()
