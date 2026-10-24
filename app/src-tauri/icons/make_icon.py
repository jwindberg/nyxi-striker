#!/usr/bin/env python3
"""Draw the app icon: the Striker's 9x9 dot screen showing a lightning bolt, in the app's
colors. Writes icon-source.png; then run `npx tauri icon src-tauri/icons/icon-source.png`
from app/ to make every size."""
from pathlib import Path
from PIL import Image, ImageDraw

BOLT = """
.....##..
....##...
...##....
..######.
.....##..
....##...
...##....
..##.....
.#.......
""".split()

SIZE = 1024
S = 4  # draw larger, then shrink for smooth edges
img = Image.new("RGBA", (SIZE * S, SIZE * S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
d.rounded_rectangle([40 * S, 40 * S, (SIZE - 40) * S, (SIZE - 40) * S], radius=200 * S, fill="#20225a", outline="#f5c400", width=28 * S)
pitch, dot, start = 80, 58, (SIZE - 80 * 8) // 2
for y, row in enumerate(BOLT):
    for x, c in enumerate(row):
        cx, cy = (start + x * pitch) * S, (start + y * pitch) * S
        d.ellipse([cx - dot * S // 2, cy - dot * S // 2, cx + dot * S // 2, cy + dot * S // 2], fill="#f5c400" if c == "#" else "#34377f")
img.resize((SIZE, SIZE), Image.LANCZOS).save(Path(__file__).with_name("icon-source.png"))
