#!/usr/bin/env python3
"""Read / set the backlight effect on a NYXI Striker keyboard over Linux hidraw.

Protocol (from the official app's dist/index.js, class hw / CK, and iot_driver):
  * Commands are 64-byte HID *feature* reports on the vendor interface
    (USB interface 2, usage page 0xFFFF / usage 2), report ID 0.
  * GET_LEDPARAM = 0x87, SET_LEDPARAM = 0x07.
  * Checksum "BIT7"/"BIT8": byte at index 7 / 8 = 0xFF - (sum of bytes before it).
    GETs use BIT7, SET_LEDPARAM uses BIT8.
  * A GET is: send feature report with the command, then read a feature report back.

LED param layout (SET and GET reply):
  [0] cmd  [1] mode  [2] 4 - speed  [3] brightness 0..4
  [4] (option << 4) | color   color: 0..6 preset, 7 = custom RGB, 8 = rainbow ("dazzle")
  [5..7] R G B
"""
import argparse
import fcntl
import glob
import os
import sys
import time

VID, PID_WIRED, PID_DONGLE = 0x3151, 0x4015, 0x4011
CMD_SET_LEDPARAM, CMD_GET_LEDPARAM = 0x07, 0x87
MAXSPEED = 4
NORMAL, DAZZLE = 7, 8

MODES = {
    0: "LightOff", 1: "LightAlwaysOn", 2: "LightBreath", 3: "LightNeon",
    4: "LightWave", 5: "LightRipple", 6: "LightRaindrop", 7: "LightSnake",
    8: "LightPressAction", 9: "LightConverage", 10: "LightSineWave",
    11: "LightKaleidoscope", 12: "LightLineWave", 13: "LightUserPicture",
    14: "LightLaser", 15: "LightCircleWave", 16: "LightDazzing", 17: "LightRainDown",
    18: "LightMeteor", 19: "LightPressActionOff", 20: "LightMusicFollow3",
    21: "LightScreenColor", 22: "LightMusicFollow2", 23: "LightTrain", 24: "LightFireWorks",
    66: "LightUserColor",
}
PRESET_COLORS = {0: "red", 1: "orange", 2: "yellow", 3: "green", 4: "cyan", 5: "blue", 6: "purple"}


def _ioc(direction, nr, size):
    return (direction << 30) | (size << 16) | (ord("H") << 8) | nr


def HIDIOCSFEATURE(n):
    return _ioc(3, 0x06, n)


def HIDIOCGFEATURE(n):
    return _ioc(3, 0x07, n)


def find_control_hidraw():
    """Return /dev/hidrawN for the keyboard's vendor control interface (interface 2)."""
    for node in sorted(glob.glob("/sys/class/hidraw/hidraw*")):
        uevent = open(os.path.join(node, "device", "uevent")).read()
        if f"{VID:08X}:{PID_WIRED:08X}" not in uevent and f"{VID:08X}:{PID_DONGLE:08X}" not in uevent:
            continue
        # The wired keyboard exposes 3 interfaces; the control one has a short (20-byte)
        # vendor descriptor with usage page 0xFFFF, usage 0x02 and a 64-byte feature report.
        rdesc = open(os.path.join(node, "device", "report_descriptor"), "rb").read()
        if (rdesc[:3] == b"\x06\xff\xff" and b"\x09\x02" in rdesc[:6]) or "/input2\n" in uevent:
            return "/dev/" + os.path.basename(node)
    return None


def checksum(buf, pos):
    buf[pos] = (0xFF - sum(buf[:pos])) & 0xFF


class Keyboard:
    def __init__(self, path):
        self.path = path
        self.fd = os.open(path, os.O_RDWR)

    def set_feature(self, payload):
        buf = bytearray(65)
        buf[1:1 + len(payload)] = payload
        fcntl.ioctl(self.fd, HIDIOCSFEATURE(65), bytes(buf))

    def get_feature(self):
        buf = bytearray(65)
        fcntl.ioctl(self.fd, HIDIOCGFEATURE(65), buf, True)
        return bytes(buf[1:])

    def query(self, cmd, delay=0.02):
        p = bytearray(64)
        p[0] = cmd
        checksum(p, 7)
        self.set_feature(p)
        time.sleep(delay)
        return self.get_feature()

    def get_led(self):
        r = self.query(CMD_GET_LEDPARAM)
        if r[0] != CMD_GET_LEDPARAM:
            raise RuntimeError(f"unexpected reply: {r[:16].hex(' ')}")
        return r

    def set_led(self, mode, speed=3, brightness=4, option=0, color=DAZZLE, rgb=(255, 255, 255)):
        p = bytearray(64)
        p[0] = CMD_SET_LEDPARAM
        p[1] = mode
        p[2] = MAXSPEED - speed
        p[3] = brightness
        p[4] = ((option & 0xF) << 4) | (color & 0xF)
        p[5:8] = bytes(rgb)
        checksum(p, 8)
        self.set_feature(p)


def describe(r):
    mode, speed, bright, opt = r[1], MAXSPEED - r[2], r[3], r[4]
    color = opt & 0xF
    if color == DAZZLE:
        cdesc = "rainbow"
    elif color == NORMAL:
        cdesc = f"custom #{r[5]:02x}{r[6]:02x}{r[7]:02x}"
    else:
        cdesc = f"preset {PRESET_COLORS.get(color, color)}"
    return (f"mode={mode} ({MODES.get(mode, '?')})  speed={speed}/4  brightness={bright}/4  "
            f"option={opt >> 4}  color={cdesc}")


def mode_from_arg(s):
    if s.isdigit():
        return int(s)
    for k, v in MODES.items():
        if v.lower() in (s.lower(), "light" + s.lower()):
            return k
    sys.exit(f"unknown mode {s!r}; choose from: " + ", ".join(v[5:] for v in MODES.values()))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dev", help="hidraw path (default: auto-detect interface 2)")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("get", help="read the current backlight effect")
    sub.add_parser("modes", help="list mode names")
    s = sub.add_parser("set", help="set the backlight effect")
    s.add_argument("mode", help="mode name (e.g. Wave, Snake, AlwaysOn) or number")
    s.add_argument("--speed", type=int, help="0..4 (default: keep current)")
    s.add_argument("--brightness", type=int, help="0..4 (default: keep current)")
    s.add_argument("--option", type=int, help="direction/variant 0..15 (default: keep current)")
    g = s.add_mutually_exclusive_group()
    g.add_argument("--rgb", help="custom color, e.g. ff0000")
    g.add_argument("--preset", type=int, choices=range(7), help="preset color 0..6")
    g.add_argument("--rainbow", action="store_true", help="rainbow / dazzle colors")
    args = ap.parse_args()

    if args.cmd == "modes":
        for k, v in MODES.items():
            print(f"{k:3d}  {v[5:]}")
        return

    path = args.dev or find_control_hidraw()
    if not path:
        sys.exit("NYXI control interface (USB interface 2) not found. Is usbhid bound to it? "
                 "Try unplugging/replugging the keyboard.")
    kb = Keyboard(path)

    cur = kb.get_led()
    if args.cmd == "get":
        print(f"{path}: {describe(cur)}")
        print("raw:", cur[:9].hex(" "))
        return

    rgb, color = tuple(cur[5:8]), cur[4] & 0xF
    if args.rgb:
        rgb, color = tuple(bytes.fromhex(args.rgb)), NORMAL
    elif args.preset is not None:
        color = args.preset
    elif args.rainbow:
        color = DAZZLE
    kb.set_led(
        mode_from_arg(args.mode),
        speed=args.speed if args.speed is not None else MAXSPEED - cur[2],
        brightness=args.brightness if args.brightness is not None else cur[3],
        option=args.option if args.option is not None else cur[4] >> 4,
        color=color,
        rgb=rgb,
    )
    time.sleep(0.2)
    print("before:", describe(cur))
    print("after: ", describe(kb.get_led()))


if __name__ == "__main__":
    main()
