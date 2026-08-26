# Extender: Wayland-Native Remote Extended Monitor

Extender is a high-performance, native Linux tool designed for **Omarchy (Hyprland)** and **GNOME (Wayland)** environments. It transforms a secondary machine (such as a laptop) into a zero-friction, ultra-low-latency remote extended monitor for your primary workstation.

---

## Architecture Overview

Extender connects two Wayland systems over a high-throughput, low-latency network pipe:

```
+-------------------------------------------------------------------------+
| WORKSTATION (Host)                                                      |
|   1. Virtual Output:                                                    |
|      - Omarchy / Hyprland: `hyprctl output create headless`            |
|      - GNOME / Mutter: `ScreenCast.RecordVirtual` D-Bus Session         |
|   2. PipeWire -> Captures Raw Video Frames from Virtual Node           |
|   3. GStreamer -> Low-latency HW/SW Encoding (VAAPI / NVENC / x264)     |
|   4. Extender Host Daemon -> RTP Streaming & UInput Event Injection    |
+------------------------------------+------------------------------------+
                                     |
                       RTP Video / UDP Input
                                     |
+------------------------------------+------------------------------------+
| LAPTOP (Client)                                                         |
|   1. Extender Client Daemon -> Session Handshake & Keepalive            |
|   2. GStreamer -> HW/SW Decoder (VAAPI / NVDEC / avdec_h264)            |
|   3. Wayland Fullscreen Sink -> Zero-latency frame presentation        |
|   4. Event Capture -> Mouse/Keyboard forwarded back to host             |
+-------------------------------------------------------------------------+
```

---

## Workspace Structure

- [`crates/extender-common`](crates/extender-common): Shared binary protocol headers, packet serializers, handshake contracts, and HID input event definitions.
- [`crates/extender-server`](crates/extender-server): Host service managing multi-compositor virtual monitors (**Omarchy/Hyprland** & **GNOME/Mutter**), PipeWire video capture, and `/dev/uinput` event simulation.
- [`crates/extender-client`](crates/extender-client): Client application handling handshake negotiation, GStreamer Wayland decoder pipelines, and input forwarding.
- [`crates/extender-cli`](crates/extender-cli): Unified binary CLI (`extender host` / `extender client`).

---

## Requirements & Prerequisites

### 1. Omarchy / Arch Linux

On Omarchy or Arch Linux workstations:
```bash
sudo pacman -S --needed \
    pipewire \
    gstreamer \
    gst-plugins-base \
    gst-plugins-good \
    gst-plugins-bad \
    gst-plugins-ugly \
    gst-plugin-va \
    gst-libav \
    hyprland
```

### 2. Ubuntu / Debian (GNOME)

On Ubuntu or Debian workstations:
```bash
sudo apt update
sudo apt install -y \
    pipewire \
    libpipewire-0.3-dev \
    libgstreamer1.0-dev \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-vaapi \
    gstreamer1.0-libav
```

---

## Usage

### 1. Build Extender
```bash
cargo build --release
```

### 2. Start Extender Host on Omarchy (Workstation)
When launched on Omarchy, Extender automatically detects Hyprland and creates a headless virtual monitor (`EXTENDER-1`):
```bash
# Auto-detects Omarchy/Hyprland with software H.264
./target/release/extender host --width 1920 --height 1080 --codec h264-software

# Explicitly specifying Omarchy / Hyprland compositor
./target/release/extender host --compositor hyprland --width 1920 --height 1080 --codec h264-software

# Intel/AMD Hardware acceleration (VAAPI)
./target/release/extender host --width 1920 --height 1080 --codec h264-vaapi

# Nvidia Hardware acceleration (NVENC)
./target/release/extender host --width 1920 --height 1080 --codec h264-nvenc
```

### 3. Connect from Extender Client (Laptop)
```bash
./target/release/extender client --server 192.168.1.50:8555 --codec h264-software
```

---

## Benchmarks & Latency Targets

- **Target Latency:** < 50ms round-trip (input-to-display) over 5 GHz Wi-Fi or Gigabit Ethernet.
- **Target Frame Rate:** 60 FPS @ 1080p (1920x1080) or 1440p (2560x1440).
- **Packet Format:** Compact binary serialization with CRC32 checksums and microsecond timestamping for jitter/RTT tracking.

---

## License
MIT OR Apache-2.0

