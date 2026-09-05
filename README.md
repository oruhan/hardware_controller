# Hardware Controller

A Linux desktop application written in Rust for monitoring and configuring supported peripherals. It currently supports the Razer DeathAdder V3 HyperSpeed over wired USB (`1532:00c4`) and its 2.4 GHz receiver (`1532:00c5`).

## Features

- Battery percentage and charging status
- USB and 2.4 GHz connection status
- Polling-rate monitoring and configuration at 125, 500, and 1000 Hz
- Automatic reconnection after device disconnects
- XDG-compliant user configuration
- A desktop interface and the `razerctl` diagnostics CLI

## Requirements

- Linux
- Rust 1.92 or newer
- Wayland or X11 runtime libraries for your desktop environment
- A C compiler and `pkg-config` for building

Typical Ubuntu/Debian build dependencies:

```bash
sudo apt install build-essential pkg-config libxkbcommon-dev libwayland-dev libx11-dev libxi-dev libxrandr-dev libxcursor-dev libgl1-mesa-dev
```

## Build and run

```bash
cargo build --release --locked
sudo make install
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw
```

Reconnect the mouse once, then launch the application from the desktop menu or a terminal:

```bash
hardware-controller
razerctl status
```

To use `cargo run --bin hardware-controller` before installing the application, install the udev rule separately:

```bash
sudo install -Dm644 packaging/udev/70-hardware-controller-razer.rules /usr/lib/udev/rules.d/70-hardware-controller-razer.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw
```

Do not run the application with `sudo`. The rule grants the active desktop session access only to the two supported HID devices.

## OpenRazer compatibility

Protocol commands and device timings match OpenRazer's DeathAdder V3 HyperSpeed implementation. OpenRazer is not required. If another OpenRazer client changes the same device concurrently, the last writer determines the active settings.

## Configuration

The device list is stored at `${XDG_CONFIG_HOME:-$HOME/.config}/hardware-controller/devices.toml`. Updates are written atomically.

## Development checks

```bash
make check
```

Hardware-independent tests run in CI. Real HID access requires a supported physical device and the installed udev rule.

## Uninstall

```bash
sudo make uninstall
sudo udevadm control --reload-rules
```

Licensed under Apache-2.0. See `LICENSE` for details.
