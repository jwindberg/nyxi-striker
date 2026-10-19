#!/usr/bin/env python3
"""Upload images / animations to the NYXI CA01 9x9 dot-matrix screen.

Protocol (from the official app: Vx.setTFTLCDData + bitmap packer tJ):
  packet = [0x24, frameCount-1 (0 if static), chunk, frameIndex, delay, layer, 0, csum] + 56 data bytes
  csum   = 0xFF - sum(bytes 0..6)
  bitmap = for each 8-row band, for each column: byte with bit n = pixel (band*8+n, column)
           9x9 -> 18 bytes (9 bytes for rows 0-7, then 9 bytes for row 8)

Frames are text files / strings: 9 lines of 9 chars, '#' (or any non-space/'.') = lit.
Multiple frames are separated by a blank line. Lines starting with ';' are metadata,
e.g. '; delay: 8' (frame delay, ~20 ms units). The animations/ library uses this format.
"""
import argparse
import glob
import os
import sys
import time

from nyxi_led import Keyboard, checksum, find_control_hidraw

CMD_SET_OLEDGIFDATA = 0x24
W = H = 9

SMILEY = """\
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

HEART = """\
.........
.##...##.
#########
#########
#########
.#######.
..#####..
...###...
....#....
"""


BLANK = "\n".join(["." * W] * H)

LIBRARY = os.path.join(os.path.dirname(os.path.abspath(__file__)), "animations")


def parse_meta(text):
    meta = {}
    for line in text.splitlines():
        if line.startswith(";") and ":" in line:
            key, _, value = line[1:].partition(":")
            meta[key.strip()] = value.strip()
    return meta


def parse_frames(text):
    frames, cur = [], []
    for line in text.splitlines() + [""]:
        if line.startswith(";"):
            continue
        if line.strip():
            cur.append([0 if c in " ." else 1 for c in line.rstrip().ljust(W, ".")[:W]])
        elif cur:
            frames.append((cur + [[0] * W] * H)[:H])
            cur = []
    return frames


def pack(frame):
    out = []
    for band in range((H + 7) // 8):
        for x in range(W):
            b = 0
            for n in range(8):
                y = band * 8 + n
                if y < H and frame[y][x]:
                    b |= 1 << n
            out.append(b)
    return out


def upload(kb, frames, delay=50, layer=0):
    animated = len(frames) > 1
    for i, frame in enumerate(frames):
        data = pack(frame)
        for chunk in range((len(data) + 55) // 56):
            p = bytearray(64)
            p[0] = CMD_SET_OLEDGIFDATA
            p[1] = len(frames) - 1 if animated else 0
            p[2] = chunk
            p[3] = i
            p[4] = delay if animated else 0
            p[5] = layer
            checksum(p, 7)
            body = data[chunk * 56:chunk * 56 + 56]
            p[8:8 + len(body)] = bytes(body)
            kb.set_feature(p)
            time.sleep(0.01)


def library():
    return sorted(glob.glob(os.path.join(LIBRARY, "*.txt")))


def load(source):
    """Built-in name, animations/ library name, or a file path -> text."""
    text = {"smiley": SMILEY, "heart": HEART, "blank": BLANK}.get(source)
    if text is not None:
        return text
    path = os.path.join(LIBRARY, source + ".txt")
    return open(path if os.path.exists(path) else source).read()


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("source", help="'smiley', 'heart', 'blank', an animations/ name, a text file of 9x9 frames, "
                                   "'list' to show the library, or 'demo' to play through it")
    ap.add_argument("--delay", type=int, help="animation frame delay 0..255, ~20 ms units (default: file's, else 50)")
    ap.add_argument("--seconds", type=float, default=6, help="demo: seconds per animation (default 6)")
    ap.add_argument("--layer", type=int, default=0)
    ap.add_argument("--dev")
    args = ap.parse_args()

    if args.source == "list":
        for path in library():
            meta = parse_meta(open(path).read())
            print(f"{os.path.basename(path)[:-4]:12} {meta.get('name', '')}: {meta.get('about', '')}")
        return

    kb = Keyboard(args.dev or find_control_hidraw())
    sources = [os.path.basename(p)[:-4] for p in library()] if args.source == "demo" else [args.source]
    for source in sources:
        text = load(source)
        frames = parse_frames(text)
        if not frames:
            sys.exit(f"no frames found in {source}")
        delay = args.delay if args.delay is not None else int(parse_meta(text).get("delay", 50))
        upload(kb, frames, delay, args.layer)
        print(f"{source}: uploaded {len(frames)} frame(s), delay {delay}", flush=True)
        if args.source == "demo":
            time.sleep(args.seconds)


if __name__ == "__main__":
    main()
