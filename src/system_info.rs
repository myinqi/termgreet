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
        data.insert("OS_AGE".to_string(), Self::get_os_age());

        // Desktop Environment / Window Manager
        data.insert("DE".to_string(), Self::get_desktop_environment());
        data.insert("WM".to_string(), Self::get_window_manager());

        // Shell
        data.insert("SHELL".to_string(), Self::get_shell());
        data.insert("TERMINAL".to_string(), Self::get_terminal());
        data.insert("FONT".to_string(), Self::get_font_info());
        data.insert("USER".to_string(), Self::get_user_info());
        data.insert("HOSTNAME".to_string(), Self::get_hostname_info());

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
        // Try nvidia-smi first for NVIDIA cards (gives cleaner names)
        if let Some(output) = Self::run_command("nvidia-smi", &["--query-gpu=name", "--format=csv,noheader"]) {
            if let Some(gpu) = output.lines().next() {
                let clean_name = gpu.trim();
                if !clean_name.is_empty() {
                    return clean_name.to_string();
                }
            }
        }

        // Try lspci as fallback
        if let Some(output) = Self::run_command("lspci", &[]) {
            for line in output.lines() {
                if line.contains("VGA compatible controller") || line.contains("3D controller") {
                    if let Some(gpu) = line.split(": ").nth(1) {
                        return Self::parse_gpu_name(gpu);
                    }
                }
            }
        }

        "Unknown GPU".to_string()
    }

    fn parse_gpu_name(raw_name: &str) -> String {
        // Parse GPU name to extract cleaner format like Fastfetch
        // Examples:
        // "NVIDIA Corporation AD104 [GeForce RTX 4070 SUPER] (rev a1)" -> "NVIDIA GeForce RTX 4070 SUPER"
        // "Advanced Micro Devices, Inc. [AMD/ATI] Radeon RX 6800 XT" -> "AMD Radeon RX 6800 XT"
        
        let mut result = raw_name.to_string();
        
        // Handle NVIDIA cards
        if result.contains("NVIDIA") {
            // Extract content from brackets [GeForce ...]
            if let Some(start) = result.find('[') {
                if let Some(end) = result.find(']') {
                    let gpu_model = &result[start + 1..end];
                    return format!("NVIDIA {}", gpu_model);
                }
            }
            // If no brackets, try to clean up the name
            result = result.replace("NVIDIA Corporation", "NVIDIA");
        }
        
        // Handle AMD cards
        if result.contains("Advanced Micro Devices") || result.contains("AMD/ATI") {
            result = result.replace("Advanced Micro Devices, Inc. [AMD/ATI]", "AMD");
            result = result.replace("Advanced Micro Devices, Inc.", "AMD");
            result = result.replace("[AMD/ATI]", "AMD");
        }
        
        // Remove revision information (rev a1), (rev 01), etc.
        if let Some(rev_pos) = result.find(" (rev ") {
            result = result[..rev_pos].to_string();
        }
        
        // Remove extra whitespace and clean up
        result = result.trim().to_string();
        
        // Remove any remaining brackets or parentheses at the end
        while result.ends_with(')') || result.ends_with(']') {
            if let Some(pos) = result.rfind(['(', '[']) {
                result = result[..pos].trim().to_string();
            } else {
                break;
            }
        }
        
        result
    }

    fn get_font_info() -> String {
        // Try to get font information from various terminal-specific methods
        
        // Debug: Check what terminal we're running in
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_else(|_| "unknown".to_string());
        
        // Try different terminal detection methods
        
        // Method 1: Check for Kitty terminal
        if term_program == "kitty" || std::env::var("KITTY_WINDOW_ID").is_ok() {
            if let Ok(home) = std::env::var("HOME") {
                let kitty_config = format!("{}/.config/kitty/kitty.conf", home);
                if let Ok(content) = std::fs::read_to_string(&kitty_config) {
                    let mut font_family = None;
                    let mut font_size = None;
                    
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("font_family") && !trimmed.starts_with("#") {
                            let font = line.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
                            let font = font.trim();
                            if !font.is_empty() {
                                font_family = Some(font.to_string());
                            }
                        } else if trimmed.starts_with("font_size") && !trimmed.starts_with("#") {
                            if let Some(size) = line.split_whitespace().nth(1) {
                                font_size = Some(size.to_string());
                            }
                        }
                    }
                    
                    if let Some(family) = font_family {
                        if let Some(size) = font_size {
                            return format!("{} ({}pt)", family, size);
                        } else {
                            return family;
                        }
                    }
                }
            }
        }
        
        // Method 2: Check for Ghostty terminal
        if term_program == "ghostty" || std::env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
            if let Ok(home) = std::env::var("HOME") {
                let ghostty_config = format!("{}/.config/ghostty/config", home);
                if let Ok(content) = std::fs::read_to_string(&ghostty_config) {
                    let mut font_family = None;
                    let mut font_size = None;
                    
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("font-family") && trimmed.contains('=') {
                            if let Some(font) = line.split('=').nth(1) {
                                font_family = Some(font.trim().trim_matches('"').trim_matches('\'').to_string());
                            }
                        } else if trimmed.starts_with("font-size") && trimmed.contains('=') {
                            if let Some(size) = line.split('=').nth(1) {
                                font_size = Some(size.trim().to_string());
                            }
                        }
                    }
                    
                    if let Some(family) = font_family {
                        if let Some(size) = font_size {
                            return format!("{} ({}pt)", family, size);
                        } else {
                            return family;
                        }
                    }
                }
            }
        }
        
        // Method 3: Check for VSCode terminal
        if term_program == "vscode" {
            if let Ok(home) = std::env::var("HOME") {
                let vscode_settings = format!("{}/.config/Code/User/settings.json", home);
                if let Ok(content) = std::fs::read_to_string(&vscode_settings) {
                    // Look for terminal font settings
                    for line in content.lines() {
                        if line.contains("terminal.integrated.fontFamily") {
                            if let Some(start) = line.find('"') {
                                if let Some(end) = line.rfind('"') {
                                    if start < end {
                                        let font = &line[start+1..end];
                                        if !font.is_empty() {
                                            return Self::format_font_with_size(font);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Method 4: Check for GNOME Terminal
        if std::env::var("TERMINAL_EMULATOR").unwrap_or_default() == "gnome-terminal" || std::env::var("GNOME_TERMINAL_SCREEN").is_ok() || std::env::var("GNOME_TERMINAL_SERVICE").is_ok() {
            // Try to get the current profile UUID
            if let Some(profile_output) = Self::run_command("gsettings", &["get", "org.gnome.Terminal.ProfilesList", "default"]) {
                let profile_uuid = profile_output.trim().trim_matches('\'').trim_matches('"');
                if !profile_uuid.is_empty() && profile_uuid != "(null)" {
                    let profile_path = format!("org.gnome.Terminal.Legacy.Profile:/org/gnome/terminal/legacy/profiles:/:{}/", profile_uuid);
                    if let Some(font_output) = Self::run_command("gsettings", &["get", &profile_path, "font"]) {
                        let font = font_output.trim().trim_matches('\'').trim_matches('"');
                        if !font.is_empty() && font != "(null)" {
                            return font.to_string();
                        }
                    }
                }
            }
            
            // Fallback: try default profile
            if let Some(output) = Self::run_command("gsettings", &["get", "org.gnome.Terminal.Legacy.Profile:/org/gnome/terminal/legacy/profiles:/:b1dcc9dd-5262-4d8d-a863-c897e6d979b9/", "font"]) {
                let font = output.trim().trim_matches('\'').trim_matches('"');
                if !font.is_empty() && font != "(null)" {
                    return font.to_string();
                }
            }
        }
        
        // Check for GNOME Terminal (dconf/gsettings) - general fallback
        if let Some(output) = Self::run_command("dconf", &["read", "/org/gnome/terminal/legacy/profiles:/:b1dcc9dd-5262-4d8d-a863-c897e6d979b9/font"]) {
            let font = output.trim().trim_matches('\'').trim_matches('"');
            if !font.is_empty() && font != "(null)" {
                return font.to_string();
            }
        }
        
        // Try gsettings for GNOME Terminal profile
        if let Some(output) = Self::run_command("gsettings", &["get", "org.gnome.Terminal.Legacy.Profile:/org/gnome/terminal/legacy/profiles:/:b1dcc9dd-5262-4d8d-a863-c897e6d979b9/", "font"]) {
            let font = output.trim().trim_matches('\'').trim_matches('"');
            if !font.is_empty() && font != "(null)" {
                return font.to_string();
            }
        }
        
        // Try to get font from terminal-specific configs
        // Check for Alacritty config (YAML and TOML formats)
        if let Ok(home) = std::env::var("HOME") {
            // Try YAML format first
            let alacritty_config = format!("{}/.config/alacritty/alacritty.yml", home);
            if let Ok(content) = std::fs::read_to_string(&alacritty_config) {
                let mut font_family = None;
                let mut font_size = None;
                
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("family:") {
                        if let Some(font) = line.split(':').nth(1) {
                            font_family = Some(font.trim().trim_matches('"').trim_matches('\'').to_string());
                        }
                    } else if trimmed.starts_with("size:") {
                        if let Some(size) = line.split(':').nth(1) {
                            font_size = Some(size.trim().to_string());
                        }
                    }
                }
                
                if let Some(family) = font_family {
                    if let Some(size) = font_size {
                        return format!("{} {}pt", family, size);
                    } else {
                        return family;
                    }
                }
            }
            
            // Try TOML format
            let alacritty_toml = format!("{}/.config/alacritty/alacritty.toml", home);
            if let Ok(content) = std::fs::read_to_string(&alacritty_toml) {
                // Simple TOML parsing for font settings
                let mut font_family = None;
                let mut font_size = None;
                
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("family") && trimmed.contains('=') {
                        if let Some(font) = line.split('=').nth(1) {
                            font_family = Some(font.trim().trim_matches('"').trim_matches('\'').to_string());
                        }
                    } else if trimmed.starts_with("size") && trimmed.contains('=') {
                        if let Some(size) = line.split('=').nth(1) {
                            font_size = Some(size.trim().to_string());
                        }
                    }
                }
                
                if let Some(family) = font_family {
                    if let Some(size) = font_size {
                        return format!("{} {}pt", family, size);
                    } else {
                        return family;
                    }
                }
            }
        }
        
        // Try to get font from Kitty config
        if let Ok(home) = std::env::var("HOME") {
            let kitty_config = format!("{}/.config/kitty/kitty.conf", home);
            if let Ok(content) = std::fs::read_to_string(&kitty_config) {
                let mut font_family = None;
                let mut font_size = None;
                
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("font_family") {
                        let font = line.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
                        let font = font.trim();
                        if !font.is_empty() {
                            font_family = Some(font.to_string());
                        }
                    } else if trimmed.starts_with("font_size") {
                        if let Some(size) = line.split_whitespace().nth(1) {
                            font_size = Some(size.to_string());
                        }
                    }
                }
                
                if let Some(family) = font_family {
                    if let Some(size) = font_size {
                        return format!("{} {}pt", family, size);
                    } else {
                        return family;
                    }
                }
            }
        }
        
        // Try gsettings for system monospace font as fallback
        if let Some(output) = Self::run_command("gsettings", &["get", "org.gnome.desktop.interface", "monospace-font-name"]) {
            let font = output.trim().trim_matches('\'').trim_matches('"');
            if !font.is_empty() && font != "(null)" {
                return font.to_string();
            }
        }
        
        // Fallback: try to detect common monospace fonts
        if let Some(output) = Self::run_command("fc-match", &["monospace"]) {
            if let Some(font) = output.split(':').next() {
                let font = font.trim();
                if !font.is_empty() {
                    return Self::format_font_with_size(font);
                }
            }
        }
        
        // If we get here, let's provide debug info to help identify the terminal
        // Also check for other common terminal environment variables
        let mut env_vars = Vec::new();
        if std::env::var("KITTY_WINDOW_ID").is_ok() { env_vars.push("KITTY"); }
        if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok() { env_vars.push("GHOSTTY"); }
        if std::env::var("GNOME_TERMINAL_SCREEN").is_ok() { env_vars.push("GNOME_TERMINAL"); }
        if std::env::var("ALACRITTY_SOCKET").is_ok() { env_vars.push("ALACRITTY"); }
        if std::env::var("WEZTERM_EXECUTABLE").is_ok() { env_vars.push("WEZTERM"); }
        
        if !env_vars.is_empty() {
            format!("Unknown Font (detected: {})", env_vars.join(", "))
        } else {
            "Unknown Font (no terminal detected)".to_string()
        }
    }
    
    fn format_font_with_size(font: &str) -> String {
        // Try to extract size information and format nicely
        if font.contains(" ") {
            let parts: Vec<&str> = font.split_whitespace().collect();
            if let Some(last) = parts.last() {
                // Check if last part is a number (size)
                if last.parse::<f32>().is_ok() {
                    let family = parts[..parts.len()-1].join(" ");
                    return format!("{} ({}pt)", family, last);
                }
            }
        }
        font.to_string()
    }
    
    fn get_user_info() -> String {
        // Get current username
        if let Ok(user) = std::env::var("USER") {
            if !user.is_empty() {
                return user;
            }
        }
        
        // Fallback to whoami command
        if let Some(output) = Self::run_command("whoami", &[]) {
            let user = output.trim();
            if !user.is_empty() {
                return user.to_string();
            }
        }
        
        // Fallback: try getent passwd with current UID
        if let Some(output) = Self::run_command("id", &["-u"]) {
            let uid = output.trim();
            if let Some(passwd_output) = Self::run_command("getent", &["passwd", uid]) {
                if let Some(username) = passwd_output.split(':').next() {
                    if !username.is_empty() {
                        return username.to_string();
                    }
                }
            }
        }
        
        "Unknown User".to_string()
    }
    
    fn get_hostname_info() -> String {
        // Try to get hostname from environment variable
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            if !hostname.is_empty() {
                return hostname;
            }
        }
        
        // Try hostname command
        if let Some(output) = Self::run_command("hostname", &[]) {
            let hostname = output.trim();
            if !hostname.is_empty() {
                return hostname.to_string();
            }
        }
        
        // Try reading /etc/hostname
        if let Ok(content) = std::fs::read_to_string("/etc/hostname") {
            let hostname = content.trim();
            if !hostname.is_empty() {
                return hostname.to_string();
            }
        }
        
        // Try uname -n
        if let Some(output) = Self::run_command("uname", &["-n"]) {
            let hostname = output.trim();
            if !hostname.is_empty() {
                return hostname.to_string();
            }
        }
        
        "Unknown Hostname".to_string()
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
            let lines: Vec<&str> = output.lines().collect();
            let mut current_display_resolution = None;
            
            // First pass: find the connected display and its resolution
            for (i, line) in lines.iter().enumerate() {
                if line.contains(" connected primary") || (line.contains(" connected ") && !line.contains(" disconnected")) {
                    // Extract resolution from connected line like "DP-3 connected primary 3440x1440+0+0"
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in &parts {
                        if part.contains("x") && part.chars().next().unwrap_or('a').is_ascii_digit() {
                            let resolution = part.split('+').next().unwrap_or("Unknown");
                            current_display_resolution = Some((resolution.to_string(), i));
                            break;
                        }
                    }
                    break;
                }
            }
            
            // Second pass: look for the refresh rate in the following indented lines
            if let Some((resolution, display_line_idx)) = current_display_resolution {
                // Look at the lines following the connected display line
                for line in lines.iter().skip(display_line_idx + 1) {
                    // Stop if we hit another display or non-indented line
                    if !line.starts_with("   ") || line.contains(" connected") {
                        break;
                    }
                    
                    // Look for the line with the current resolution and * marker
                    if line.contains("*") {
                        let parts: Vec<&str> = line.trim().split_whitespace().collect();
                        if let Some(line_resolution) = parts.first() {
                            // Check if this line matches our display resolution
                            if line_resolution == &resolution {
                                // Look for refresh rate in the same line
                                for part in &parts {
                                    if part.contains("*") {
                                        let rate_str = part.replace("*", "").replace("+", "");
                                        if let Ok(rate) = rate_str.parse::<f32>() {
                                            return format!("{} @ {:.0}Hz", resolution, rate);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // If no refresh rate found, return just resolution
                return resolution;
            }
        }

        // Try wlr-randr for Wayland
        if let Some(output) = Self::run_command("wlr-randr", &[]) {
            for line in output.lines() {
                if line.contains("current") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let mut resolution = None;
                    let mut refresh_rate = None;
                    
                    for part in &parts {
                        if part.contains("x") && part.chars().next().unwrap_or('a').is_ascii_digit() {
                            resolution = Some(part.to_string());
                        } else if part.ends_with("Hz") {
                            if let Ok(rate) = part.replace("Hz", "").parse::<f32>() {
                                refresh_rate = Some(rate);
                            }
                        }
                    }
                    
                    if let Some(res) = resolution {
                        if let Some(rate) = refresh_rate {
                            return format!("{} @ {:.0}Hz", res, rate);
                        } else {
                            return res;
                        }
                    }
                }
            }
        }
        
        // Try swaymsg for Sway/Wayland
        if let Some(output) = Self::run_command("swaymsg", &["-t", "get_outputs"]) {
            // This would require JSON parsing, but let's try a simple approach
            if output.contains("current_mode") {
                // Look for patterns like "width":3440,"height":1440,"refresh":165000
                let lines: Vec<&str> = output.lines().collect();
                for line in lines {
                    if line.contains("current_mode") {
                        // Simple regex-like parsing for width, height, refresh
                        let mut width = None;
                        let mut height = None;
                        let mut refresh = None;
                        
                        if let Some(w_start) = line.find("\"width\":") {
                            if let Some(w_end) = line[w_start+8..].find(',') {
                                if let Ok(w) = line[w_start+8..w_start+8+w_end].parse::<u32>() {
                                    width = Some(w);
                                }
                            }
                        }
                        
                        if let Some(h_start) = line.find("\"height\":") {
                            if let Some(h_end) = line[h_start+9..].find(',') {
                                if let Ok(h) = line[h_start+9..h_start+9+h_end].parse::<u32>() {
                                    height = Some(h);
                                }
                            }
                        }
                        
                        if let Some(r_start) = line.find("\"refresh\":") {
                            if let Some(r_end) = line[r_start+10..].find(',') {
                                if let Ok(r) = line[r_start+10..r_start+10+r_end].parse::<u32>() {
                                    refresh = Some(r / 1000); // Convert from mHz to Hz
                                }
                            } else if let Some(r_end) = line[r_start+10..].find('}') {
                                if let Ok(r) = line[r_start+10..r_start+10+r_end].parse::<u32>() {
                                    refresh = Some(r / 1000); // Convert from mHz to Hz
                                }
                            }
                        }
                        
                        if let (Some(w), Some(h)) = (width, height) {
                            if let Some(r) = refresh {
                                return format!("{}x{} @ {}Hz", w, h, r);
                            } else {
                                return format!("{}x{}", w, h);
                            }
                        }
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

    fn get_os_age() -> String {
        use std::time::SystemTime;
        
        // Try different approaches to find OS installation date
        let install_paths = [
            "/lost+found",           // Root filesystem creation (Linux)
            "/var/log/installer",    // Ubuntu/Debian installer logs
            "/var/log/anaconda",     // Red Hat/Fedora installer logs
            "/etc",                  // Fallback: /etc directory
            "/boot",                 // Boot directory
        ];
        
        let mut oldest_time: Option<SystemTime> = None;
        
        for path in &install_paths {
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(created) = metadata.created().or_else(|_| metadata.modified()) {
                    match oldest_time {
                        None => oldest_time = Some(created),
                        Some(existing) => {
                            if created < existing {
                                oldest_time = Some(created);
                            }
                        }
                    }
                }
            }
        }
        
        // Calculate days since installation
        if let Some(install_time) = oldest_time {
            if let Ok(duration) = SystemTime::now().duration_since(install_time) {
                let days = duration.as_secs() / (24 * 60 * 60);
                return format!("{} days", days);
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
