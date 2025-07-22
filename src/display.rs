use anyhow::Result;
use colored::*;
use viuer::{Config as ViuerConfig, print_from_file};

use crate::config::{Config, MotdConfig};
use crate::system_info::SystemInfo;

pub struct Display {
    config: Config,
    show_images: bool,
}

impl Display {
    pub fn new(config: Config, show_images: bool) -> Self {
        Self {
            config,
            show_images,
        }
    }

    pub fn show(&self, system_info: &SystemInfo) -> Result<()> {
        // Prepare system info lines
        let info_lines = self.prepare_system_info_lines(system_info);
        
        // Prepare ASCII art or image lines
        let art_lines = if self.show_images && self.config.display.show_image {
            self.prepare_image_lines()?
        } else if let Some(ref ascii_art) = self.config.display.ascii_art {
            ascii_art.lines().map(|s| s.to_string()).collect()
        } else {
            self.get_default_ascii_lines()
        };
        
        // Show title if configured
        if let Some(ref title) = self.config.general.title {
            println!("{}", self.apply_color(title, &self.config.general.colors.title));
            println!();
        }
        
        // Display in two columns (art left, info right)
        self.show_two_column_layout(&art_lines, &info_lines);
        
        Ok(())
    }

    pub fn show_motd(motd_config: &MotdConfig) {
        if !motd_config.enabled || motd_config.messages.is_empty() {
            return;
        }

        let message = if motd_config.random {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            use std::time::{SystemTime, UNIX_EPOCH};

            let mut hasher = DefaultHasher::new();
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .hash(&mut hasher);
            let index = (hasher.finish() as usize) % motd_config.messages.len();
            &motd_config.messages[index]
        } else {
            &motd_config.messages[0]
        };

        // Create a temporary Display instance for color application
        let temp_display = Display {
            config: crate::config::Config::default(),
            show_images: false,
        };
        println!("{}", temp_display.apply_color(message, &motd_config.color));
    }

    fn show_image(&self) -> Result<()> {
        if let Some(ref image_path) = self.config.display.image_path {
            if image_path.exists() {
                let viuer_config = ViuerConfig {
                    transparent: true,
                    absolute_offset: false,
                    width: Some(self.config.display.image_size.width),
                    height: Some(self.config.display.image_size.height),
                    ..Default::default()
                };

                match print_from_file(image_path, &viuer_config) {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        eprintln!("Warning: Failed to display image: {}", e);
                    }
                }
            }
        }

        // Fallback to default ASCII art if image fails
        // This method is now handled in prepare_image_lines
        Ok(())
    }

    fn prepare_image_lines(&self) -> Result<Vec<String>> {
        if let Some(ref image_path) = self.config.display.image_path {
            if image_path.exists() {
                // For now, return empty lines as image display in two-column layout is complex
                // This would need terminal-specific image positioning
                return Ok(vec!["[Image would be displayed here]".to_string()]);
            }
        }
        Ok(self.get_default_ascii_lines())
    }
    
    fn get_default_ascii_lines(&self) -> Vec<String> {
        let ascii_art = r#"╭─────────────────────╮
│                     │
│     TermGreet       │
│                     │
╰─────────────────────╯"#;
        ascii_art.lines().map(|line| {
            self.apply_color(line, &self.config.general.colors.title).to_string()
        }).collect()
    }
    
    fn prepare_system_info_lines(&self, system_info: &SystemInfo) -> Vec<String> {
        let modules = &self.config.modules;
        let colors = &self.config.general.colors;
        let separator = &self.config.general.separator;
        let mut lines = Vec::new();

        // Define the order and mapping of modules
        let module_order = [
            ("os", "OS", modules.os),
            ("kernel", "Kernel", modules.kernel),
            ("uptime", "Uptime", modules.uptime),
            ("packages", "Packages", modules.packages),
            ("shell", "Shell", modules.shell),
            ("resolution", "Resolution", modules.resolution),
            ("de", "DE", modules.de),
            ("wm", "WM", modules.wm),
            ("theme", "Theme", modules.theme),
            ("icons", "Icons", modules.icons),
            ("terminal", "Terminal", modules.terminal),
            ("cpu", "CPU", modules.cpu),
            ("gpu", "GPU", modules.gpu),
            ("memory", "Memory", modules.memory),
            ("disk", "Disk", modules.disk),
            ("battery", "Battery", modules.battery),
            ("locale", "Locale", modules.locale),
        ];

        for (key, display_name, enabled) in module_order {
            if enabled {
                if let Some(value) = system_info.data.get(&key.to_uppercase()) {
                    let line = format!(
                        "{}{}{}",
                        self.apply_color(display_name, &colors.title),
                        self.apply_color(separator, &colors.separator),
                        self.apply_color(value, &colors.info)
                    );
                    lines.push(line);
                }
            }
        }
        
        lines
    }
    
    fn show_two_column_layout(&self, art_lines: &[String], info_lines: &[String]) {
        let max_lines = art_lines.len().max(info_lines.len());
        let art_width = 25; // Fixed width for ASCII art column
        
        for i in 0..max_lines {
            let art_line = art_lines.get(i).cloned().unwrap_or_default();
            let info_line = info_lines.get(i).cloned().unwrap_or_default();
            
            // Calculate padding after ASCII art
            let art_display_len = self.strip_ansi_codes(&art_line).len();
            let padding_len = if art_display_len < art_width {
                art_width - art_display_len
            } else {
                2 // Minimum spacing
            };
            let padding = " ".repeat(padding_len);
            
            println!("{}{}{}", art_line, padding, info_line);
        }
        
        // Add some spacing after the layout
        for _ in 0..self.config.display.padding {
            println!();
        }
    }
    
    fn strip_ansi_codes(&self, text: &str) -> String {
        // More robust ANSI code removal for length calculation
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Skip escape sequence
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['
                    // Skip until we find a letter (end of escape sequence)
                    while let Some(next_ch) = chars.next() {
                        if next_ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                result.push(ch);
            }
        }
        
        result
    }


}

impl Display {
    fn apply_color(&self, text: &str, color_name: &str) -> ColoredString {
        let colored = text.normal();
        match color_name.to_lowercase().as_str() {
            "black" => colored.black(),
            "red" => colored.red(),
            "green" => colored.green(),
            "yellow" => colored.yellow(),
            "blue" => colored.blue(),
            "magenta" => colored.magenta(),
            "cyan" => colored.cyan(),
            "white" => colored.white(),
            "bright_black" => colored.bright_black(),
            "bright_red" => colored.bright_red(),
            "bright_green" => colored.bright_green(),
            "bright_yellow" => colored.bright_yellow(),
            "bright_blue" => colored.bright_blue(),
            "bright_magenta" => colored.bright_magenta(),
            "bright_cyan" => colored.bright_cyan(),
            "bright_white" => colored.bright_white(),
            _ => colored, // Default: no color
        }
    }
}
