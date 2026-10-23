# nyxi-driver

A replacement for `iot_driver`, the background driver that NYXI's official Electron app talks to. NYXI ships it only for Windows (`iot_driver.exe`); this one is written in Rust and runs on macOS and Linux too, so the official app can run there. Our own app in `../app` does not use it.

It serves the official app's gRPC-web API on `127.0.0.1:3814` (schema in `proto/driver.proto`) and passes the app's 64-byte feature reports to the keyboard through `hidapi`.

## Build and run

Normally `../nyxi-app.sh` builds it and starts it together with the app; see the main README. To build it alone:

```
cargo build --release
./target/release/nyxi-driver
```

Both binaries in `target/release/` are the same program: `nyxi-driver`, and `iot_driver` under the name the official app looks for. On Linux the build needs `libudev-dev`, and the keyboard needs the udev rule from the main README.

## What it does and doesn't do

- Lists connected devices and sends and receives feature reports for the app: lighting, key remapping, macros, settings and the screen all work this way.
- Keeps the app's local database (`iot_db`) with `sled`.
- Streams computer stats (`watchSystemInfo`) for models with a stats display.
- `watchVender` is a placeholder, so the app doesn't update when a setting is changed with the keyboard's Fn keys; reopen the app to see it.
- Firmware updates are a stub. Don't try one through it.

## Files

- `src/device_manager.rs`: device discovery, feature reports and the checksum.
- `src/grpc_service.rs`: the gRPC methods.
- `src/config.rs` and `supported_devices.toml`: which devices (VID, PID, usage) the app is offered.
- `src/db.rs`: the app's database.
- `src/system.rs`: computer stats.
