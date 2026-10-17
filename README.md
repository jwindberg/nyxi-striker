# NYXI CA01 Striker: open-source control

Tools for controlling the NYXI CA01 "Striker" keyboard (backlight, 9x9 screen, settings) without NYXI's Windows-only app.

- `PROTOCOL.md`: the complete USB HID protocol. Start here.
- `nyxi_led.py`: read and set the backlight (`python3 nyxi_led.py get`, `python3 nyxi_led.py modes`).
- `nyxi_screen.py`: draw on the 9x9 screen (`python3 nyxi_screen.py heart`).
- `nyxi-driver/`: Rust replacement for NYXI's `iot_driver`, which the official Electron app talks to over gRPC-web on 127.0.0.1:3814.
- `nyxi-app.sh`: runs the official app on Linux against `nyxi-driver`. It needs a local `extracted_asar/` and `electron-linux/`, which are not in this repo because the app is NYXI's code.

Goal: our own cross-platform app (macOS, Windows, Linux) that anyone can build from this repo.

## Linux setup

Let normal users access the keyboard:

```
echo 'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3151", MODE="0666"' | sudo tee /etc/udev/rules.d/99-nyxi.rules
sudo udevadm control --reload && sudo udevadm trigger
```

The Python tools currently use Linux hidraw ioctls; macOS support comes with the app.
