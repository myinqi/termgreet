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
    pub fn gather_with_config(config: &crate::config::Config) -> Self {
        let mut data = HashMap::new();
        let mut sys = System::new_all();
        sys.refresh_all();

        // OS Information
        data.insert("OS".to_string(), Self::get_os_info());
        data.insert("KERNEL".to_string(), Self::get_kernel_version());
        data.insert("LINUX".to_string(), Self::get_linux_info());
        data.insert("UPTIME".to_string(), Self::format_uptime(System::uptime()));
        data.insert("OS_AGE".to_string(), Self::get_os_age());

        // Desktop Environment / Window Manager
        data.insert("DE".to_string(), Self::get_desktop_environment());
        data.insert("WM".to_string(), Self::get_window_manager());

        // Shell and Terminal - use version functions if enabled
        if config.modules.show_versions {
            data.insert("SHELL".to_string(), Self::get_shell_with_version());
            data.insert("TERMINAL".to_string(), Self::get_terminal_with_version());
        } else {
            data.insert("SHELL".to_string(), Self::get_shell());
            data.insert("TERMINAL".to_string(), Self::get_terminal());
        }
        data.insert("TERMINAL_SHELL_COMBINED".to_string(), Self::get_terminal_shell_combined(config.modules.show_versions));
        data.insert("FONT".to_string(), Self::get_font_info());
        data.insert("USER".to_string(), Self::get_user_info());
        data.insert("HOSTNAME".to_string(), Self::get_hostname_info());
        data.insert("USER_AT_HOST".to_string(), Self::get_user_at_host_info());

        // Hardware Information
        data.insert("CPU".to_string(), Self::get_cpu_info(&sys));
        data.insert("CPU_TEMP".to_string(), Self::get_cpu_temperature());
        data.insert("GPU".to_string(), Self::get_gpu_info());
        data.insert("GPU_TEMP".to_string(), Self::get_gpu_temperature());
        data.insert("TEMP_COMBINED".to_string(), Self::get_temp_combined());
        data.insert("GPU_DRIVER".to_string(), Self::get_gpu_driver_info());
        data.insert("MEMORY".to_string(), Self::get_memory_info(&sys));
        data.insert("DISK".to_string(), Self::get_disk_info(&sys));
        data.insert("DYSK".to_string(), Self::get_dysk_info());

        // Display Information
        data.insert("RESOLUTION".to_string(), Self::get_resolution());
        
        // Network Information
        data.insert("NETWORK".to_string(), Self::get_network_info());
        data.insert("PUBLIC_IP".to_string(), Self::get_public_ip_info());

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

        // Flatpak packages (if available)
        if let Some(flatpak_packages) = Self::get_flatpak_packages() {
            data.insert("FLATPAK_PACKAGES".to_string(), flatpak_packages);
        }

        // Combined packages (main + flatpak)
        if let Some(combined_packages) = Self::get_combined_packages() {
            data.insert("PACKAGES_COMBINED".to_string(), combined_packages);
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

    fn get_linux_info() -> String {
        let os = Self::get_os_info();
        let kernel = Self::get_kernel_version();
        format!("{} - {}", os, kernel)
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
    
    fn get_shell_with_version() -> String {
        let shell_name = env::var("SHELL")
            .map(|shell| {
                shell.split('/').last().unwrap_or("Unknown").to_string()
            })
            .unwrap_or_else(|_| "Unknown".to_string());
        
        if shell_name == "Unknown" {
            return shell_name;
        }
        
        // Try to get version for common shells
        match shell_name.as_str() {
            "bash" => {
                if let Some(output) = Self::run_command("bash", &["--version"]) {
                    if let Some(line) = output.lines().next() {
                        // Parse version from "GNU bash, version 5.2.21(1)-release"
                        if let Some(version_start) = line.find("version ") {
                            let version_part = &line[version_start + 8..];
                            if let Some(version_end) = version_part.find('(') {
                                let version = &version_part[..version_end];
                                return format!("{} {}", shell_name, version);
                            } else if let Some(version_end) = version_part.find(' ') {
                                let version = &version_part[..version_end];
                                return format!("{} {}", shell_name, version);
                            }
                        }
                    }
                }
            },
            "zsh" => {
                if let Some(output) = Self::run_command("zsh", &["--version"]) {
                    if let Some(line) = output.lines().next() {
                        // Parse version from "zsh 5.9 (x86_64-pc-linux-gnu)"
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            return format!("{} {}", shell_name, parts[1]);
                        }
                    }
                }
            },
            "fish" => {
                if let Some(output) = Self::run_command("fish", &["--version"]) {
                    if let Some(line) = output.lines().next() {
                        // Parse version from "fish, version 3.6.1"
                        if let Some(version_start) = line.find("version ") {
                            let version = &line[version_start + 8..].trim();
                            return format!("{} {}", shell_name, version);
                        }
                    }
                }
            },
            "dash" => {
                // dash doesn't have a --version flag, try to get from package manager
                if let Some(output) = Self::run_command("dpkg", &["-l", "dash"]) {
                    for line in output.lines() {
                        if line.contains("dash") && line.starts_with("ii") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 3 {
                                return format!("{} {}", shell_name, parts[2]);
                            }
                        }
                    }
                }
            },
            _ => {
                // Try generic --version for other shells
                if let Some(output) = Self::run_command(&shell_name, &["--version"]) {
                    if let Some(line) = output.lines().next() {
                        // Try to extract version number
                        let words: Vec<&str> = line.split_whitespace().collect();
                        for word in &words {
                            // Look for version-like patterns (e.g., "1.2.3", "5.9")
                            if word.chars().next().unwrap_or('a').is_ascii_digit() && word.contains('.') {
                                return format!("{} {}", shell_name, word);
                            }
                        }
                    }
                }
            }
        }
        
        shell_name
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

    fn get_terminal_with_version() -> String {
        let terminal_name = Self::get_terminal();
        
        if terminal_name == "Unknown" {
            return terminal_name;
        }
        
        // Try to get version for common terminals
        match terminal_name.as_str() {
            "kitty" => {
                if let Some(output) = Self::run_command("kitty", &["--version"]) {
                    if let Some(line) = output.lines().next() {
                        // Parse version from "kitty 0.32.2"
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            return format!("{} {}", terminal_name, parts[1]);
                        }
                    }
                }
            },
            "ghostty" => {
                if let Some(output) = Self::run_command("ghostty", &["--version"]) {
                    if let Some(line) = output.lines().next() {
                        // Parse version from "ghostty 1.0.0"
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            return format!("{} {}", terminal_name, parts[1]);
                        }
                    }
                }
            },
            "alacritty" => {
                if let Some(output) = Self::run_command("alacritty", &["--version"]) {
                    if let Some(line) = output.lines().next() {
                        // Parse version from "alacritty 0.13.2"
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            return format!("{} {}", terminal_name, parts[1]);
                        }
                    }
                }
            },
            "gnome-terminal" => {
                if let Some(output) = Self::run_command("gnome-terminal", &["--version"]) {
                    if let Some(line) = output.lines().next() {
                        // Parse version from "GNOME Terminal 3.48.2 using VTE 0.70.3 +BIDI +GNUTLS +ICU +SYSTEMD"
                        if let Some(version_start) = line.find("Terminal ") {
                            let version_part = &line[version_start + 9..];
                            if let Some(version_end) = version_part.find(' ') {
                                let version = &version_part[..version_end];
                                return format!("GNOME Terminal {}", version);
                            }
                        }
                    }
                }
            },
            "konsole" => {
                if let Some(output) = Self::run_command("konsole", &["--version"]) {
                    for line in output.lines() {
                        if line.starts_with("konsole ") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 2 {
                                return format!("{} {}", terminal_name, parts[1]);
                            }
                        }
                    }
                }
            },
            "wezterm" => {
                if let Some(output) = Self::run_command("wezterm", &["--version"]) {
                    if let Some(line) = output.lines().next() {
                        // Parse version from "wezterm 20240203-110809-5046fc22"
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            return format!("{} {}", terminal_name, parts[1]);
                        }
                    }
                }
            },
            _ => {
                // Try generic --version for other terminals
                if let Some(output) = Self::run_command(&terminal_name, &["--version"]) {
                    if let Some(line) = output.lines().next() {
                        // Try to extract version number
                        let words: Vec<&str> = line.split_whitespace().collect();
                        for word in &words {
                            // Look for version-like patterns (e.g., "1.2.3", "0.13.2")
                            if word.chars().next().unwrap_or('a').is_ascii_digit() && word.contains('.') {
                                return format!("{} {}", terminal_name, word);
                            }
                        }
                    }
                }
            }
        }
        
        terminal_name
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
                    // Try to get VRAM usage for NVIDIA GPUs (used/total)
                    if let Some(vram_output) = Self::run_command("nvidia-smi", &["--query-gpu=memory.used,memory.total", "--format=csv,noheader,nounits"]) {
                        if let Some(vram_line) = vram_output.lines().next() {
                            let parts: Vec<&str> = vram_line.split(',').map(|s| s.trim()).collect();
                            if parts.len() == 2 {
                                if let (Ok(used_mb), Ok(total_mb)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                                    let used_gb = used_mb as f64 / 1024.0;
                                    let total_gb = total_mb as f64 / 1024.0;
                                    return format!("{} ({:.1}GB / {:.1}GB)", clean_name, used_gb, total_gb);
                                }
                            }
                        }
                    }
                    // Fallback to name only if VRAM query fails
                    return clean_name.to_string();
                }
            }
        }

        // Try lspci as fallback (and attempt VRAM detection for AMD/Intel)
        if let Some(output) = Self::run_command("lspci", &[]) {
            for line in output.lines() {
                if line.contains("VGA compatible controller") || line.contains("3D controller") {
                    if let Some(gpu) = line.split(": ").nth(1) {
                        let clean_gpu_name = Self::parse_gpu_name(gpu);
                        
                        // Try to get VRAM usage for AMD/Intel GPUs
                        if let Some((used_gb, total_gb)) = Self::get_non_nvidia_vram_usage() {
                            return format!("{} ({:.1}GB / {:.1}GB)", clean_gpu_name, used_gb, total_gb);
                        }
                        
                        return clean_gpu_name;
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

    fn get_user_at_host_info() -> String {
        let user = Self::get_user_info();
        let hostname = Self::get_hostname_info();
        format!("{}@{}", user, hostname)
    }

    fn get_gpu_driver_info() -> String {
        // Try to detect GPU driver from various sources
        
        // Method 1: Check loaded kernel modules for GPU drivers
        if let Some(output) = Self::run_command("lsmod", &[]) {
            let lines: Vec<&str> = output.lines().collect();
            
            // Check for NVIDIA drivers
            for line in &lines {
                if line.starts_with("nvidia ") {
                    // Check if this is the open source driver by looking for nvidia-open packages
                    let is_open_source = Self::is_nvidia_open_source_driver();
                    
                    // Try to get NVIDIA driver version
                    if let Some(version_output) = Self::run_command("nvidia-smi", &["--query-gpu=driver_version", "--format=csv,noheader,nounits"]) {
                        let version = version_output.trim();
                        if !version.is_empty() {
                            if is_open_source {
                                return format!("NVIDIA (open source) {}", version);
                            } else {
                                return format!("NVIDIA (proprietary) {}", version);
                            }
                        }
                    }
                    
                    if is_open_source {
                        return "NVIDIA (open source)".to_string();
                    } else {
                        return "NVIDIA (proprietary)".to_string();
                    }
                }
            }
            
            // Check for AMD drivers
            for line in &lines {
                if line.starts_with("amdgpu ") {
                    return "AMDGPU (open source)".to_string();
                } else if line.starts_with("radeon ") {
                    return "Radeon (open source)".to_string();
                }
            }
            
            // Check for Intel drivers
            for line in &lines {
                if line.starts_with("i915 ") {
                    return "Intel i915 (open source)".to_string();
                } else if line.starts_with("xe ") {
                    return "Intel Xe (open source)".to_string();
                }
            }
            
            // Check for other drivers
            for line in &lines {
                if line.starts_with("nouveau ") {
                    return "Nouveau (open source)".to_string();
                } else if line.starts_with("vmwgfx ") {
                    return "VMware SVGA (open source)".to_string();
                } else if line.starts_with("virtio_gpu ") {
                    return "VirtIO GPU (open source)".to_string();
                }
            }
        }
        
        // Method 2: Check /proc/driver/nvidia/version for NVIDIA
        if let Ok(content) = std::fs::read_to_string("/proc/driver/nvidia/version") {
            if let Some(line) = content.lines().next() {
                // Parse version from line like "NVRM version: NVIDIA UNIX x86_64 Kernel Module  535.154.05"
                if let Some(version_start) = line.rfind(" ") {
                    let version = &line[version_start + 1..];
                    if !version.is_empty() {
                        return format!("NVIDIA (proprietary) {}", version);
                    }
                }
                return "NVIDIA (proprietary)".to_string();
            }
        }
        
        // Method 3: Check dmesg for GPU driver initialization (requires root or specific permissions)
        if let Some(output) = Self::run_command("dmesg", &[]) {
            let lines: Vec<&str> = output.lines().collect();
            
            // Look for NVIDIA driver messages
            for line in lines.iter().rev().take(1000) { // Check last 1000 lines
                if line.contains("NVIDIA") && (line.contains("driver") || line.contains("GPU")) {
                    if line.contains("version") {
                        // Try to extract version from dmesg
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        for (i, part) in parts.iter().enumerate() {
                            if *part == "version" && i + 1 < parts.len() {
                                return format!("NVIDIA (proprietary) {}", parts[i + 1]);
                            }
                        }
                    }
                    return "NVIDIA (proprietary)".to_string();
                }
                
                // Look for AMD driver messages
                if line.contains("amdgpu") && line.contains("driver") {
                    return "AMDGPU (open source)".to_string();
                }
                
                // Look for Intel driver messages
                if line.contains("i915") && line.contains("driver") {
                    return "Intel i915 (open source)".to_string();
                }
            }
        }
        
        // Method 4: Check glxinfo for OpenGL driver info
        if let Some(output) = Self::run_command("glxinfo", &["-B"]) {
            for line in output.lines() {
                if line.contains("OpenGL renderer string:") {
                    if line.contains("NVIDIA") {
                        return "NVIDIA (proprietary)".to_string();
                    } else if line.contains("AMD") || line.contains("Radeon") {
                        return "AMDGPU/Radeon (open source)".to_string();
                    } else if line.contains("Intel") {
                        return "Intel (open source)".to_string();
                    }
                }
            }
        }
        
        // Method 5: Check /sys/module for loaded GPU modules
        let gpu_modules = [
            ("/sys/module/nvidia", "NVIDIA (proprietary)"),
            ("/sys/module/amdgpu", "AMDGPU (open source)"),
            ("/sys/module/radeon", "Radeon (open source)"),
            ("/sys/module/i915", "Intel i915 (open source)"),
            ("/sys/module/xe", "Intel Xe (open source)"),
            ("/sys/module/nouveau", "Nouveau (open source)"),
        ];
        
        for (path, driver_name) in &gpu_modules {
            if std::path::Path::new(path).exists() {
                return driver_name.to_string();
            }
        }
        
        "Unknown Driver".to_string()
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
        // Get specific important mountpoints with filesystem info
        let mut disk_info = Vec::new();
        
        // Always check root partition first
        if let Some(info) = Self::get_disk_info_with_filesystem("/") {
            if !info.is_empty() && info != "Unknown" {
                disk_info.push(format!("/ {}", info));
            }
        }
        
        // Check /boot/efi if it exists
        if let Some(info) = Self::get_disk_info_with_filesystem("/boot/efi") {
            if !info.is_empty() && info != "Unknown" {
                disk_info.push(format!("/boot {}", info));
            }
        }
        
        // Check /home only if it's on a separate device from /
        if Self::is_separate_partition("/", "/home") {
            if let Some(info) = Self::get_disk_info_with_filesystem("/home") {
                if !info.is_empty() && info != "Unknown" {
                    disk_info.push(format!("/home {}", info));
                }
            }
        }
        
        // Join with bullet separator
        if !disk_info.is_empty() {
            disk_info.join(" • ")
        } else {
            // Fallback: try to get at least root filesystem info
            Self::get_disk_info_with_filesystem("/").unwrap_or_else(|| "Unknown".to_string())
        }
    }
    
    fn get_single_disk_info(mount_point: &str) -> Option<String> {
        // Simple wrapper for backward compatibility
        Self::get_single_disk_info_with_device(mount_point).map(|(info, _)| info)
    }
    
    fn get_disk_info_with_filesystem(mount_point: &str) -> Option<String> {
        // Try multiple approaches to get disk info with filesystem type
        
        // Method 1: Use df -hT command with POSIX locale
        if let Some(output) = Self::run_command_with_env("df", &["-hT", mount_point], &[("LC_ALL", "C")]) {
            if let Some(result) = Self::parse_df_output_with_fs(&output, "LC_ALL=C") {
                return Some(result);
            }
        }
        
        // Method 2: Try df -hT without locale override
        if let Some(output) = Self::run_command("df", &["-hT", mount_point]) {
            if let Some(result) = Self::parse_df_output_with_fs(&output, "default") {
                return Some(result);
            }
        }
        
        // Method 3: Try df -hT with explicit LANG=C
        if let Some(output) = Self::run_command_with_env("df", &["-hT", mount_point], &[("LANG", "C")]) {
            if let Some(result) = Self::parse_df_output_with_fs(&output, "LANG=C") {
                return Some(result);
            }
        }
        
        // Fallback: try without filesystem type
        Self::get_single_disk_info(mount_point)
    }
    
    fn is_separate_partition(mount1: &str, mount2: &str) -> bool {
        // Check if two mount points are on separate devices/partitions
        let device1 = Self::get_device_for_mountpoint(mount1);
        let device2 = Self::get_device_for_mountpoint(mount2);
        
        match (device1, device2) {
            (Some(d1), Some(d2)) => d1 != d2,
            _ => false, // If we can't determine devices, assume they're the same
        }
    }
    
    fn get_device_for_mountpoint(mount_point: &str) -> Option<String> {
        // Get the device name for a specific mount point using df
        if let Some(output) = Self::run_command_with_env("df", &[mount_point], &[("LC_ALL", "C")]) {
            for line in output.lines().skip(1) { // Skip header line
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 1 {
                    return Some(parts[0].to_string());
                }
            }
        }
        None
    }
    
    fn get_single_disk_info_with_device(mount_point: &str) -> Option<(String, String)> {
        // Try multiple approaches to get disk info for a specific mount point
        
        // Method 1: Use df command with POSIX locale
        if let Some(output) = Self::run_command_with_env("df", &["-h", mount_point], &[("LC_ALL", "C")]) {
            if let Some((result, device)) = Self::parse_df_output_with_device(&output, "LC_ALL=C") {
                return Some((result, device));
            }
        }
        
        // Method 2: Try df without locale override
        if let Some(output) = Self::run_command("df", &["-h", mount_point]) {
            if let Some((result, device)) = Self::parse_df_output_with_device(&output, "default") {
                return Some((result, device));
            }
        }
        
        // Method 3: Try df with explicit LANG=C
        if let Some(output) = Self::run_command_with_env("df", &["-h", mount_point], &[("LANG", "C")]) {
            if let Some((result, device)) = Self::parse_df_output_with_device(&output, "LANG=C") {
                return Some((result, device));
            }
        }
        
        None
    }
    

    
    fn parse_df_output_with_device(output: &str, _method: &str) -> Option<(String, String)> {
        for line in output.lines().skip(1) { // Skip header line
            let parts: Vec<&str> = line.split_whitespace().collect();
            
            // Handle different df output formats
            if parts.len() >= 6 {
                // Standard format: Filesystem Size Used Avail Use% Mounted
                let device = parts[0].to_string();
                let total = parts[1];
                let used = parts[2];
                let usage = parts[4];
                return Some((format!("{} / {} ({})", used, total, usage), device));
            } else if parts.len() >= 5 {
                // Alternative format: might have filesystem on separate line
                let device = parts[0].to_string();
                let total = parts[1];
                let used = parts[2];
                let usage = parts[4];
                return Some((format!("{} / {} ({})", used, total, usage), device));
            } else if parts.len() == 4 {
                // Another format variation
                let device = "unknown".to_string();
                let total = parts[0];
                let used = parts[1];
                let usage = parts[2];
                return Some((format!("{} / {} ({})", used, total, usage), device));
            }
        }
        None
    }
    
    fn parse_df_output_with_fs(output: &str, _method: &str) -> Option<String> {
        for line in output.lines().skip(1) { // Skip header line
            let parts: Vec<&str> = line.split_whitespace().collect();
            
            // Handle df -hT output format: Filesystem Type Size Used Avail Use% Mounted
            if parts.len() >= 7 {
                // Standard format with filesystem type: Filesystem Type Size Used Avail Use% Mounted
                let filesystem = parts[1];
                let total = parts[2];
                let used = parts[3];
                let usage = parts[5];
                return Some(format!("{} / {} ({}) [{}]", used, total, usage, filesystem));
            } else if parts.len() >= 6 {
                // Alternative format: might be missing some fields
                let filesystem = parts[1];
                let total = parts[2];
                let used = parts[3];
                let usage = parts[4];
                return Some(format!("{} / {} ({}) [{}]", used, total, usage, filesystem));
            }
        }
        None
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

    fn get_flatpak_packages() -> Option<String> {
        // Check if flatpak is installed and get package count
        if let Some(output) = Self::run_command("flatpak", &["list", "--app"]) {
            let count = output.lines().filter(|line| !line.trim().is_empty()).count();
            if count > 0 {
                return Some(format!("{} (flatpak)", count));
            }
        }
        None
    }

    fn get_combined_packages() -> Option<String> {
        let mut package_parts = Vec::new();
        
        // Get main package manager count
        if let Some(packages) = Self::get_package_count() {
            package_parts.push(packages);
        }
        
        // Get flatpak count
        if let Some(flatpak) = Self::get_flatpak_packages() {
            package_parts.push(flatpak);
        }
        
        if !package_parts.is_empty() {
            Some(package_parts.join(", "))
        } else {
            None
        }
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

    fn run_command(command: &str, args: &[&str]) -> Option<String> {
        match std::process::Command::new(command)
            .args(args)
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn run_command_with_env(command: &str, args: &[&str], env_vars: &[(&str, &str)]) -> Option<String> {
        let mut cmd = std::process::Command::new(command);
        cmd.args(args);
        
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
        
        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn get_cpu_temperature() -> String {
        // Try to get CPU temperature from sensors command
        if let Ok(output) = std::process::Command::new("sensors")
            .output()
        {
            if output.status.success() {
                let sensors_output = String::from_utf8_lossy(&output.stdout);
                
                // Look for k10temp (AMD) or coretemp (Intel) blocks
                for line in sensors_output.lines() {
                    let line = line.trim();
                    
                    // AMD: Look for Tctl temperature
                    if line.starts_with("Tctl:") {
                        if let Some(temp_part) = line.split_whitespace().nth(1) {
                            if let Some(temp) = temp_part.strip_prefix("+").and_then(|t| t.strip_suffix("°C")) {
                                return format!("{}°C", temp);
                            }
                        }
                    }
                    
                    // Intel: Look for Core 0 temperature (first core as representative)
                    if line.starts_with("Core 0:") {
                        if let Some(temp_part) = line.split_whitespace().nth(2) {
                            if let Some(temp) = temp_part.strip_prefix("+").and_then(|t| t.strip_suffix("°C")) {
                                return format!("{}°C", temp);
                            }
                        }
                    }
                    
                    // Generic: Look for Package id 0 (Intel)
                    if line.starts_with("Package id 0:") {
                        if let Some(temp_part) = line.split_whitespace().nth(3) {
                            if let Some(temp) = temp_part.strip_prefix("+").and_then(|t| t.strip_suffix("°C")) {
                                return format!("{}°C", temp);
                            }
                        }
                    }
                }
            }
        }
        
        "N/A".to_string()
    }

    fn get_gpu_temperature() -> String {
        // Try NVIDIA first (nvidia-smi)
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=temperature.gpu", "--format=csv,noheader"])
            .output()
        {
            if output.status.success() {
                let temp_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !temp_str.is_empty() && temp_str != "N/A" {
                    // Parse as float and format with one decimal place
                    if let Ok(temp_float) = temp_str.parse::<f32>() {
                        return format!("{:.1}°C", temp_float);
                    }
                    return format!("{}°C", temp_str);
                }
            }
        }
        
        // Try AMD via sensors (amdgpu)
        if let Ok(output) = std::process::Command::new("sensors")
            .output()
        {
            if output.status.success() {
                let sensors_output = String::from_utf8_lossy(&output.stdout);
                
                // Look for amdgpu temperature
                for line in sensors_output.lines() {
                    let line = line.trim();
                    
                    if line.starts_with("edge:") || line.starts_with("junction:") {
                        if let Some(temp_part) = line.split_whitespace().nth(1) {
                            if let Some(temp) = temp_part.strip_prefix("+").and_then(|t| t.strip_suffix("°C")) {
                                // Parse as float and format with one decimal place
                                if let Ok(temp_float) = temp.parse::<f32>() {
                                    return format!("{:.1}°C", temp_float);
                                }
                                return format!("{}°C", temp);
                            }
                        }
                    }
                }
            }
        }
        
        "N/A".to_string()
    }

    fn get_temp_combined() -> String {
        let cpu_temp = Self::get_cpu_temperature();
        let gpu_temp = Self::get_gpu_temperature();
        
        // Only show if both temperatures are available
        if cpu_temp != "N/A" && gpu_temp != "N/A" {
            format!("CPU {} • GPU {}", cpu_temp, gpu_temp)
        } else if cpu_temp != "N/A" {
            // Show only CPU if GPU is not available
            format!("CPU {}", cpu_temp)
        } else if gpu_temp != "N/A" {
            // Show only GPU if CPU is not available
            format!("GPU {}", gpu_temp)
        } else {
            // Neither available
            "N/A".to_string()
        }
    }

    fn get_terminal_shell_combined(show_versions: bool) -> String {
        let terminal = if show_versions {
            Self::get_terminal_with_version()
        } else {
            Self::get_terminal()
        };
        
        let shell = if show_versions {
            Self::get_shell_with_version()
        } else {
            Self::get_shell()
        };
        
        // Only show if both are available and not "Unknown"
        if terminal != "Unknown" && shell != "Unknown" {
            format!("{} • {}", terminal, shell)
        } else if terminal != "Unknown" {
            // Show only terminal if shell is not available
            terminal
        } else if shell != "Unknown" {
            // Show only shell if terminal is not available
            shell
        } else {
            // Neither available
            "Unknown".to_string()
        }
    }

    fn get_non_nvidia_vram_usage() -> Option<(f64, f64)> {
        // Method 1: Try glxinfo for OpenGL memory info
        if let Some(output) = Self::run_command("glxinfo", &[]) {
            for line in output.lines() {
                let line = line.trim();
                // Look for dedicated video memory
                if line.contains("Dedicated video memory:") {
                    if let Some(memory_part) = line.split(":").nth(1) {
                        if let Some(mb_str) = memory_part.trim().split_whitespace().next() {
                            if let Ok(mb) = mb_str.parse::<u32>() {
                                let total_gb = mb as f64 / 1024.0;
                                return Some((0.0, total_gb)); // Can't get used from glxinfo
                            }
                        }
                    }
                }
                // Alternative: Video memory
                if line.contains("Video memory:") && line.contains("MB") {
                    if let Some(memory_part) = line.split(":").nth(1) {
                        if let Some(mb_str) = memory_part.trim().split_whitespace().next() {
                            if let Ok(mb) = mb_str.parse::<u32>() {
                                let total_gb = mb as f64 / 1024.0;
                                return Some((0.0, total_gb)); // Can't get used from glxinfo
                            }
                        }
                    }
                }
            }
        }
        
        // Method 2: Try reading from /sys/class/drm/ for AMD GPUs
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("card") && !name.contains("-") {
                        // Try to read both used and total VRAM from AMD GPU
                        let vram_used_path = path.join("device/mem_info_vram_used");
                        let vram_total_path = path.join("device/mem_info_vram_total");
                        
                        if let (Ok(used_content), Ok(total_content)) = (
                            std::fs::read_to_string(&vram_used_path),
                            std::fs::read_to_string(&vram_total_path)
                        ) {
                            if let (Ok(used_bytes), Ok(total_bytes)) = (
                                used_content.trim().parse::<u64>(),
                                total_content.trim().parse::<u64>()
                            ) {
                                let used_gb = used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                                let total_gb = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                                if total_gb > 0.1 { // Only return if we have significant VRAM
                                    return Some((used_gb, total_gb));
                                }
                            }
                        }
                        
                        // Fallback: Try to get total only from alternative paths
                        let vram_size_path = path.join("device/vram_size");
                        if let Ok(vram_content) = std::fs::read_to_string(&vram_size_path) {
                            if let Ok(vram_bytes) = vram_content.trim().parse::<u64>() {
                                let total_gb = vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                                if total_gb > 0.1 {
                                    // Can't get used VRAM, so return 0 as used
                                    return Some((0.0, total_gb));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Method 3: Try lspci -v for memory information
        if let Some(output) = Self::run_command("lspci", &["-v"]) {
            let mut in_vga_section = false;
            for line in output.lines() {
                if line.contains("VGA compatible controller") || line.contains("3D controller") {
                    in_vga_section = true;
                    continue;
                }
                
                if in_vga_section {
                    // Look for memory ranges
                    if line.trim().starts_with("Memory at") && line.contains("size=") {
                        if let Some(size_part) = line.split("size=").nth(1) {
                            let size_str = size_part.split_whitespace().next().unwrap_or("");
                            if size_str.ends_with("M") {
                                if let Ok(mb) = size_str.trim_end_matches('M').parse::<u32>() {
                                    if mb >= 512 { // Only consider if >= 512MB (likely VRAM)
                                        let total_gb = mb as f64 / 1024.0;
                                        return Some((0.0, total_gb)); // Can't get used from lspci
                                    }
                                }
                            } else if size_str.ends_with("G") {
                                if let Ok(gb) = size_str.trim_end_matches('G').parse::<f64>() {
                                    if gb >= 0.5 {
                                        return Some((0.0, gb)); // Can't get used from lspci
                                    }
                                }
                            }
                        }
                    }
                    
                    // Stop when we reach the next device
                    if line.starts_with(char::is_numeric) {
                        break;
                    }
                }
            }
        }
        
        None
    }

    fn get_network_info() -> String {
        // Try to get the primary network interface and its IP
        if let Some(output) = Self::run_command("ip", &["route", "show", "default"]) {
            // Extract default interface from "default via ... dev <interface>"
            if let Some(line) = output.lines().next() {
                if let Some(dev_pos) = line.find(" dev ") {
                    let after_dev = &line[dev_pos + 5..];
                    if let Some(interface) = after_dev.split_whitespace().next() {
                        // Get IP address for this interface
                        if let Some(ip_output) = Self::run_command("ip", &["addr", "show", interface]) {
                            for ip_line in ip_output.lines() {
                                let trimmed = ip_line.trim();
                                if trimmed.starts_with("inet ") && !trimmed.contains("127.0.0.1") {
                                    if let Some(ip_part) = trimmed.split_whitespace().nth(1) {
                                        if let Some(ip) = ip_part.split('/').next() {
                                            return format!("{} ({})", ip, interface);
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Fallback: show interface name only
                        return format!("Connected ({})", interface);
                    }
                }
            }
        }
        
        // Alternative: Try to find any active interface with IP
        if let Some(output) = Self::run_command("ip", &["addr", "show"]) {
            let mut current_interface = String::new();
            for line in output.lines() {
                let trimmed = line.trim();
                
                // Check for interface line (starts with number)
                if let Some(first_char) = trimmed.chars().next() {
                    if first_char.is_ascii_digit() {
                        if let Some(interface_part) = trimmed.split(':').nth(1) {
                            current_interface = interface_part.trim().to_string();
                        }
                    }
                }
                
                // Check for inet line with non-loopback IP
                if trimmed.starts_with("inet ") && !trimmed.contains("127.0.0.1") && !current_interface.is_empty() {
                    if let Some(ip_part) = trimmed.split_whitespace().nth(1) {
                        if let Some(ip) = ip_part.split('/').next() {
                            return format!("{} ({})", ip, current_interface);
                        }
                    }
                }
            }
        }
        
        "Not connected".to_string()
    }

    fn get_public_ip_info() -> String {
        // Try multiple services to get public IP
        let services = [
            "ifconfig.me",
            "ipinfo.io/ip",
            "icanhazip.com",
            "checkip.amazonaws.com"
        ];
        
        for service in &services {
            if let Some(output) = Self::run_command("curl", &["-s", "--max-time", "3", service]) {
                let ip = output.trim();
                if !ip.is_empty() && ip.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    // Try to get ISP info
                    if let Some(isp) = Self::get_isp_info(&ip) {
                        return format!("{} ({})", ip, isp);
                    }
                    return ip.to_string();
                }
            }
        }
        
        "Not available".to_string()
    }
    
    fn get_isp_info(ip: &str) -> Option<String> {
        // Try to get ISP information from ipinfo.io
        let url = format!("ipinfo.io/{}/org", ip);
        if let Some(output) = Self::run_command("curl", &["-s", "--max-time", "3", &url]) {
            let org = output.trim();
            if !org.is_empty() && !org.contains("error") {
                // Clean up ISP name (remove AS numbers)
                if let Some(space_pos) = org.find(' ') {
                    let clean_name = &org[space_pos + 1..];
                    if !clean_name.is_empty() {
                        return Some(clean_name.to_string());
                    }
                }
                return Some(org.to_string());
            }
        }
        None
    }

    fn get_dysk_info() -> String {
        // Get all mounted filesystems using df command
        let mut mount_info = Vec::new();
        
        // Use df to get all mounted filesystems with human-readable sizes
        if let Some(output) = Self::run_command_with_env("df", &["-hT"], &[("LC_ALL", "C")]) {
            for line in output.lines().skip(1) { // Skip header line
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 7 {
                    let device = parts[0];
                    let filesystem = parts[1];
                    let total = parts[2];
                    let used = parts[3];
                    let _available = parts[4];
                    let usage_percent = parts[5];
                    let mount_point = parts[6];
                    
                    // Skip pseudo filesystems and special mounts
                    if Self::should_include_in_dysk(device, filesystem, mount_point) {
                        // Parse usage percentage
                        let usage_num = usage_percent.trim_end_matches('%')
                            .parse::<u32>()
                            .unwrap_or(0);
                        
                        // Create progress bar
                        let progress_bar = Self::create_progress_bar(usage_num);
                        
                        // Clean device name (remove /dev/ prefix)
                        let clean_device = device.strip_prefix("/dev/").unwrap_or(device);
                        
                        // Format the drive info in the new order:
                        // Progress Bar, Percent (3 chars), Device Name, Usage (fixed width), Filesystem, Mount Point
                        let drive_info = format!(
                            "{} {:>3}% {} {:>4}/{:<4} [{}] {}",
                            progress_bar,
                            usage_num,
                            clean_device,
                            used,
                            total,
                            filesystem,
                            mount_point
                        );
                        
                        mount_info.push(drive_info);
                    }
                }
            }
        }
        
        // Also check for additional mounted drives that might be missed
        if let Some(output) = Self::run_command("mount", &[]) {
            for line in output.lines() {
                if line.contains(" on ") && line.contains(" type ") {
                    let parts: Vec<&str> = line.split(" on ").collect();
                    if parts.len() >= 2 {
                        let device = parts[0].trim();
                        let rest = parts[1];
                        if let Some(type_pos) = rest.find(" type ") {
                            let mount_point = rest[..type_pos].trim();
                            
                            // Check if this is a removable/temporary mount we haven't seen
                            if Self::is_temporary_mount(device, mount_point) &&
                               !mount_info.iter().any(|info| info.contains(mount_point)) {
                                
                                // Get disk usage for this mount
                                if let Some(usage_info) = Self::get_disk_usage_for_mount(mount_point) {
                                    mount_info.push(usage_info);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if mount_info.is_empty() {
            "No mounted drives found".to_string()
        } else {
            mount_info.join("\n")
        }
    }
    
    fn should_include_in_dysk(device: &str, filesystem: &str, mount_point: &str) -> bool {
        // Skip pseudo filesystems
        let pseudo_fs = [
            "tmpfs", "devtmpfs", "sysfs", "proc", "devpts", "cgroup", "cgroup2",
            "pstore", "bpf", "configfs", "debugfs", "mqueue", "hugetlbfs",
            "fusectl", "securityfs", "tracefs", "overlay", "squashfs"
        ];
        
        if pseudo_fs.contains(&filesystem) {
            return false;
        }
        
        // Skip special mount points but allow /run/media (removable drives)
        let skip_mounts = [
            "/dev", "/proc", "/sys", "/tmp", "/var/tmp",
            "/dev/shm", "/run/lock", "/sys/fs/cgroup", "/run/user"
        ];
        
        // Check if it's a mount point we should skip, but make exceptions for real drives
        for &skip in &skip_mounts {
            if mount_point.starts_with(skip) {
                return false;
            }
        }
        
        // Skip /run/* except for /run/media (removable drives)
        if mount_point.starts_with("/run/") && !mount_point.starts_with("/run/media/") {
            return false;
        }
        
        // Skip loop devices unless they're meaningful (like snap packages)
        if device.starts_with("/dev/loop") && !mount_point.starts_with("/snap/") {
            return false;
        }
        
        // Include real block devices (nvme, sd, etc.)
        if device.starts_with("/dev/nvme") || device.starts_with("/dev/sd") || 
           device.starts_with("/dev/hd") || device.starts_with("/dev/vd") {
            return true;
        }
        
        // Include common filesystem types on real mount points
        let real_fs = ["ext4", "ext3", "ext2", "xfs", "btrfs", "ntfs", "vfat", "fat32", "exfat"];
        if real_fs.contains(&filesystem) {
            return true;
        }
        
        false
    }
    
    fn is_temporary_mount(device: &str, mount_point: &str) -> bool {
        // Check for removable media patterns - any real device in removable locations
        (device.starts_with("/dev/sd") || device.starts_with("/dev/nvme")) && (
            mount_point.contains("/media/") ||
            mount_point.contains("/mnt/") ||
            mount_point.contains("/run/media/")
        )
    }
    
    fn get_disk_usage_for_mount(mount_point: &str) -> Option<String> {
        if let Some(output) = Self::run_command_with_env("df", &["-hT", mount_point], &[("LC_ALL", "C")]) {
            for line in output.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 7 {
                    let device = parts[0];
                    let filesystem = parts[1];
                    let total = parts[2];
                    let used = parts[3];
                    let usage_percent = parts[5];
                    
                    let usage_num = usage_percent.trim_end_matches('%')
                        .parse::<u32>()
                        .unwrap_or(0);
                    
                    let progress_bar = Self::create_progress_bar(usage_num);
                    
                    // Clean device name (remove /dev/ prefix)
                    let clean_device = device.strip_prefix("/dev/").unwrap_or(device);
                    
                    return Some(format!(
                        "{} {:>3}% {} {:>4}/{:<4} [{}] {}",
                        progress_bar,
                        usage_num,
                        clean_device,
                        used,
                        total,
                        filesystem,
                        mount_point
                    ));
                }
            }
        }
        None
    }
    
    fn create_progress_bar(usage_percent: u32) -> String {
        let bar_length = 10;
        let filled_length = (usage_percent * bar_length / 100).min(bar_length);
        let empty_length = bar_length - filled_length;
        
        // Use different characters based on usage level
        let (fill_char, empty_char) = if usage_percent >= 90 {
            ("█", "░")  // Red zone - full blocks
        } else if usage_percent >= 70 {
            ("▓", "░")  // Yellow zone - medium blocks
        } else {
            ("▒", "░")  // Green zone - light blocks
        };
        
        format!("[{}{}]", 
                fill_char.repeat(filled_length as usize),
                empty_char.repeat(empty_length as usize))
    }
    
    fn is_nvidia_open_source_driver() -> bool {
        // Check for NVIDIA open source driver packages
        // This works for various distributions that have nvidia-open packages
        
        // Method 1: Check pacman (Arch/CachyOS/Vanilla Arch)
        if let Some(output) = Self::run_command("pacman", &["-Q"]) {
            // Search for any package containing "nvidia-open" in its name
            for line in output.lines() {
                if line.contains("nvidia-open") {
                    return true;
                }
            }
        }
        
        // Method 2: Check dpkg (Debian/Ubuntu)
        if let Some(output) = Self::run_command("dpkg", &["-l"]) {
            // Search for any package containing "nvidia-open" in its name
            for line in output.lines() {
                if line.contains("nvidia-open") {
                    return true;
                }
            }
        }
        
        // Method 3: Check rpm (Red Hat/Fedora)
        if let Some(output) = Self::run_command("rpm", &["-qa"]) {
            // Search for any package containing "nvidia-open" in its name
            for line in output.lines() {
                if line.contains("nvidia-open") {
                    return true;
                }
            }
        }
        
        // Method 4: Check for open source driver files
        if std::path::Path::new("/usr/lib/modules").exists() {
            if let Ok(entries) = std::fs::read_dir("/usr/lib/modules") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let nvidia_open_path = path.join("kernel/drivers/gpu/drm/nvidia-drm-open.ko");
                        if nvidia_open_path.exists() {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
}
