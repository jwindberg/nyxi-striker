# NYXI CA01 Striker: open-source control

Tools for controlling the NYXI CA01 "Striker" keyboard (backlight, 9x9 screen, settings) without NYXI's Windows-only app.

- `PROTOCOL.md`: the complete USB HID protocol. Start here.
- `nyxi_led.py`: read and set the backlight (`python3 nyxi_led.py get`, `python3 nyxi_led.py modes`).
- `nyxi_screen.py`: draw on the 9x9 screen (`python3 nyxi_screen.py heart`).
- `nyxi-driver/`: Rust replacement for NYXI's `iot_driver`, which the official Electron app talks to over gRPC-web on 127.0.0.1:3814.
- `nyxi-app.sh`: runs the official app on macOS or Linux against `nyxi-driver`.
- `setup-official-app.sh`: one-time setup for `nyxi-app.sh`. NYXI's app is their code, so it is not in this repo; this gets it from the private repo `jwindberg/nyxi-official-app` (or from their Windows installer).

## Running the official NYXI app on macOS or Linux

Needs Rust (`rustup`), Node.js/npm and access to the private repo `jwindberg/nyxi-official-app`. On Linux the driver build also needs `sudo apt install libudev-dev`.

```
./setup-official-app.sh    # once per machine; rerun to update the app
./nyxi-app.sh              # builds the driver on first run, then starts it and the app
```

Setup clones the app into `extracted_asar/` and installs it inside the downloaded Electron so it runs as an installed app. Without access to that repo, pass NYXI's Windows installer instead (`./setup-official-app.sh path/to/NYXI_SETUP.exe`, needs 7-Zip: `brew install sevenzip` or `sudo apt install 7zip`); nothing from the `.exe` is executed. On macOS the app starts its own driver from `~/Library/Application Support/NYXI Driver/iot_driver_v160`, so `nyxi-app.sh` puts our driver there before each launch.

On macOS, closing the window leaves the app running; quit it with Cmd+Q to stop the driver too.

Goal: our own cross-platform app (macOS, Windows, Linux) that anyone can build from this repo.

## Linux setup

Let normal users access the keyboard:

```
echo 'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3151", MODE="0666"' | sudo tee /etc/udev/rules.d/99-nyxi.rules
sudo udevadm control --reload && sudo udevadm trigger
```

The Python tools currently use Linux hidraw ioctls; macOS support comes with the app.
