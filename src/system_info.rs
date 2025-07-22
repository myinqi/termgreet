use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::Command;
use sysinfo::System;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub data: HashMap<String, String>,
}

impl SystemInfo {
    pub fn gather() -> Self {
        let mut data = HashMap::new();
        let mut sys = System::new_all();
        sys.refresh_all();

        // OS Information
        data.insert("OS".to_string(), Self::get_os_info());
        data.insert("KERNEL".to_string(), Self::get_kernel_version());
        data.insert("UPTIME".to_string(), Self::format_uptime(System::uptime()));

        // Desktop Environment / Window Manager
        data.insert("DE".to_string(), Self::get_desktop_environment());
        data.insert("WM".to_string(), Self::get_window_manager());

        // Shell
        data.insert("SHELL".to_string(), Self::get_shell());
        data.insert("TERMINAL".to_string(), Self::get_terminal());

        // Hardware Information
        data.insert("CPU".to_string(), Self::get_cpu_info(&sys));
        data.insert("GPU".to_string(), Self::get_gpu_info());
        data.insert("MEMORY".to_string(), Self::get_memory_info(&sys));
        data.insert("DISK".to_string(), Self::get_disk_info(&sys));

        // Display Information
        data.insert("RESOLUTION".to_string(), Self::get_resolution());

        // Additional modules that might be missing
        data.insert("THEME".to_string(), Self::get_theme());
        data.insert("ICONS".to_string(), Self::get_icons());

        // Battery (if available)
        if let Some(battery) = Self::get_battery_info() {
            data.insert("BATTERY".to_string(), battery);
        }

        // Package count (if available)
        if let Some(packages) = Self::get_package_count() {
            data.insert("PACKAGES".to_string(), packages);
        }

        // Locale
        data.insert("LOCALE".to_string(), Self::get_locale());

        Self { data }
    }

    fn get_os_info() -> String {
        if let Ok(content) = fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    return line.split('=').nth(1)
                        .unwrap_or("Unknown")
                        .trim_matches('"')
                        .to_string();
                }
            }
        }
        
        // Fallback to uname
        Self::run_command("uname", &["-sr"])
            .unwrap_or_else(|| "Unknown OS".to_string())
    }

    fn get_kernel_version() -> String {
        Self::run_command("uname", &["-r"])
            .unwrap_or_else(|| "Unknown".to_string())
    }

    fn format_uptime(uptime_seconds: u64) -> String {
        let days = uptime_seconds / 86400;
        let hours = (uptime_seconds % 86400) / 3600;
        let minutes = (uptime_seconds % 3600) / 60;

        if days > 0 {
            format!("{} days, {} hours, {} mins", days, hours, minutes)
        } else if hours > 0 {
            format!("{} hours, {} mins", hours, minutes)
        } else {
            format!("{} mins", minutes)
        }
    }

    fn get_desktop_environment() -> String {
        // Check common DE environment variables
        let de_vars = [
            "XDG_CURRENT_DESKTOP",
            "DESKTOP_SESSION",
            "GNOME_DESKTOP_SESSION_ID",
            "KDE_FULL_SESSION",
        ];

        for var in &de_vars {
            if let Ok(value) = env::var(var) {
                if !value.is_empty() {
                    return value;
                }
            }
        }

        "Unknown".to_string()
    }

    fn get_window_manager() -> String {
        // Check XDG_SESSION_TYPE first
        let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_default();
        
        // For Wayland sessions
        if session_type == "wayland" {
            // Check for Wayland compositors
            let wayland_wms = [
                ("kwin_wayland", "KWin"),
                ("gnome-shell", "GNOME Shell"),
                ("weston", "Weston"),
                ("sway", "Sway"),
                ("river", "River"),
                ("hyprland", "Hyprland"),
                ("wayfire", "Wayfire"),
            ];
            
            for (process, name) in &wayland_wms {
                if Self::run_command("pgrep", &["-x", process]).is_some() {
                    return name.to_string();
                }
            }
            
            // Check WAYLAND_DISPLAY for compositor info
            if let Ok(display) = env::var("WAYLAND_DISPLAY") {
                if !display.is_empty() {
                    return "Wayland Compositor".to_string();
                }
            }
        }
        
        // For X11 sessions
        if session_type == "x11" || env::var("DISPLAY").is_ok() {
            // Try getting WM name from X11 properties
            if let Some(output) = Self::run_command("xprop", &["-root", "-notype", "_NET_WM_NAME"]) {
                if let Some(wm_name) = output.split('=').nth(1) {
                    let wm_name = wm_name.trim().trim_matches('"').trim();
                    if !wm_name.is_empty() && wm_name != "(null)" {
                        return wm_name.to_string();
                    }
                }
            }
            
            // Check for common X11 WMs by process
            let x11_wms = [
                ("kwin_x11", "KWin"),
                ("kwin", "KWin"),
                ("gnome-shell", "GNOME Shell"),
                ("xfwm4", "Xfwm4"),
                ("openbox", "Openbox"),
                ("i3", "i3"),
                ("bspwm", "bspwm"),
                ("dwm", "dwm"),
                ("awesome", "awesome"),
                ("xmonad", "xmonad"),
                ("fluxbox", "Fluxbox"),
                ("blackbox", "Blackbox"),
                ("icewm", "IceWM"),
                ("jwm", "JWM"),
                ("herbstluftwm", "herbstluftwm"),
            ];
            
            for (process, name) in &x11_wms {
                if Self::run_command("pgrep", &["-x", process]).is_some() {
                    return name.to_string();
                }
            }
        }
        
        // Check environment variables as fallback
        if let Ok(wm) = env::var("WINDOW_MANAGER") {
            if !wm.is_empty() {
                return wm;
            }
        }
        
        // Check desktop session
        if let Ok(session) = env::var("DESKTOP_SESSION") {
            match session.to_lowercase().as_str() {
                "plasma" | "plasmawayland" | "plasmax11" => return "KWin".to_string(),
                "gnome" | "gnome-wayland" | "gnome-xorg" => return "GNOME Shell".to_string(),
                "xfce" => return "Xfwm4".to_string(),
                "lxde" => return "Openbox".to_string(),
                "i3" => return "i3".to_string(),
                _ => {}
            }
        }
        
        "Unknown".to_string()
    }

    fn get_shell() -> String {
        env::var("SHELL")
            .map(|shell| {
                shell.split('/').last().unwrap_or("Unknown").to_string()
            })
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    fn get_terminal() -> String {
        // Check common terminal environment variables
        let term_vars = ["TERM_PROGRAM", "TERMINAL_EMULATOR", "TERM"];
        
        for var in &term_vars {
            if let Ok(value) = env::var(var) {
                if !value.is_empty() && value != "xterm-256color" {
                    return value;
                }
            }
        }

        // Try to detect from parent process
        if let Ok(ppid) = env::var("PPID") {
            if let Ok(output) = Command::new("ps")
                .args(&["-p", &ppid, "-o", "comm="])
                .output()
            {
                if let Ok(comm) = String::from_utf8(output.stdout) {
                    return comm.trim().to_string();
                }
            }
        }

        "Unknown".to_string()
    }

    fn get_cpu_info(sys: &System) -> String {
        if let Some(cpu) = sys.cpus().first() {
            let brand = cpu.brand().trim();
            let cores = sys.cpus().len();
            format!("{} ({} cores)", brand, cores)
        } else {
            "Unknown CPU".to_string()
        }
    }

    fn get_gpu_info() -> String {
        // Try lspci first
        if let Some(output) = Self::run_command("lspci", &[]) {
            for line in output.lines() {
                if line.contains("VGA compatible controller") || line.contains("3D controller") {
                    if let Some(gpu) = line.split(": ").nth(1) {
                        return gpu.to_string();
                    }
                }
            }
        }

        // Try nvidia-smi for NVIDIA cards
        if let Some(output) = Self::run_command("nvidia-smi", &["--query-gpu=name", "--format=csv,noheader"]) {
            if let Some(gpu) = output.lines().next() {
                return gpu.trim().to_string();
            }
        }

        "Unknown GPU".to_string()
    }

    fn get_memory_info(sys: &System) -> String {
        let total_mem = sys.total_memory() / 1024 / 1024; // Convert to MB
        let used_mem = sys.used_memory() / 1024 / 1024;
        let total_gb = total_mem as f64 / 1024.0;
        let used_gb = used_mem as f64 / 1024.0;
        
        format!("{:.1}GB / {:.1}GB ({:.0}%)", 
                used_gb, total_gb, (used_mem as f64 / total_mem as f64) * 100.0)
    }

    fn get_disk_info(_sys: &System) -> String {
        // Use df command as fallback since sysinfo disk API changed
        if let Some(output) = Self::run_command("df", &["-h", "/"]) {
            for line in output.lines().skip(1) { // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let total = parts[1];
                    let used = parts[2];
                    let usage = parts[4];
                    return format!("{} / {} ({})", used, total, usage);
                }
            }
        }
        "Unknown".to_string()
    }

    fn get_resolution() -> String {
        // Try xrandr for X11
        if let Some(output) = Self::run_command("xrandr", &["--current"]) {
            for line in output.lines() {
                if line.contains(" connected primary") || line.contains(" connected ") {
                    if let Some(res_part) = line.split_whitespace().find(|s| s.contains("x") && s.chars().next().unwrap_or('a').is_ascii_digit()) {
                        return res_part.split('+').next().unwrap_or("Unknown").to_string();
                    }
                }
            }
        }

        // Try wlr-randr for Wayland
        if let Some(output) = Self::run_command("wlr-randr", &[]) {
            for line in output.lines() {
                if line.contains("current") {
                    if let Some(res) = line.split_whitespace().find(|s| s.contains("x")) {
                        return res.to_string();
                    }
                }
            }
        }

        "Unknown".to_string()
    }

    fn get_battery_info() -> Option<String> {
        // Check /sys/class/power_supply for battery info
        if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("BAT") {
                        if let (Ok(capacity), Ok(status)) = (
                            fs::read_to_string(path.join("capacity")),
                            fs::read_to_string(path.join("status"))
                        ) {
                            let capacity = capacity.trim();
                            let status = status.trim();
                            return Some(format!("{}% ({})", capacity, status));
                        }
                    }
                }
            }
        }
        None
    }

    fn get_package_count() -> Option<String> {
        // Try different package managers
        let package_managers = [
            ("dpkg", vec!["--get-selections"]),
            ("rpm", vec!["-qa"]),
            ("pacman", vec!["-Q"]),
            ("emerge", vec!["--list-sets"]),
            ("xbps-query", vec!["-l"]),
        ];

        for (cmd, args) in &package_managers {
            if let Some(output) = Self::run_command(cmd, args) {
                let count = output.lines().count();
                return Some(format!("{} ({})", count, cmd));
            }
        }

        None
    }

    fn get_locale() -> String {
        env::var("LANG")
            .or_else(|_| env::var("LC_ALL"))
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    fn get_theme() -> String {
        // Try to get GTK theme
        if let Some(gtk_theme) = Self::run_command("gsettings", &["get", "org.gnome.desktop.interface", "gtk-theme"]) {
            let theme = gtk_theme.trim().trim_matches('\'').trim_matches('"');
            if !theme.is_empty() && theme != "Unknown" {
                return theme.to_string();
            }
        }
        
        // Try to get KDE theme
        if let Ok(kde_config) = fs::read_to_string(format!(
            "{}/.config/kdeglobals", 
            env::var("HOME").unwrap_or_default()
        )) {
            for line in kde_config.lines() {
                if line.starts_with("ColorScheme=") {
                    if let Some(theme) = line.split('=').nth(1) {
                        return theme.to_string();
                    }
                }
            }
        }
        
        "Unknown".to_string()
    }

    fn get_icons() -> String {
        // Try to get GTK icon theme
        if let Some(icon_theme) = Self::run_command("gsettings", &["get", "org.gnome.desktop.interface", "icon-theme"]) {
            let theme = icon_theme.trim().trim_matches('\'').trim_matches('"');
            if !theme.is_empty() && theme != "Unknown" {
                return theme.to_string();
            }
        }
        
        // Try to get KDE icon theme
        if let Ok(kde_config) = fs::read_to_string(format!(
            "{}/.config/kdeglobals", 
            env::var("HOME").unwrap_or_default()
        )) {
            for line in kde_config.lines() {
                if line.starts_with("Theme=") {
                    if let Some(theme) = line.split('=').nth(1) {
                        return theme.to_string();
                    }
                }
            }
        }
        
        "Unknown".to_string()
    }

    fn run_command(cmd: &str, args: &[&str]) -> Option<String> {
        Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
    }
}
