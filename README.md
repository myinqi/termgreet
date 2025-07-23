# TermGreet

A configurable system information tool written in Rust, similar to neofetch/fastfetch, with support for PNG images and MOTD (Message of the Day).

## ✨ Features

- 🖥️ **Comprehensive System Info**: CPU, GPU, Memory, OS, Window Manager, Shell, Terminal, and more
- 🎨 **Modern Image Support**: PNG images with Kitty Graphics Protocol for pixel-perfect rendering
- ⚙️ **TOML Configuration**: Fully configurable via `~/.config/termgreet/config.toml`
- 📝 **MOTD Support**: Display configurable welcome messages
- 🎯 **Fastfetch-style Layout**: Clean, aligned output with customizable colors
- 🔧 **Modular Design**: Enable/disable individual modules as needed
- 🚀 **Version Display**: Optional version information for Shell, Terminal, DE, and WM
- 🎮 **GPU Driver Detection**: Shows driver type (open source/proprietary) with version
- 📊 **Enhanced Resolution**: Display resolution with refresh rate (e.g., 3440x1440 @ 165Hz)
- 🔤 **Font Detection**: Accurate terminal font detection with size information
- 🌐 **Universal Terminal Support**: Works in Kitty, Ghostty, Alacritty, GNOME Terminal, VSCode, and more

## 📦 Installation

```bash
# Clone the repository
git clone https://github.com/myinqi/termgreet.git
cd termgreet

# Build and install
cargo build --release
cargo install --path .
```

## 🚀 Usage

```bash
# Basic usage
termgreet

# Use custom configuration file
termgreet --config /path/to/config.toml

# Show only MOTD
termgreet --motd

# Disable images
termgreet --no-image

# Show help
termgreet --help
```

## ⚙️ Configuration

TermGreet automatically creates a default configuration file at `~/.config/termgreet/config.toml` on first run.

### Main Configuration (`~/.config/termgreet/config.toml`)

```toml
[general]
title = "System Information"

# Configurable separator with spacing and alignment
[general.separator]
symbol = "<>"
space_before = 2
space_after = 6
align_separator = true  # Align all separators in a column

# Separate colors for different elements
[general.colors]
title = "bright_cyan"
module = "bright_blue"     # Module names (OS, CPU, etc.)
info = "bright_white"      # System information values
separator = "bright_blue"

[display]
show_image = true
image_path = "/path/to/your/image.png"
prefer_kitty_graphics = true  # Use Kitty Graphics Protocol when available
padding = 2

# Fine-tune image scaling for perfect aspect ratio
[display.image_size]
width = 35
height = 15
cell_width = 17   # Pixels per terminal character (width)
cell_height = 24  # Pixels per terminal character (height)

[modules]
show_versions = true  # Show version info for Shell, Terminal, DE, WM
show_motd = true     # Enable/disable MOTD display
os = true
kernel = true
uptime = true
os_age = true        # Days since OS installation
packages = true
shell = true
resolution = true    # Now includes refresh rate
de = true
wm = true
theme = false
icons = false
terminal = true
font = true          # Terminal font with size
user = true          # Current username
hostname = true      # Computer name
cpu = true
gpu = true
gpu_driver = true    # GPU driver with open source/proprietary label
memory = true
disk = true
battery = false
locale = false

# Path to MOTD configuration file
motd_file = "~/.config/termgreet/motd.toml"
```

### MOTD Configuration (`~/.config/termgreet/motd.toml`)

```toml
enabled = true
random = true
color = "bright_green"

messages = [
    "Welcome to your system!",
    "Have a great day!",
    "Ready to code!",
    "System is running smoothly!",
    "Time to be productive!"
]
```

## 📋 Available Modules

### System Information
- **os**: Operating system information
- **kernel**: Kernel version
- **uptime**: System uptime
- **os_age**: Days since OS installation
- **packages**: Number of installed packages
- **locale**: System locale

### Environment
- **shell**: Shell with version (e.g., `zsh 5.9`)
- **terminal**: Terminal emulator with version (e.g., `ghostty 1.0.0`)
- **font**: Terminal font with size (e.g., `JetBrainsMono Nerd Font (13pt)`)
- **user**: Current username
- **hostname**: Computer name
- **de**: Desktop environment with version
- **wm**: Window manager with version
- **theme**: System theme (if available)
- **icons**: Icon theme (if available)

### Hardware
- **cpu**: CPU information with core count
- **gpu**: GPU information (cleaned, Fastfetch-style)
- **gpu_driver**: GPU driver with type and version (e.g., `NVIDIA (proprietary) 575.64.05`)
- **memory**: Memory usage
- **disk**: Disk usage
- **battery**: Battery status (if available)
- **resolution**: Display resolution with refresh rate (e.g., `3440x1440 @ 165Hz`)

## 🎨 Available Colors

**Standard Colors:**
- `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`

**Bright Colors:**
- `bright_black`, `bright_red`, `bright_green`, `bright_yellow`, `bright_blue`, `bright_magenta`, `bright_cyan`, `bright_white`

## 🖼️ Image Support

TermGreet provides advanced image rendering capabilities:

### Kitty Graphics Protocol
- **Pixel-perfect rendering** in Kitty, Ghostty, iTerm2, WezTerm
- **Automatic terminal detection** with graceful fallback
- **Configurable aspect ratio** via `cell_width` and `cell_height`

### Universal Compatibility
- **Block graphics fallback** for unsupported terminals (VSCode, etc.)
- **Consistent scaling** across all terminal types
- **Info-only mode** if image loading fails

### Configuration Tips
1. Set `prefer_kitty_graphics = true` for modern terminals
2. Adjust `cell_width` and `cell_height` for perfect aspect ratio
3. Use `width` and `height` to control overall image size

## 🔍 Advanced Features

### GPU Driver Detection
Automatically detects and categorizes GPU drivers:
- **Proprietary**: `NVIDIA (proprietary) 575.64.05`
- **Open Source**: `AMDGPU (open source)`, `Intel i915 (open source)`, `Nouveau (open source)`

### Font Detection
Accurate font detection across terminals:
- **Kitty**: Reads from `kitty.conf`
- **Ghostty**: Reads from `ghostty/config`
- **Alacritty**: Supports both YAML and TOML configs
- **GNOME Terminal**: Uses gsettings and dconf
- **VSCode**: Reads from settings.json

### Resolution with Refresh Rate
Enhanced resolution detection:
- **X11**: Parses `xrandr` output for active modes
- **Wayland**: Supports `wlr-randr` and `swaymsg`
- **Format**: `3440x1440 @ 165Hz`

### Version Information
Optional version display for:
- **Shell**: `bash 5.2.21`, `zsh 5.9`, `fish 3.6.1`
- **Terminal**: `kitty 0.32.2`, `alacritty 0.13.2`
- **DE/WM**: Detected when available

## 🔧 Window Manager Detection

Reliable detection for various environments:

**Wayland Compositors:**
- KWin (Wayland), GNOME Shell, Sway, Hyprland
- River, Wayfire, Weston, Labwc

**X11 Window Managers:**
- KWin (X11), GNOME Shell, Xfwm4, Openbox
- i3, bspwm, dwm, awesome, xmonad
- Fluxbox, Blackbox, IceWM, JWM, herbstluftwm

## 📊 Example Output

```
System Information

[PNG Image]              OS          <>      CachyOS Linux
                         Kernel      <>      6.15.7-2-cachyos
                         Uptime      <>      2 hours, 34 minutes
                         Shell       <>      zsh 5.9
                         Terminal    <>      ghostty 1.0.0
                         Font        <>      JetBrainsMono Nerd Font (13pt)
                         User        <>      khrom
                         Hostname    <>      workstation
                         DE          <>      KDE Plasma 6.0.2
                         WM          <>      KWin
                         Resolution  <>      3440x1440 @ 165Hz
                         CPU         <>      AMD Ryzen 9 7950X (32 cores)
                         GPU         <>      NVIDIA GeForce RTX 4070 SUPER
                         GPU Driver  <>      NVIDIA (proprietary) 575.64.05
                         Memory      <>      10.31 GiB / 62.42 GiB (17%)
                         Disk        <>      256G / 1.0T (25%)

Welcome to your system!
```

## 🛠️ Development

```bash
# Run development version
cargo run

# Run tests
cargo test

# Format code
cargo fmt

# Linting
cargo clippy

# Build release
cargo build --release
```

## 🤝 Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 📈 Comparison with Other Tools

| Feature | TermGreet | Fastfetch | Neofetch |
|---------|-----------|-----------|----------|
| PNG Images | ✅ | ✅ | ❌ |
| Kitty Graphics Protocol | ✅ | ✅ | ❌ |
| TOML Configuration | ✅ | ❌ | ❌ |
| MOTD Support | ✅ | ❌ | ❌ |
| Version Display | ✅ | ✅ | ❌ |
| GPU Driver Detection | ✅ | ✅ | ❌ |
| Refresh Rate Display | ✅ | ✅ | ❌ |
| Font Detection | ✅ | ✅ | ❌ |
| Configurable Separators | ✅ | ❌ | ❌ |
| Speed | ⚡ | ⚡⚡ | 🐌 |
| Customizability | ✅ | ✅ | ✅ |
| Written in Rust | ✅ | ❌ | ❌ |
