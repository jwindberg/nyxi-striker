#!/usr/bin/env python3
"""Generate the 9x9 screen animation library (animations/*.txt).

File format (read by nyxi_screen.py): lines starting with ';' are metadata
('; name: ...', '; delay: N' in ~20 ms units, '; about: ...'); the rest is
frames of 9 lines each, '#' = lit, '.' = off, separated by blank lines.
The keyboard takes at most 50 frames per animation.
"""
import math
import os
import random

W = H = 9
MAX_FRAMES = 50
OUT = os.path.dirname(os.path.abspath(__file__))


def blank():
    return [[0] * W for _ in range(H)]


def put(g, x, y):
    x, y = int(round(x)), int(round(y))
    if 0 <= x < W and 0 <= y < H:
        g[y][x] = 1


def sprite(g, rows, ox, oy):
    for dy, row in enumerate(rows):
        for dx, c in enumerate(row):
            if c == "#":
                put(g, ox + dx, oy + dy)


def parse(text):
    return [[1 if c == "#" else 0 for c in line] for line in text.strip().splitlines()]


def text_of(g):
    return "\n".join("".join("#" if v else "." for v in row) for row in g)


def save(slug, name, delay, about, frames):
    assert 1 <= len(frames) <= MAX_FRAMES, (slug, len(frames))
    with open(os.path.join(OUT, slug + ".txt"), "w") as f:
        f.write(f"; name: {name}\n; delay: {delay}\n; about: {about}\n\n")
        f.write("\n\n".join(text_of(g) for g in frames) + "\n")
    print(f"{slug:12} {len(frames):2} frames  delay {delay}")


# ---------------------------------------------------------------- shapes

HEART_BIG = parse("""
.........
.##...##.
#########
#########
#########
.#######.
..#####..
...###...
....#....
""")

HEART_SMALL = parse("""
.........
.........
..##.##..
.#######.
.#######.
..#####..
...###...
....#....
.........
""")

SMILEY = """
..#####..
.#.....#.
#..#.#..#
#.......#
#.#...#.#
#..###..#
.#.....#.
..#####..
.........
"""

INVADER_A = ["...##...", "..####..", ".######.", "##.##.##", "########", "..#..#..", ".#.##.#.", "#.#..#.#"]
INVADER_B = ["...##...", "..####..", ".######.", "##.##.##", "########", ".#.##.#.", "#......#", ".#....#."]

PAC_OPEN = [".###.", "###..", "##...", "###..", ".###."]
PAC_SHUT = [".###.", "#####", "#####", "#####", ".###."]

FONT = {  # 7 rows tall
    "1": [".#.", "##.", ".#.", ".#.", ".#.", ".#.", "###"],
    "U": ["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    "P": ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
    "N": ["#...#", "##..#", "##..#", "#.#.#", "#..##", "#..##", "#...#"],
    "Y": ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
    "X": ["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"],
    "I": ["###", ".#.", ".#.", ".#.", ".#.", ".#.", "###"],
}


# ---------------------------------------------------------------- animations

def heartbeat():
    seq = [HEART_SMALL, HEART_BIG, HEART_SMALL, HEART_BIG, HEART_BIG, HEART_SMALL, HEART_SMALL, HEART_SMALL]
    save("heartbeat", "Heartbeat", 8, "A heart pulsing lub-dub", [[r[:] for r in g] for g in seq])


def spinner():
    frames = []
    for pts in ([(4, y) for y in range(W)], [(i, W - 1 - i) for i in range(W)],
                [(x, 4) for x in range(W)], [(i, i) for i in range(W)]):
        g = blank()
        for x, y in pts:
            put(g, x, y)
        frames.append(g)
    save("spinner", "Spinner", 10, "A line turning like a propeller", frames)


def bouncing_ball():
    frames = []
    hop = [7, 4, 2, 1, 1, 2, 4]          # one bounce, ball top row
    for t in range(14):
        x = t if t <= 7 else 14 - t      # 0..7..1 back and forth
        y = hop[t % 7]
        g = blank()
        for dx in (0, 1):
            for dy in (0, 1):
                put(g, x + dx, y + dy)
        frames.append(g)
    save("ball", "Bouncing ball", 7, "A ball bouncing across the screen", frames)


def rain():
    rng = random.Random(3)
    period = 12
    drops = [(x, rng.randrange(period)) for x in range(W)] + [(rng.randrange(W), rng.randrange(period)) for _ in range(3)]
    frames = []
    for t in range(period):
        g = blank()
        for x, phase in drops:
            y = (t + phase) % period - 2
            put(g, x, y)
            put(g, x, y - 1)
        frames.append(g)
    save("rain", "Rain", 4, "Raindrops falling", frames)


def matrix():
    rng = random.Random(7)
    period = 18
    cols = [(x, rng.choice((1, 2)), rng.randrange(period), rng.choice((3, 4, 5))) for x in range(W)]
    frames = []
    for t in range(period):
        g = blank()
        for x, speed, phase, length in cols:
            head = (t * speed + phase) % period - 2
            for k in range(length):
                if k == 0 or (k + t) % 3:     # flicker the trail a little
                    put(g, x, head - k)
        frames.append(g)
    save("matrix", "Matrix", 9, "Falling code, Matrix style", frames)


def glider():
    cells = {(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)}
    frames = []
    for _ in range(36):                  # glider moves (1,1) every 4 gens; 36 gens wraps the 9x9 torus
        g = blank()
        for x, y in cells:
            g[y][x] = 1
        frames.append(g)
        counts = {}
        for x, y in cells:
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    if dx or dy:
                        k = ((x + dx) % W, (y + dy) % H)
                        counts[k] = counts.get(k, 0) + 1
        cells = {k for k, n in counts.items() if n == 3 or (n == 2 and k in cells)}
    save("glider", "Game of Life glider", 6, "Conway's glider crossing the screen forever", frames)


def invader():
    frames = []
    for shape, x in ((INVADER_A, 0), (INVADER_B, 1), (INVADER_A, 1), (INVADER_B, 0)):
        g = blank()
        sprite(g, shape, x, 0)
        frames.append(g)
    save("invader", "Space invader", 20, "A marching space invader", frames)


def pacman():
    frames = []
    dots = list(range(1, W, 3))
    for t, x in enumerate(range(-5, W + 1)):
        g = blank()
        for d in dots:
            if d > x + 2:                # dots not yet eaten
                put(g, d, 4)
        sprite(g, PAC_OPEN if t % 2 == 0 else PAC_SHUT, x, 2)
        frames.append(g)
    save("pacman", "Pac-Man", 10, "Pac-Man eating a row of dots", frames)


def scroll(slug, name, about, text, delay=4):
    glyphs = [FONT[c] for c in text]
    width = sum(len(gl[0]) + 1 for gl in glyphs) - 1
    frames = []
    for off in range(W, -width - 1, -1):
        g = blank()
        x = off
        for gl in glyphs:
            sprite(g, gl, x, 1)
            x += len(gl[0]) + 1
        frames.append(g)
    save(slug, name, delay, about, frames)


def ripple():
    frames = []
    for r in range(7):
        g = blank()
        for y in range(H):
            for x in range(W):
                if abs(math.hypot(x - 4, y - 4) - r) < 0.5:
                    g[y][x] = 1
        frames.append(g)
    save("ripple", "Ripple", 10, "Rings spreading from a drop in a pond", frames)


def warp():
    rng = random.Random(11)
    period = 10
    stars = [(rng.uniform(0, 2 * math.pi), rng.randrange(period)) for _ in range(10)]
    frames = []
    for t in range(period):
        g = blank()
        for angle, phase in stars:
            d = ((t + phase) % period) ** 1.5 * 0.2     # accelerate outward
            put(g, 4 + d * math.cos(angle), 4 + d * math.sin(angle))
        frames.append(g)
    save("warp", "Warp speed", 5, "Stars streaking past at warp speed", frames)


def fireworks():
    frames = []
    dirs = [(1, 0), (-1, 0), (0, 1), (0, -1), (0.7, 0.7), (-0.7, 0.7), (0.7, -0.7), (-0.7, -0.7)]
    for cx, top in ((4, 3), (2, 2), (6, 4)):
        for y in range(8, top, -1):                    # rocket climbing
            g = blank()
            put(g, cx, y)
            frames.append(g)
        for r in (1, 2, 3, 3.5):                       # burst
            g = blank()
            for i, (dx, dy) in enumerate(dirs):
                if r < 3 or i % 2 == 0:                # thin out as it fades
                    put(g, cx + dx * r, top + dy * r + (1 if r > 3 else 0))
            frames.append(g)
        frames.append(blank())
    save("fireworks", "Fireworks", 9, "Three rockets bursting", frames)


def equalizer():
    rng = random.Random(5)
    heights = [rng.randint(1, 8) for _ in range(W)]
    frames = []
    for _ in range(16):
        heights = [max(1, min(9, h + rng.choice((-2, -1, 1, 2)))) for h in heights]
        g = blank()
        for x, h in enumerate(heights):
            for y in range(H - h, H):
                g[y][x] = 1
        frames.append(g)
    save("equalizer", "Equalizer", 5, "Music equalizer bars", frames)


def eye():
    outline = ["..#####..", ".#.....#.", "#.......#", "#.......#", "#.......#", "#.......#", "#.......#", ".#.....#.", "..#####.."]
    lid = [".........", ".........", ".........", ".........", "#########", ".........", ".........", ".........", "........."]

    iris = [".#.", "###", "###", ".#."]

    def look(px, py):
        g = blank()
        sprite(g, outline, 0, 0)
        sprite(g, iris, px - 1, py - 2)
        return g

    shut = blank()
    sprite(shut, lid, 0, 0)
    seq = [look(4, 4)] * 4 + [look(2, 4)] * 4 + [look(4, 4)] * 2 + [look(6, 4)] * 4 + [look(4, 4)] * 3 + [shut] + [look(4, 4)] * 2 + [look(4, 3)] * 3
    save("eye", "Watching eye", 8, "An eye that looks around and blinks", [[r[:] for r in g] for g in seq])


def wave():
    frames = []
    for t in range(W):
        g = blank()
        for x in range(W):
            put(g, x, 4 + 3.5 * math.sin(2 * math.pi * (x + t) / W))
            put(g, x, 4 + 3.5 * math.sin(2 * math.pi * (x - t) / W + math.pi))
        frames.append(g)
    save("wave", "Sine waves", 5, "Two sine waves rolling past each other", frames)


def snake():
    # A closed loop that winds back and forth through the square, then returns up the left edge.
    path = ([(x, 1) for x in range(1, 8)] + [(7, 2)] + [(x, 3) for x in range(7, 0, -1)] + [(1, 4)]
            + [(x, 5) for x in range(1, 8)] + [(7, 6)] + [(x, 7) for x in range(7, -1, -1)]
            + [(0, y) for y in range(6, 0, -1)])
    frames = []
    for t in range(len(path)):
        g = blank()
        for k in range(6):
            x, y = path[(t - k) % len(path)]
            put(g, x, y)
        frames.append(g)
    save("snake", "Snake", 7, "A snake winding through the square", frames)


def dino():
    run_a = ["..##", "..##", "###.", ".#.#"]
    run_b = ["..##", "..##", "###.", "#..#"]
    jump = [0, 0, 0, 0, 0, 0, 2, 3, 3, 2, 0, 0]        # dino height by frame
    frames = []
    for t in range(12):
        g = blank()
        for x in range(W):
            put(g, x, 8)                                # ground
        cx = 9 - t                                      # cactus runs right to left
        for y in (5, 6, 7):
            put(g, cx, y)
        put(g, cx - 1, 6)
        sprite(g, run_a if t % 2 else run_b, 1, 4 - jump[t])
        frames.append(g)
    save("dino", "Dino run", 6, "The offline dinosaur jumping a cactus", frames)


def tetris():
    board = set()
    frames = []

    def snap(extra=()):
        g = blank()
        for x, y in board | set(extra):
            put(g, x, y)
        frames.append(g)

    def drop(cells, x0):
        piece = [(x + x0, y) for x, y in cells]
        top = -max(y for _, y in piece) - 1
        y = top
        while all((x, py + y + 1) not in board and py + y + 1 < H for x, py in piece):
            y += 1
            if (y - top) % 2 == 0:                      # draw every other row to keep it short
                snap([(x, py + y) for x, py in piece])
        board.update((x, py + y) for x, py in piece)
        snap()

    def clear():
        full = [y for y in range(H) if all((x, y) in board for x in range(W))]
        if not full:
            return
        board.difference_update((x, y) for x in range(W) for y in full)
        snap()                                          # flash: rows gone
        for y in sorted(full):
            moved = {(x, by + 1) if by < y else (x, by) for x, by in board}
            board.clear()
            board.update(moved)
        snap()

    O = [(0, 0), (1, 0), (0, 1), (1, 1)]
    I = [(0, 0), (0, 1), (0, 2), (0, 3)]
    drop(I, 8)
    for x in (0, 2, 4, 6):
        drop(O, x)
    clear()
    for x in (0, 2, 4, 6):
        drop(O, x)
    clear()
    save("tetris", "Tetris", 9, "Blocks falling and clearing lines", frames)


def pulse():
    frames = []
    for s in (0, 1, 2, 3, 4, 3, 2, 1):
        g = blank()
        for i in range(4 - s, 5 + s):
            for j in (4 - s, 4 + s):
                put(g, i, j)
                put(g, j, i)
        frames.append(g)
    save("pulse", "Pulse", 6, "A square breathing in and out", frames)


def loading():
    frames = []
    for n in list(range(8)) + [7, 7]:
        g = blank()
        for x in range(W):
            put(g, x, 3)
            put(g, x, 5)
        put(g, 0, 4)
        put(g, 8, 4)
        for x in range(1, 1 + n):
            put(g, x, 4)
        frames.append(g)
    save("loading", "Loading", 10, "A progress bar filling up", frames)


def wink():
    open_ = parse(SMILEY)
    winked = [r[:] for r in open_]
    winked[2] = [1 if c == "#" else 0 for c in "#..#....#"]
    winked[3] = [1 if c == "#" else 0 for c in "#....##.#"]
    save("wink", "Winking smiley", 25, "A smiley that winks", [open_, open_, open_, winked])


# ---------------------------------------------------------------- still images (one frame)

IMAGES = {
    # "0" sorts it first: by convention the collection starts with a blank screen.
    "img-0-blank": ("Blank", "Nothing lit: the screen is dark", "\n".join(["." * W] * H)),
    "img-heart": ("Heart", "A heart", "\n".join(text_of(HEART_BIG).splitlines())),
    "img-smiley": ("Smiley", "A smiling face", SMILEY),
    "img-star": ("Star", "A five-pointed star", """
....#....
....#....
...###...
#########
.#######.
..#####..
..##.##..
.##...##.
##.....##
"""),
    "img-note": ("Music", "A pair of music notes", """
...#####.
...#####.
...#...#.
...#...#.
...#...#.
.###.###.
####.####
####.####
.##...##.
"""),
    "img-check": ("Check", "A check mark", """
.........
........#
.......##
......##.
#....##..
##..##...
.####....
..##.....
.........
"""),
    "img-cross": ("Cross", "A big X", """
#.......#
.#.....#.
..#...#..
...#.#...
....#....
...#.#...
..#...#..
.#.....#.
#.......#
"""),
    "img-arrow": ("Arrow", "An arrow pointing up", """
....#....
...###...
..#.#.#..
.#..#..#.
#...#...#
....#....
....#....
....#....
....#....
"""),
    "img-ghost": ("Ghost", "A Pac-Man ghost", """
...###...
.#######.
#########
#..##..##
#..##..##
#########
#########
#########
#.##.##.#
"""),
    "img-skull": ("Skull", "A skull", """
.#######.
#########
#..###..#
#..###..#
#########
###.#.###
.#######.
..#.#.#..
.........
"""),
    "img-sun": ("Sun", "A shining sun", """
#...#...#
.#.....#.
...###...
..#####..
#.#####.#
..#####..
...###...
.#.....#.
#...#...#
"""),
    "img-moon": ("Moon", "A crescent moon", """
...###...
..##.....
.##......
.##......
.##......
.##......
.##......
..##.....
...###...
"""),
    "img-bolt": ("Lightning", "A lightning bolt", """
.....##..
....##...
...##....
..######.
.....##..
....##...
...##....
..##.....
.#.......
"""),
    "img-cat": ("Cat", "A cat's face", """
#.......#
##.....##
#########
##.###.##
#########
###.#.###
.##...##.
..#####..
.........
"""),
    "img-house": ("House", "A little house", """
....#....
...###...
..#####..
.#######.
#########
.#.....#.
.#.##..#.
.#.##..#.
.#######.
"""),
    "img-coffee": ("Coffee", "A steaming cup of coffee", """
..#.#.#..
.#.#.#...
.........
#######..
#.....###
#.....#.#
#.....###
.#...#...
..###....
"""),
    "img-diamond": ("Diamond", "A diamond", """
....#....
...###...
..#####..
.#######.
#########
.#######.
..#####..
...###...
....#....
"""),
    "img-hi": ("Hi", "The word HI", """
.........
#..#.###.
#..#..#..
#..#..#..
####..#..
#..#..#..
#..#..#..
#..#.###.
.........
"""),
}


def images():
    for slug, (name, about, art) in IMAGES.items():
        g = parse(art)
        assert len(g) == H and all(len(r) == W for r in g), slug
        save(slug, name, 0, about, [g])


if __name__ == "__main__":
    images()
    for make in (heartbeat, spinner, bouncing_ball, rain, matrix, glider, invader, pacman, ripple, warp,
                 fireworks, equalizer, eye, wave, snake, tetris, pulse, loading, wink):
        make()
