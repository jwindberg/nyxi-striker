# NYXI Striker Keyboard Control Protocol

Last updated 2026-10-17. Living copy (with comments): https://claude.ai/code/artifact/6ff822f3-1b26-429c-9f84-1afd0db36134

## Overview

Everything on the NYXI Striker (model CA01) is configured with 64-byte HID feature reports sent to USB interface 2, each carrying a one-byte command and a simple checksum. Reading and setting the backlight and uploading images to the 9x9 dot-matrix screen were verified on real hardware on 2026-10-17 (Linux, firmware 1.05). The protocol comes from NYXI's official Windows app, an Electron front end plus a Rust helper (`iot_driver.exe`). Its JavaScript (`extracted_asar/dist/index.js`) builds every packet, so it is the reference for anything not yet verified.

What can be controlled, and how sure we are:

| Feature | Commands | Status |
| --- | --- | --- |
| Backlight effect, speed, brightness, color | 0x07 set / 0x87 get | Verified read and write |
| 9x9 dot-matrix screen, custom slot | 0x24 upload | Verified (still images and animations) |
| Second lighting zone | 0x08 set / 0x88 get | Read verified; write from app code |
| Firmware version, profile, polling rate, debounce, sleep | 0x80, 0x85, 0x84, 0x91, 0x92 get | Read verified |
| Key remap, macros, setting polling rate, debounce, sleep | 0x13, 0x16, 0x04, 0x11, 0x12 | From app code only |
| Choosing the screen's built-in animation | none found | Hardware key Fn + Left Ctrl only |

Minimal working example (Python, Linux): read the current effect, then switch to Wave.

```python
import fcntl, os, time

def ioc(nr, size):  # _IOC(_IOC_READ|_IOC_WRITE, 'H', nr, size)
    return (3 << 30) | (size << 16) | (ord('H') << 8) | nr

fd = os.open('/dev/hidraw9', os.O_RDWR)  # the interface-2 node; see USB section

def send(payload, csum_pos):
    p = bytearray(64); p[:len(payload)] = payload
    p[csum_pos] = (0xFF - sum(p[:csum_pos])) & 0xFF
    fcntl.ioctl(fd, ioc(0x06, 65), bytes([0]) + bytes(p))  # HIDIOCSFEATURE

def get(cmd):
    send([cmd], 7); time.sleep(0.02)
    buf = bytearray(65); fcntl.ioctl(fd, ioc(0x07, 65), buf, True)  # HIDIOCGFEATURE
    return bytes(buf[1:])

print(get(0x87)[:8].hex(' '))        # e.g. 87 07 02 04 08 b4 b4 b4 = Snake, rainbow
send([0x07, 4, 4 - 3, 4, 0x08], 8)  # Wave, speed 3, brightness 4, rainbow
```

Working tools in the project folder: `nyxi_led.py` (backlight), `nyxi_screen.py` (screen), `nyxi-app.sh` (official app on Linux).

## USB device and interfaces

Only interface 2 takes configuration commands; interfaces 0 and 1 are the normal keyboard and media-key channels and ignore them. Wired, the keyboard enumerates as `3151:4015` ("ROYUAN NYXI CA01", bcdDevice 1.05); through the 2.4 GHz dongle it is `3151:4011`. Internally the app calls the model `yc500_nyxift98_soc` (group `rongyuan_k_rgb`, a Rongyuan/Yichip OEM design shared by many brands).

| Interface | HID usage | Endpoint | Role |
| --- | --- | --- | --- |
| 0 | 0x0001 / 0x06 (keyboard) | 0x81 IN, 8 bytes | Typing (boot keyboard) |
| 1 | 0x000C consumer + vendor collections | 0x82 IN, 16 bytes | Media keys and vendor event reports |
| 2 | 0xFFFF / 0x02 (vendor) | 0x83 IN, 8 bytes | Configuration: 64-byte feature reports |

How to find the right device node:

- Linux hidraw: the node whose sysfs `device/uevent` shows `HID_PHYS=...input2`, or whose report descriptor starts `06 FF FF 09 02`.
- hidapi (any OS): `vendor_id 0x3151`, `usage_page 0xFFFF`, `usage 0x02` (interface number 2 where reported).

Linux setup:

1. Permissions, once: `/etc/udev/rules.d/99-nyxi.rules` containing `SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3151", MODE="0666"`, then `sudo udevadm control --reload-rules` and replug.
2. Driver binding pitfall: if interface 2 has no hidraw node, `usbhid` is not bound to it (a libusb tool that detached it and never reattached will cause this). Check `readlink /sys/bus/usb/devices/<port>:1.2/driver`; fix with `echo <port>:1.2 | sudo tee /sys/bus/usb/drivers/usbhid/bind` or by replugging. On the test machine the port was `1-2.3`.

macOS note: an earlier attempt saw empty (all-zero) replies to GET requests on macOS and blamed the OS. That attempt also used the wrong checksum, so retest with the correct one before assuming a macOS limitation.

## Transport and packet format

Every command is one 64-byte feature report with no report ID: byte 0 is the command, and a checksum byte sits at index 7 or 8. Interface 2's report descriptor (`06 FF FF 09 02 A1 01 09 02 15 80 25 7F 95 40 75 08 B1 02 C0`) defines exactly one 64-byte feature report. APIs that expect a report-ID prefix (hidraw ioctls, hidapi) get a 65-byte buffer starting with `0x00`.

- **SET (write):** `HIDIOCSFEATURE(65)` / `hid_send_feature_report` with `[0x00] + payload[64]`.
- **GET (read):** send the GET command as a SET, wait about 5 to 20 ms, then `HIDIOCGFEATURE(65)` / `hid_get_feature_report` into a 65-byte buffer. The reply's byte 0 echoes the command.
- **Unsupported GET:** the keyboard returns your request unchanged (for example `AB 00 00 00 00 00 00 54`), or the previous reply if the command is unknown. Treat a reply that equals the request, or whose byte 0 differs from the command, as unsupported.
- **Pacing:** the official app waits 10 ms before most sends, 500 ms before and after SET_LEDPARAM (`COMMONDELAY`), and 1000 ms (`BIGCMDDELAY`) after multi-packet writes such as key maps. Back-to-back sends without delays worked in testing, but follow the app for bulk writes.
- **Wireless dongle:** the app adds 60 ms per send for Bluetooth and uses a different path for 2.4 GHz; none of this was tested.

### Checksum

The checksum makes the bytes up to and including it sum to 0xFF (mod 256). The app names the two variants `BIT7` and `BIT8` after the index where the checksum byte goes:

```
buf[k] = (0xFF - (buf[0] + buf[1] + ... + buf[k-1])) mod 256,   k = 7 or 8
```

| Type | Checksum at | Used by |
| --- | --- | --- |
| BIT7 (enum 0) | byte 7, over bytes 0..6 | All GETs, key remap and macro chunks, screen upload |
| BIT8 (enum 1) | byte 8, over bytes 0..7 | SET_LEDPARAM and SET_SLEDPARAM (data fills bytes 1..7) |
| NONE (enum 2) | none | Raw writes that carry their own checksum |

Example: GET_LEDPARAM is `87 00 00 00 00 00 00 78` followed by 56 zero bytes (0x87 + 0x78 = 0xFF). The JavaScript computes this itself in places (`u[7] = 255 - (255 & sum)` in the key-map writer), which confirms the formula. Earlier notes said `sum & 0x7F` at byte 63; that is wrong and makes the keyboard ignore every command.

## Command reference

A GET command is its SET command plus 0x80, and every GET uses checksum BIT7. The values below are what the Striker's device class (`CK`, via its parents `Qz`, `hw` and `Kx`) actually uses. Other NYXI models reuse the same names with different numbers, so don't copy constants from other classes in the app.

| Set | Get | Name in app | Payload summary | Status |
| --- | --- | --- | --- | --- |
| 0x04 | 0x84 | REPORT (polling rate) | [1] profile, [2] 1=1000 Hz, 2=500, 4=250, 8=125 | Verified (set 500, read back, restored) |
| 0x05 | 0x85 | PROFILE | [1] profile 0..2 | Get verified (reads 0) |
| 0x06 | 0x86 | KBOPTION | Option bits, see Other settings | Get verified; set verified for the Windows-key lock and Mac-mode bits |
| 0x07 | 0x87 | LEDPARAM (backlight) | See Backlight; set uses BIT8 | Verified |
| 0x08 | 0x88 | SLEDPARAM (second zone) | Same layout as LEDPARAM; set uses BIT8 | Get verified |
| 0x0A | 0x8A | KEYENABLE | Not decoded | Unverified |
| 0x0C | 0x8C | USERPIC | Per-key color picture being shown, see Per-key custom colors | Verified (uploaded, read back, restored) |
| 0x0D | — | AUDIO | Music-reactive lighting frames | Unverified |
| 0x0E | — | WINDOS | Screen-color lighting frames | Unverified |
| 0x11 | 0x91 | DEBOUNCE | Set [2] ms; get reply [2] | Verified (set 3, read back, restored to 1) |
| 0x12 | 0x92 | SLEEPTIME | 16-bit seconds, see Other settings | Verified (changed, read back, restored) |
| 0x13 | 0x93 | KEYMATRIX_SIMPLE | One key remap, see Other settings | Verified (set, read back with 0x89, restored) |
| 0x14 | 0x94 | USERPIC_SIMPLE | One key's custom color: [1] picture, [2] key slot, [8..10] RGB | Single writes verified; drops writes in quick succession, use 0x0C |
| 0x15 | 0x95 | FN_SIMPLE | One Fn-layer key remap | Verified (set, read back with 0x90, restored) |
| 0x16 | 0x96 | MACRO_SIMPLE | Macro body in 56-byte chunks; read back with 0x8B | Write verified |
| 0x17 | 0x97 | CMD_AUTOOSEN | Not decoded | Get verified (reads 0) |
| 0x24 | — | OLEDGIFDATA (9x9 screen) | See Dot-matrix screen | Verified |
| 0x2B | 0xAB | OLEDEFFECT | For other models' screens | Ignored (SET has no effect; GET returns a byte unrelated to the animation) |
| — | 0x80 | REV (firmware) | Reply [1..2] little-endian, 0x0105 = 1.05 | Verified |
| — | 0x83 | BATTERY | Reply [1] (reads 0 when wired) | Read only, meaning unverified |
| — | 0x8F | INFOR (device id) | Reply [1..2] little-endian, 0x05C1 = 1473 = Striker wired | Verified |

Commands that can damage or reset the keyboard are listed under Hazards. Never send them while exploring.

## Backlight (LEDPARAM)

One 64-byte packet sets the whole per-key backlight: command 0x07, mode, speed, brightness, option/color byte, RGB, then the BIT8 checksum at byte 8. GET 0x87 returns the same layout (with 0x87 in byte 0 and no checksum at byte 8), so read, change one field, and write back. Both directions were verified on hardware.

| Byte | Meaning | Values |
| --- | --- | --- |
| 0 | Command | 0x07 set, 0x87 get |
| 1 | Mode | See the mode table |
| 2 | Inverted speed | 4 − speed; speed 0 (slowest) to 4 (fastest), so raw 0 = fastest |
| 3 | Brightness | 0 (off) to 4 (max) |
| 4 | Option and color | High nibble = option (mode-specific); low nibble = color mode |
| 5, 6, 7 | Red, green, blue | 0 to 255; used when the color mode is 7 |
| 8 | Checksum (SET only) | 0xFF − (sum of bytes 0..7) mod 256 |
| 9..63 | Unused | 0 |

Color mode (low nibble of byte 4):

| Value | Meaning |
| --- | --- |
| 0..6 | Preset color: 0 red FF0000, 1 orange FF8000, 2 yellow FFFF00, 3 green 00FF00, 4 cyan 00FFFF, 5 blue 0000FF, 6 purple FF00FF |
| 7 | Custom RGB from bytes 5..7 (app name `NORMAL`) |
| 8 | Rainbow / multicolor (app name `DAZZLE`) |

Every effect uses the color except Off, Neon and the per-key picture (13), going by the official app's settings for the Striker (`rgb`/`dazzle` flags) and by its Neon packet, which has no color or option. The app only writes 7 or 8; presets appear when they are chosen with Fn keys. Writing a preset value is untested. Quirk: the app sends pure white FFFFFF as FAFFFA and maps it back on read.

Modes (byte 1) and their options (high nibble of byte 4). "Offered" means the app shows the mode for the Striker.

| Mode | App name | Option nibble | Offered |
| --- | --- | --- | --- |
| 0 | LightOff | — | Yes |
| 1 | LightAlwaysOn (solid) | — | Yes |
| 2 | LightBreath | — | Yes |
| 3 | LightNeon | 0 default, 1 random | Yes |
| 4 | LightWave | 0 right, 1 left, 2 down, 3 up | Yes |
| 5 | LightRipple | 0 full, 1 single (read only) | Yes |
| 6 | LightRaindrop | — | Yes |
| 7 | LightSnake | 0 "z" path, 1 "return" path | Yes |
| 8 | LightPressAction (reactive) | 0 on to off, 1 off to on (read only) | Yes |
| 9 | LightConverage | — | Yes |
| 10 | LightSineWave | — | Yes |
| 11 | LightKaleidoscope | 0 out, 1 in | Yes |
| 12 | LightLineWave | 0 right, 1 left | Yes |
| 13 | LightUserPicture (per-key colors) | 0, 1, 2 for pictures 1..3 (byte 4 = 0x00, 0x10, 0x20; color nibble 0); app sends RGB 00 C8 C8 | Yes |
| 14 | LightLaser | — | Yes |
| 15 | LightCircleWave | 0 anti-clockwise, 1 clockwise | Yes |
| 16 | LightDazzing | — | Yes |
| 17 | LightRainDown | — | Yes |
| 18 | LightMeteor | — | Yes |
| 19 | LightPressActionOff | — | Yes |
| 20 | LightMusicFollow3 (music reactive) | 0 upright, 1 separate, 2 intersect | Yes |
| 21 | LightScreenColor (follows screen) | —; brightness fixed at 4 | Yes |
| 22 | LightMusicFollow2 | 0 upright, 1 separate, 2 intersect | Yes |
| 23, 24, 66 | LightTrain, LightFireWorks, LightUserColor | FireWorks: 0 right, 1 left | Not for Striker |

Modes 20 to 22 only animate while the host streams data (commands 0x0D and 0x0E); the app does that, a bare SET does not. For those modes the app writes color mode 0 for rainbow and 4 otherwise, not 7 or 8.

Worked examples (first 9 bytes; bytes 9..63 are zero, and the 65-byte buffer sent to the OS starts with an extra 0x00):

| Intent | Bytes 0..8 |
| --- | --- |
| Read current setting | `87 00 00 00 00 00 00 78` (GET, BIT7 checksum at byte 7) |
| Reply seen for Snake, speed 2, max brightness, rainbow | `87 07 02 04 08 B4 B4 B4 00` |
| Wave, speed 3, brightness 4, rainbow | `07 04 01 04 08 00 00 00 E7` |
| Solid red, brightness 4 | `07 01 04 04 07 FF 00 00 E9` |
| Wave leftward, speed 4, custom blue | `07 04 00 04 17 00 00 FF DA` |
| Lights off | `07 00 04 00 07 00 00 00 ED` |

The app sends speed 0 (raw byte 4) for modes without a speed setting, as in the solid-red and off rows.

## Second lighting zone (SLEDPARAM)

The keyboard keeps a second, independent lighting setting under commands 0x08 (set) and 0x88 (get). It uses the backlight's byte layout with two differences: the speed byte is not inverted, and only six modes exist. On the Striker it appears to drive nothing: writing it from the new app (several modes and colors, with the "sLedOff" flag clear) produced no visible change, and NYXI's app does not show a second zone for this model. The keyboard-option bits include a matching "sLedOff" flag.

| Byte | Meaning | Values |
| --- | --- | --- |
| 0 | Command | 0x08 set, 0x88 get |
| 1 | Mode | 0 off, 1 solid, 2 breath, 3 neon, 4 wave, 5 snake |
| 2 | Speed, NOT inverted | 0 to 4 as-is |
| 3 | Brightness | 0 to 4 |
| 4 | Option and color | Same nibbles as the backlight: wave direction 0..3, snake 0/1; color 7 custom, 8 rainbow |
| 5, 6, 7 | Red, green, blue | Custom color |
| 8 | Checksum (SET only) | BIT8, as for the backlight |

Read on the test keyboard: `88 04 03 04 08 FF FF FF` = wave, speed 3, brightness 4, rainbow.

One trap in the app: when asked to put this zone into a music or screen mode, it actually sends a backlight command (0x07 with mode 20, 21 or 22). Writing to 0x08 has not been tested.

## 9x9 dot-matrix screen

The screen is a 9x9 grid of single-color dots. The computer can only replace its one custom slot, which takes a still image or an animation sent with command 0x24. Verified on hardware: still images and multi-frame animations display correctly, and an upload immediately switches the screen to the custom slot, whatever it was showing. The built-in animations (jumping bird, Pac-Man, cat) can only be chosen on the keyboard, so a host app that wants a preset picker should upload its own animations.

### Upload packet

One packet per frame: an 8-byte header, then the frame's bitmap in bytes 8..63. A 9x9 frame needs 18 bytes, so it always fits in one packet.

| Byte | Meaning | Still image | Animation |
| --- | --- | --- | --- |
| 0 | Command | 0x24 | 0x24 |
| 1 | Frame count minus 1 | 0 | N − 1 |
| 2 | Chunk index within the frame | 0 | 0 |
| 3 | Frame index | 0 | 0 to N − 1 |
| 4 | Frame delay | 0 | 0 to 255, about 20 ms per unit (50 ≈ 1 frame/s, 10 ≈ 5 frames/s, observed by eye); app default 50 |
| 5 | Layer | 0 | 0 |
| 6 | Unused | 0 | 0 |
| 7 | Checksum, BIT7 | 0xFF − sum of bytes 0..6 | same |
| 8..25 | Bitmap, 18 bytes | see below | see below |
| 26..63 | Padding | 0 | 0 |

For an animation, send frames 0, 1, …, N − 1 in order, one packet each; the app waits 1 ms between them. The app allows up to 50 frames for this screen size. Verified: a 4-frame animation played in a loop. The keyboard plays every frame at one speed: a test with delay 10 on frame 0 and 100 on frame 1 showed both frames for the same, slow time (about 2 s each), so per-frame delays are not honored: the keyboard used the last frame's delay, or the largest (the test cannot tell which). Send the same delay in every frame.

### Bitmap packing

A dot is on when its bit is 1. Rows are y = 0 (top) to 8, columns x = 0 (left) to 8. The bytes come in two bands of 9: first rows 0..7, then row 8. Within a band, one byte per column from left to right, and bit n of that byte is row (band × 8 + n). The least significant bit is the top row of the band.

```
for band in 0, 1:                    # rows 0-7, then row 8
    for x in 0..8:                   # columns left to right
        byte = 0
        for n in 0..7:
            y = band * 8 + n
            if y <= 8 and pixel[y][x]: byte |= 1 << n
        output byte
```

Worked example, the heart that was uploaded during testing:

```
.........     row 0
.##...##.     row 1
#########     row 2
#########     row 3
#########     row 4
.#######.     row 5
..#####..     row 6
...###...     row 7
....#....     row 8
```

Full packet, bytes 0..25 (the rest are zero):

`24 00 00 00 00 00 00 DB 1C 3E 7E FC FC FC 7E 3E 1C 00 00 00 00 01 00 00 00 00`

Column 0 is `1C` (rows 2, 3, 4 lit), and the second band's `01` at column 4 is the single dot at the bottom tip.

### What the computer cannot do (as far as is known)

- **Choose a built-in animation.** Fn + Left Ctrl cycles: jumping bird → custom slot → Pac-Man → cat → jumping bird (Fn + Right Ctrl does nothing). No host command for it is known: the OLEDEFFECT commands (0x2B/0xAB) are ignored. Earlier checks for a USB event or a changed setting were made while pressing Right Ctrl, so they did not actually cycle and should be repeated with Left Ctrl.
- **Remove the custom slot.** It stays in the rotation. The nearest thing to deleting it is uploading an all-blank frame.
- **Turn the screen off.** Fn + Page Down turns it on and off on the keyboard; no host command is known. Both screen shortcuts are firmware functions in the Fn layer (read with 0x90): Fn + Page Down is action `0a 0c 00 00`, Fn + Left Ctrl is `13 00 00 00`. They run only from a key press. The base class's OLEDOPTION (0x22) is not an on/off switch either: the app uses it to send disk, memory, CPU and network figures to models with a color screen.
- **Use the OLED commands.** 0x2B/0xAB are for other models' screens. On the Striker, SET 0x2B with mode 0 (off) or 1 (on) changed nothing, and GET 0xAB returned `ab 06 …` or `ab 00 …` regardless of the animation shown.


## Other settings

These layouts are taken from the official app's code for the Striker. The GET replies listed as verified in the command table were read on hardware. SETs verified on hardware (written, read back, restored): polling rate, debounce, sleep timers, and the Windows-key lock bit (also confirmed by the key itself). The other SETs in this section are untested. Unless stated otherwise, every packet is 64 bytes with a BIT7 checksum at byte 7, and the payload starts at byte 1 or byte 8 as shown.

### Profile

The Striker has 3 profiles (0, 1, 2). Set: `05 [profile]`; get: `85` → reply byte 1 = profile. Many other commands take the profile in byte 1.

### Polling rate

Set: `04 [profile] [rate]`, rate 1 = 1000 Hz, 2 = 500, 4 = 250, 8 = 125. Get: `84 [profile]` → reply byte 2. The test keyboard reported 1 (1000 Hz).

### Keyboard options

Set: `06 [profile] [flags] [fnMatrix] [powerSave]`; get: `86 [profile]` → the same three bytes at 2, 3 and 4.

| Flags bit | Meaning when 1 |
| --- | --- |
| 0 (0x01) | Windows key locked (verified: the key stops working) |
| 1 (0x02) | Mac mode (verified: the keyboard's Fn layer, read with 0x90, switches Fn+F3 from Win+Tab to Ctrl+Up, Fn+F4 from Win+D to Cmd+M, Fn+F12 from Win+Space to Ctrl+Space, and clears Fn+Win; the normal key map is unchanged) |
| 3 (0x08) | WASD and arrow keys swapped |
| 4 (0x10) | Backlight off |
| 5 (0x20) | Second zone off |
| 6 (0x40) | "keyboardMode" (meaning unknown) |
| 7 (0x80) | Keyboard locked |

Byte 3 bit 0 = "Fn key matrix" (meaning unknown); byte 4 nonzero = power-save mode. The app reads Mac mode from bit 1 but writes `Number("mac")` (NaN, so 0) into bit 2, so it can never turn Mac mode on, and any option write from it turns Mac mode off. Bit 2 is stored if written, but it is not a mode: with it set, the Fn layer reads back as all 0xFF. Read-modify-write the byte.

### Debounce

Set: `11 00 [ms]` (value in byte 2); get: `91` → reply byte 2. The app allows up to 10.

### Sleep timers

Four 16-bit little-endian values in seconds: light sleep and deep sleep, each for Bluetooth and 2.4 GHz. 0 disables a timer. The app limits them to 1 to 60 minutes (light) and 10 to 60 minutes (deep).

| Field | GET reply bytes | SET bytes |
| --- | --- | --- |
| Light sleep, Bluetooth | 1..2 | 8..9 |
| Light sleep, 2.4 GHz | 3..4 | 10..11 |
| Deep sleep, Bluetooth | 5..6 | 12..13 |
| Deep sleep, 2.4 GHz | 7..8 | 14..15 |

The SET fields start at byte 8 because byte 7 holds the checksum. The test keyboard read `92 78 00 78 00 90 06 90 06` = 120 s light and 1680 s deep for both radios.

### Key remapping (one key at a time)

Set: `13 [profile] [slot] 00 00 00 00 [csum] [c0] [c1] [c2] [c3]`: the slot in byte 2, and the 4-byte action code in bytes 8..11.

**Slot** = the key's position in the Striker's default key matrix: 128 slots of 4 bytes, column by column, 6 rows per column. Each slot's 4 bytes are the key's default action code (normally `00 00 [HID usage] 00`). Used slots on the Striker:

```
0 Esc, 1 `, 2 Tab, 3 CapsLock, 4 LShift, 5 LCtrl, 6 F1, 7 1, 8 Q, 9 A, 10 ISO-\, 12 F2, 13 2, 14 W, 15 S, 16 Z,
17 LWin, 18 F3, 19 3, 20 E, 21 D, 22 X, 23 LAlt, 24 F4, 25 4, 26 R, 27 F, 28 C, 30 F5, 31 5, 32 T, 33 G, 34 V,
36 F6, 37 6, 38 Y, 39 H, 40 B, 41 Space, 42 F7, 43 7, 44 U, 45 J, 46 N, 47 KP+, 48 F8, 49 8, 50 I, 51 K, 52 M,
53 KPEnter, 54 F9, 55 9, 56 O, 57 L, 58 comma, 59 RAlt, 60 F10, 61 0, 62 P, 63 ;, 64 ., 65 Fn, 66 F11, 67 -,
68 [, 69 ', 70 /, 71 RCtrl, 72 F12, 73 =, 74 ], 76 RShift, 77 Left, 78 Delete, 79 Backspace, 80 \, 81 Enter,
82 Up, 83 Down, 84 PageUp, 85 NumLock, 86 KP7, 87 KP4, 88 KP1, 89 Right, 90 PageDown, 91 KP/, 92 KP8, 93 KP5,
94 KP2, 95 KP0, 96 KP-, 97 KP*, 98 KP9, 99 KP6, 100 KP3, 101 KP., 102 Volume down (consumer 0xEA),
103 Volume up (consumer 0xE9)
```

Source: array `zK` in `dist/index.js`. Fn's slot code is `00 0A 01 00`, and the volume entries are consumer-page codes.

**Action code** (4 bytes):

| Action | Bytes |
| --- | --- |
| Disable the key | `00 00 00 00` |
| Key, optionally with one modifier | `00 [mod] [HID usage] [second HID usage or 0]`; mod 0 none, 0xE0 Ctrl, 0xE1 Shift, 0xE2 Alt, 0xE3 Win |
| Key code above 255 | The 32-bit value, big-endian |
| Mouse or special function | From the app's tables `qz` (mouse) and `Lx` (functions) |
| Run a macro | `09 [mode] [macro index] 00`; mode 0 = repeat N times, 1 = toggle on/off, 2 = repeat while held |

Example: CapsLock (slot 3) acts as Left Ctrl in profile 0: `13 00 03 00 00 00 00 E9 00 00 E0 00`.

### Fn layer

Same as key remapping, but command 0x15 with the Fn layer index in byte 1: `15 [fnIndex] [slot] … [csum] [c0..c3]`. The Striker has one Fn layer. To restore a key's default Fn action, the app copies 4 bytes from the default Fn table (`PK` for this model).

### Reading the key map, Fn layer and macros

Verified read-only on 2026-10-20. Each read is a GET with BIT7 checksum, `[cmd] [index] [chunk] 00 00 00 00 [csum]`, repeated for each chunk. Unlike the other GETs, the reply carries no command byte: all 64 bytes are data, and the app joins the chunks.

| Read | Command | Index | Chunks | Result on this keyboard |
| --- | --- | --- | --- | --- |
| Key map | 0x89 | profile | 0..7 (128 slots × 4 bytes) | Identical to the app's default table `zK` |
| Fn layer | 0x90 | Fn layer 0 | 0..7 | Matches `PK` except slot 71 (Right Ctrl): keyboard `00 00 00 00`, `PK` `13 03 01 00` |
| Macro | 0x8B | macro number | 0..3 (256 bytes) | Macros 0..2 all zero (none set) |

The Right Ctrl difference fits what the keyboard does: Fn + Right Ctrl does nothing, while Fn + Left Ctrl (slot 5, `13 00 00 00` in both) cycles the screen. So the app's default Fn table is not exactly this firmware's, and "restore default" on Right Ctrl would write an action the keyboard didn't ship with.

Tested 2026-10-20: with Fn + Right Ctrl set to `13 03 01 00` (written with 0x15, which read back correctly, so Fn-layer writes work), pressing it had no visible effect on the screen or backlight. Each press did send vendor event `05 21 40 00` or `05 21 42 00`, alternating, so the firmware recognizes the action and flips some internal state, but nothing readable changed (KBOPTION, profile, polling rate, debounce, sleep, both lighting zones all unchanged). Probably a function for other models in the same firmware family. Restored to `00 00 00 00`.

Macro write verified on 2026-10-20: a one-chunk macro (`16 00 00 38 01 00 00 [csum]` + body `01 00 04 8A 04 00`, i.e. repeat once, A down with a 10 ms delay, A up) read back byte for byte with 0x8B, and writing a zero chunk cleared it. The change takes 0.1 to 0.3 s to appear: a read 50 ms after the write still returns the old contents, so wait at least 300 ms before reading back. Note from the app code: it only sends chunks that contain a nonzero byte, so deleting a macro in the app sends nothing and probably leaves it on the keyboard; to clear one, write a zero chunk 0 with the last-chunk flag set.

Running a macro from a key, verified the same day: macro 0 = `01 00 0B 8A 0B 0A 0C 8A 0C 00` (h down/up, i down/up, 10 ms apart), key remap `13 00 5B 00 00 00 00 [csum] 09 00 00 00` (keypad /, slot 91 → run macro 0, mode 0). Pressing keypad / typed "hi". The macro's keys arrive as NKRO bitmap reports (report ID 1, a different hidraw node from the 8-byte boot reports that ordinary typing produced). Writing the key's default code `00 00 54 00` back restored it. This also verifies single-key remapping (0x13).

### Macros

A macro is a 256-byte body sent in 56-byte chunks with command 0x16. Only chunks containing nonzero bytes are sent (at most 5).

Chunk header: `16 [macro index] [chunk n] 38 [1 if last chunk else 0] 00 00 [csum]`, then 56 body bytes (0x38 = 56).

Body, as the app builds it (method `setMacro`):

1. Bytes 0..1: repeat count, 16-bit little-endian.
2. Then the events in order:
    - Key: `[HID usage] [attr]`
    - Mouse button: `[button code from table Tx] [attr]`
    - Mouse move: `F9 [attr] [dx as int8] [dy as int8]`
    - Delay: a 16-bit little-endian millisecond value (see the rules below)
3. `attr` = bit 7 set for press, clear for release (mouse move has no press bit). Its low 7 bits hold the delay that follows the event, when the next event is a delay of 127 ms or less; that delay event is then consumed.
4. When an event's attr carries no delay, the next item must be a delay word: either the delay itself (16-bit LE) or `00 00` if the next event is not a delay.
5. A delay that follows an event whose attr already carried a delay is written as `00 00` followed by the 16-bit value.

This is the least certain part of the document; check it against `setMacro` in `dist/index.js` and read a macro back (0x96) before relying on it.

### Per-key custom colors

The keyboard stores three pictures, each one RGB color per key slot (128 slots, 384 bytes; the slots are the key map's, see `zK`). Backlight mode 13 shows one, chosen by the option nibble. Verified on hardware: reading and uploading act on **the picture being shown**, whatever the packet says, so select it with mode 13 first, as the official app does.

- **Read:** GET `8C 00 [chunk]` for chunks 0..5. Each reply is 64 bytes of picture data (no command byte); slot n is at bytes 3n..3n+2. Chunks above 5 return repeats of other data, not the other pictures.
- **Upload:** 7 packets `0C 00 80 01 [chunk 0..6] 00 00 [BIT7 csum]` followed by 56 data bytes (chunk 6 is mostly padding). The app waits 20 ms between packets and pauses afterwards; 500 ms was enough for an immediate readback to match.
- **One key:** `14 [picture 0..2] [slot] 00 00 00 00 [csum] [R] [G] [B]` changes one key of any picture, shown or not. A single write sticks, but writes sent in quick succession are mostly dropped (half were lost even 200 ms apart), so the app never uses it; neither does ours.

As shipped, picture 1 lights WASD and the arrow keys red, and pictures 2 and 3 have red, green and blue key groups. (On the test keyboard pictures 2 and 3 were later reduced to their red keys after test writes.)

## Vendor event channel (interface 1)

When a setting changes on the keyboard itself (an Fn-key shortcut), it may send a 4-byte input report with ID 5 on interface 1: `05 [a0] [a1] [a2]`. The official app uses these events to know when to re-read the keyboard's settings. Interface 1's report descriptor declares report 5 as 3 vendor bytes (`85 05 09 01 15 00 26 FF 00 75 08 95 03 81 02`). Read it as ordinary input reports (`read()` on the hidraw node, or `hid_read`); no request is needed.

How the app decodes `[a0, a1, a2]`:

| Event | Meaning |
| --- | --- |
| `0F 01 00` / `0F 00 00` | Start / end of a burst of changes; the app waits for the end before acting |
| a0 = 0x01 | Profile switched |
| a0 = 0x04..0x07 | Backlight changed; earlier notes report 04 = effect (a1 = mode), 05 = brightness, 06 = speed, 07 = color |
| a0 = 0x08..0x0B | Second zone changed |
| a0 = 0x03 | Option changed: a2 = 0 or 2 → Windows/Mac (a1 bit 1 = Mac); a2 = 8 → Fn layer switched (low hex digit of a1); a2 = 9 → power-save mode |
| `0D 00 00` | Factory reset done |
| `13 00 00` | Sleep mode toggled |
| `2C 00 00` | Screen clear finished |
| `80 00 [1..3]` | Shortcut to open app 1..3 (used by other models) |
| `21 40 00` / `21 42 00` | Not in the app's decoder. Sent by Fn action `13 03 01 00` on the Striker, alternating on each press, with no visible effect |

The app debounces these by 500 ms and then re-reads the affected setting with a GET. Status: an earlier session recorded backlight events (a0 = 04, a1 = mode) on this keyboard. In a later test, pressing Fn + Right Ctrl produced no event, but that key combination does not cycle the screen (Fn + Left Ctrl does), so whether cycling sends an event is still unknown.

## Official app and the Linux driver

The official app is a UI that builds packets and hands them to a local helper over gRPC-web. Any program that speaks the same gRPC API can stand in for the Windows helper, and that is how the app runs on Linux.

```
+-------------------+     +-------------------+     +-------------------+     +---------------------+
| NYXI app          | --> | gRPC-web          | --> | iot_driver        | --> | Striker keyboard       |
| Electron UI       |     | 127.0.0.1:3814    |     | adds the checksum |     | interface 2: config |
| builds packets    |     | sendMsg, readMsg  |     | HID feature I/O   |     | interface 1: events |
+-------------------+     +-------------------+     +-------------------+     +---------------------+
          ^                                                                              |
          +--------------- watchVender stream: keyboard events back to the app ---------+
```

The app passes each packet without its checksum, plus the checksum type. The driver fills in the checksum, sends the feature report and reads the reply.

### gRPC API the app needs (service `driver.DriverGrpc`, schema in `driver.proto`)

| Method | Request → response | Purpose |
| --- | --- | --- |
| watchDevList | Empty → stream DeviceList | Devices present; the app matches `id` (1473 = Striker wired, −2147485121 = dongle) and uses `path` as the device address |
| sendMsg | SendMsg{devicepath, msg, checksumtype, dangledevtype} → ResSend{err} | Apply the checksum, send one feature report; err "" = success |
| readMsg | ReadMsg{devicepath} → ResRead{err, msg} | Read one feature report (64 bytes, no report ID) |
| sendRawFeature, readRawFeature | Same messages | Raw variants, used for the 2.4 GHz dongle |
| watchVender | Empty → stream VenderMsg{msg} | Forward interface-1 event reports; msg = `05 a0 a1 a2` |
| insertDb, getItemFromDb, getAllKeysFromDb, getAllValuesFromDb, deleteItemFromDb | Key/value records | The app's local storage for profiles, macros and pictures |
| watchSystemInfo | Empty → stream SystemInfo | CPU and memory stats for screens that show them |
| setLightType, upgradeOTAGATT, muteMicrophone and others | Various | Other models and firmware updates |

### Running it on Linux (working as of 2026-10-17)

- Driver: `nyxi-driver/` (Rust: tonic, tonic-web, hidapi), built with `cargo build --release --bin iot_driver`. It listens on 127.0.0.1:3814.
- Runtime: the app's bundled Electron was a macOS build; the Linux build of Electron 22.3.27 lives in `electron-linux/` and runs with `--no-sandbox`.
- On Linux the app does not start any driver (its launcher only handles Windows and macOS), so `nyxi-app.sh` starts the driver, runs the app, and stops the driver on exit.
- Driver gaps: watchVender is a stub (keyboard-side changes don't refresh the app), and the firmware-update method only pretends to succeed.

## Hazards, unknowns and source map

Never scan command numbers blindly. In this family, "high" values are not all harmless GETs (0xAC erases flash), and several commands reset the keyboard or put it into firmware-update mode. Use only commands documented above, and read a setting back before writing it.

### Do not send while exploring

| Command | Name in app | Effect |
| --- | --- | --- |
| 0x02 | SET_RESERT | Factory reset (all profiles, remaps, macros) |
| 0x7F | SET_BOOTLOATER | Reboots into the firmware bootloader; the keyboard stops working as a keyboard |
| 0xAC | SET_FLASHCHIPERASSE | Erases the screen's flash chip (the app runs it only for color-screen models) |
| 0x30, 0x31, 0xB0, 0xB1 | OLED bootloader / start / checksum | Screen firmware update |
| 0x40, 0x41, 0xC0, 0xC1 | MLED bootloader / start / checksum | LED-controller firmware update |
| 0x7A to 0x7E | G_CMD test mode, status, ready, length | Factory test and update handshake |
| 0x09 | SET_KEYMATRIX (full) | Rewrites the whole key map in 9 chunks; bad data remaps every key. Use 0x13 for single keys |

The app's "firmware update" downloads files from NYXI's servers and drives these commands. The Linux driver does not implement updates; do not try one through it.

### Open questions

- [x] Does uploading to the screen switch it to the custom slot? Yes, immediately.
- [x] Do multi-frame screen animations play, and what unit is the frame delay? Yes; about 20 ms per unit.
- [ ] Is there any host command that chooses a built-in screen animation? None found. The screen can only be turned off with its Fn shortcut.
- [x] What does the second lighting zone (0x08) light up on the Striker? Nothing visible; see "Second lighting zone".
- [x] Do the GET replies work on macOS with the correct checksum? Yes: the app reads its settings on macOS (2026-10-24).
- [x] Can the key map, Fn layer and macros be read? Yes (0x89, 0x90, 0x8B); see "Reading the key map, Fn layer and macros".
- [x] Does a macro run from a key? Yes: keypad / set to `09 00 00 00` typed the macro ("hi"), then was restored.
- [x] Which bit does the keyboard use for Mac mode? Bit 1 (0x02); see Keyboard options.

### Where to find things

The app's JavaScript is one minified file, `extracted_asar/dist/index.js` (5.5 MB). Pretty-print it first (`npx js-beautify@1.15.1 index.js > pretty.js`), then search for these strings:

| Search for | What it is |
| --- | --- |
| `yc500_nyxift98_soc` | Striker device entries (VID/PID, ids, screen size) and its class `CK` |
| `CK=function` | Striker class; its parent chain is CK → Qz → hw → Jx → Zx → Hx → Kx → Ix → Vx |
| `FEA_CMD_` | Command constants; later classes override earlier ones, so resolve them along the chain |
| `setLightSetting`, `getLightSetting` | Backlight packets (take the copy inside class hw) |
| `setTFTLCDData` | Screen upload (in class Vx) |
| `tJ=function` | 9x9 bitmap packer |
| `zK=[` | Striker default key matrix (slot table) |
| `setKeyConfigSimple`, `configToChangeArr` | Single-key remap and action codes (class Qz) |
| `setMacro`, `_setMacro` | Macro body and chunking |
| `watchVender` | Vendor event decoding |
| `CheckSumType` | Checksum enum: BIT7 0, BIT8 1, NONE 2 |

Other files in the project folder:

- `nyxi_led.py`, `nyxi_screen.py`: small, working reference implementations of the backlight and screen protocols.
- `nyxi-driver/`: the Linux gRPC driver; `src/device_manager.rs` has the checksum and transport code.
- `extracted_asar/iot_driver.exe`: NYXI's original Windows driver (32-bit Rust, stripped). It uses hidapi's `HidD_SetFeature` / `HidD_GetFeature`.
- `extracted_asar/iot_driver` (Linux ELF): not NYXI's. It is an older build of the replacement driver with the wrong checksum; ignore it.
