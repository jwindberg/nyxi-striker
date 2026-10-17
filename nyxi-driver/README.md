# NYXI Driver (Cross-Platform Rust Reimplementation)

A high-performance, cross-platform background driver daemon for NYXI keyboards, mice, and gamepads. Replaces the Windows-only `iot_driver.exe` with a native Rust implementation that runs on **macOS**, **Linux**, and **Windows**.

## Features

- **Protocol Buffers / gRPC-Web**: Full compatibility with the official NYXI Electron UI over HTTP/2 and HTTP/1.1 on `127.0.0.1:3814`.
- **Cross-Platform HID**: Uses `hidapi` for native access on macOS (IOKit), Linux (`hidraw`), and Windows.
- **Embedded Database**: Uses `sled` for lightning-fast local storage of user profiles, macros, lighting setups, and keymaps in `userData/iot_db`.
- **System Telemetry**: Real-time CPU, RAM, disk, and network stats streaming (`watchSystemInfo`) for keyboard OLED displays and hardware monitor modes.
- **Hardware Profile Auto-Matching**: Matches over 40 device IDs from `supported_devices.toml`.

---

## Quick Start

### 1. Build the Driver

Ensure Rust is installed (`rustc`, `cargo`, and `protoc`):

```bash
cd nyxi-driver
cargo build --release
```

Binaries are placed in `target/release/`:
- `nyxi-driver` (CLI/daemon binary)
- `iot_driver` (drop-in binary for the Electron app bundle)

### 2. Run the Driver

Run the daemon in your terminal (or as a system service):

```bash
./nyxi-driver/target/release/nyxi-driver
```

You should see logs indicating active device detection and the server listening:
```text
INFO nyxi_driver: Starting NYXI cross-platform driver (iot_driver)...
INFO nyxi_driver::config: Loaded device config from "supported_devices.toml"
INFO nyxi_driver: NYXI driver listening on http://0.0.0.0:3814 (gRPC & gRPC-Web)
INFO nyxi_driver::device_manager: Detected device list change (3 devices active)
```

### 3. Run the Electron App

In a separate terminal, launch the extracted app using native Electron:

```bash
cd extracted_asar
npx -y electron .
```

The application will launch natively, connect to `http://127.0.0.1:3814`, and display your hardware.

---

## Repository Structure

- [`src/config.rs`](src/config.rs): Device VID/PID/Usage matching against `supported_devices.toml`.
- [`src/db.rs`](src/db.rs): Persistent database implementation via `sled`.
- [`src/device_manager.rs`](src/device_manager.rs): Cross-platform USB HID discovery, 64-byte feature reports, and checksum calculation.
- [`src/grpc_service.rs`](src/grpc_service.rs): Complete implementation of the 19 gRPC methods defined in `driver.proto`.
- [`src/system.rs`](src/system.rs): CPU, memory, and disk telemetry.
- [`proto/driver.proto`](proto/driver.proto): The reconstructed Protocol Buffers service schema.
