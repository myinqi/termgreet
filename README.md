# TermGreet

Ein konfigurierbares System-Info-Tool in Rust, ähnlich wie neofetch/fastfetch, mit Unterstützung für PNG-Bilder und MOTD (Message of the Day).

## Features

- 🖥️ **Systeminfo-Anzeige**: CPU, GPU, Speicher, Betriebssystem, Window Manager, etc.
- 🎨 **ASCII-Art & PNG-Bilder**: Unterstützt sowohl ASCII-Art als auch PNG-Bilder (für Ghostty/Kitty)
- ⚙️ **TOML-Konfiguration**: Vollständig konfigurierbar über `~/.config/termgreet/config.toml`
- 📝 **MOTD-Support**: Zeigt konfigurierbare Nachrichten an
- 🎯 **Fastfetch-ähnliches Layout**: Zweispaltiges Layout mit anpassbaren Farben
- 🔧 **Modular**: Einzelne Module können aktiviert/deaktiviert werden

## Installation

```bash
# Repository klonen
git clone https://github.com/myinqi/termgreet.git
cd termgreet

# Kompilieren und installieren
cargo build --release
cargo install --path .
```

## Verwendung

```bash
# Grundlegende Verwendung
termgreet

# Mit eigener Konfigurationsdatei
termgreet --config /pfad/zur/config.toml

# Nur MOTD anzeigen
termgreet --motd

# Bilder deaktivieren
termgreet --no-image

# Hilfe anzeigen
termgreet --help
```

## Konfiguration

TermGreet erstellt automatisch eine Standard-Konfigurationsdatei unter `~/.config/termgreet/config.toml` beim ersten Start.

### Hauptkonfiguration (`~/.config/termgreet/config.toml`)

```toml
[general]
title = "System Information"
separator = " -> "

[general.colors]
title = "bright_cyan"
info = "bright_white"
separator = "bright_blue"

[display]
show_image = true
image_path = "/pfad/zu/ihrem/bild.png"  # Optional: PNG-Bild anstatt ASCII-Art
padding = 2

[display.image_size]
width = 40
height = 20

[modules]
os = true
kernel = true
uptime = true
packages = false
shell = true
resolution = true
de = true
wm = true
theme = false
icons = false
terminal = true
cpu = true
gpu = true
memory = true
disk = true
battery = true
locale = false

# Pfad zur MOTD-Konfigurationsdatei
motd_file = "~/.config/termgreet/motd.toml"
```

### MOTD-Konfiguration (`~/.config/termgreet/motd.toml`)

```toml
enabled = true
random = true
color = "bright_green"

messages = [
    "Willkommen auf Ihrem System!",
    "Haben Sie einen großartigen Tag!",
    "Bereit zum Programmieren!",
    "Möge der Code mit Ihnen sein! 🚀"
]
```

## Verfügbare Module

- **os**: Betriebssystem-Information
- **kernel**: Kernel-Version
- **uptime**: System-Laufzeit
- **packages**: Anzahl installierter Pakete
- **shell**: Verwendete Shell
- **resolution**: Bildschirmauflösung
- **de**: Desktop-Umgebung
- **wm**: Window Manager
- **theme**: System-Theme (falls verfügbar)
- **icons**: Icon-Theme (falls verfügbar)
- **terminal**: Terminal-Emulator
- **cpu**: CPU-Information
- **gpu**: GPU-Information
- **memory**: Speicherverbrauch
- **disk**: Festplattenbelegung
- **battery**: Akkustatus (falls verfügbar)
- **locale**: System-Locale

## Verfügbare Farben

- `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`
- `bright_black`, `bright_red`, `bright_green`, `bright_yellow`, `bright_blue`, `bright_magenta`, `bright_cyan`, `bright_white`

## PNG-Bilder

TermGreet unterstützt PNG-Bilder in Terminals, die Bildanzeige unterstützen (wie Ghostty und Kitty):

1. Setzen Sie `show_image = true` in der Konfiguration
2. Geben Sie den Pfad zu Ihrem PNG-Bild in `image_path` an
3. Passen Sie `image_size` nach Bedarf an

Wenn die Bildanzeige fehlschlägt, fällt TermGreet automatisch auf ASCII-Art zurück.

## Window Manager Erkennung

TermGreet erkennt zuverlässig verschiedene Window Manager:

**Wayland-Compositors:**
- KWin (Wayland)
- GNOME Shell
- Sway
- Hyprland
- River
- Wayfire
- Weston

**X11 Window Manager:**
- KWin (X11)
- GNOME Shell
- Xfwm4
- Openbox
- i3, bspwm, dwm
- awesome, xmonad
- Fluxbox, Blackbox
- IceWM, JWM
- herbstluftwm

## Beispiel-Ausgabe

```
System Information

╭─────────────────────╮  OS -> CachyOS Linux
│                     │  Kernel -> 6.15.7-2-cachyos
│     TermGreet       │  DE -> KDE
│                     │  WM -> KWin
╰─────────────────────╯  CPU -> AMD Ryzen 9 7950X (32 cores)
                         GPU -> NVIDIA GeForce RTX 4070 SUPER
                         Memory -> 10.31 GiB / 62.42 GiB (17%)
                         Disk -> 256G / 1.0T (25%)
```

## Entwicklung

```bash
# Entwicklungsversion ausführen
cargo run

# Tests ausführen
cargo test

# Code formatieren
cargo fmt

# Linting
cargo clippy
```

## Beiträge

Beiträge sind willkommen! Bitte:

1. Forken Sie das Repository
2. Erstellen Sie einen Feature-Branch
3. Committen Sie Ihre Änderungen
4. Pushen Sie zum Branch
5. Öffnen Sie einen Pull Request

## Lizenz

[Lizenz hier einfügen]

## Vergleich mit anderen Tools

| Feature | TermGreet | Fastfetch | Neofetch |
|---------|-----------|-----------|----------|
| PNG-Bilder | ✅ | ✅ | ❌ |
| TOML-Konfiguration | ✅ | ❌ | ❌ |
| MOTD-Support | ✅ | ❌ | ❌ |
| Geschwindigkeit | ⚡ | ⚡⚡ | 🐌 |
| Anpassbarkeit | ✅ | ✅ | ✅ |
| Rust | ✅ | ✅ | ❌ |
