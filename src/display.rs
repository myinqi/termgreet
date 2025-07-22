use anyhow::Result;
use colored::*;
use viuer::{Config as ViuerConfig, print_from_file};

use crate::config::{Config, MotdConfig};
use crate::system_info::SystemInfo;
use crate::kitty_graphics::KittyGraphics;

pub struct Display {
    config: Config,
    show_images: bool,
    kitty_graphics: KittyGraphics,
}

impl Display {
    pub fn new(config: Config, show_images: bool) -> Self {
        Self {
            config,
            show_images,
            kitty_graphics: KittyGraphics::new(),
        }
    }

    pub fn show(&self, system_info: &SystemInfo) -> Result<()> {
        // Show title if configured
        if let Some(ref title) = self.config.general.title {
            println!("{}", self.apply_color(title, &self.config.general.colors.title));
            println!();
        }
        
        // Prepare system info lines
        let info_lines = self.prepare_system_info_lines(system_info);
        
        // Check if we should display a PNG image with special layout
        if self.show_images && self.config.display.show_image {
            if let Some(ref image_path) = self.config.display.image_path {
                if image_path.exists() {
                    // Use special image layout that renders image and info side by side
                    self.show_image_with_side_info(image_path, &info_lines)?;
                    return Ok(());
                }
            }
        }
        
        // Use ASCII art with standard two-column layout
        let art_lines = if let Some(ref ascii_art) = self.config.display.ascii_art {
            ascii_art.lines().map(|s| s.to_string()).collect()
        } else {
            self.get_default_ascii_lines()
        };
        
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
            kitty_graphics: KittyGraphics::new(),
        };
        println!("{}", temp_display.apply_color(message, &motd_config.color));
    }




    
    fn render_image_to_terminal(&self, image_path: &std::path::Path) -> Result<()> {
        let width = self.config.display.image_size.width;
        let height = self.config.display.image_size.height;
        
        // Try Kitty Graphics Protocol first for pixel-perfect rendering
        if self.kitty_graphics.supports_kitty {
            match self.kitty_graphics.render_image(image_path, width, height) {
                Ok(_) => {
                    println!("[Kitty Graphics Protocol] Rendered with pixel-perfect quality");
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("[Warning] Kitty Graphics failed: {}, falling back to viuer", e);
                }
            }
        }
        
        // Fallback to viuer for terminals that don't support Kitty Graphics Protocol
        let viuer_config = ViuerConfig {
            transparent: true,
            absolute_offset: false,
            width: Some(width),
            height: Some(height),
            use_kitty: true,  // viuer's own kitty support (different from our implementation)
            use_iterm: true,  // Enable iTerm2 graphics protocol
            truecolor: true,  // Use 24-bit colors for better color accuracy
            ..Default::default()
        };

        print_from_file(image_path, &viuer_config)
            .map(|_| ()) // Ignore the returned dimensions
            .map_err(|e| anyhow::anyhow!("Failed to display image: {}", e))
    }
    
    fn show_image_with_side_info(&self, image_path: &std::path::Path, info_lines: &[String]) -> Result<()> {
        // This is the tricky part: we need to render the image and position text beside it
        // Terminal image rendering makes this challenging, but we'll try a hybrid approach
        
        let _image_height = self.config.display.image_size.height as usize;
        let _image_width = self.config.display.image_size.width as usize;
        
        // First, try to render the image
        match self.render_image_to_terminal(image_path) {
            Ok(_) => {
                // Image rendered successfully
                // Now we need to position the cursor back up to add text beside it
                // This is a limitation of terminal graphics - we'll show info below for now
                // but with better formatting
                
                println!(); // Add spacing after image
                
                // Show system info in a clean single column below the image
                for line in info_lines.iter() {
                    println!("{}", line);
                }
            }
            Err(_) => {
                // Failed to render image, fall back to ASCII art layout
                let art_lines = if let Some(ref ascii_art) = self.config.display.ascii_art {
                    ascii_art.lines().map(|s| s.to_string()).collect()
                } else {
                    self.get_default_ascii_lines()
                };
                self.show_two_column_layout(&art_lines, info_lines);
            }
        }
        
        // Add padding
        for _ in 0..self.config.display.padding {
            println!();
        }
        
        Ok(())
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
            ("os_age", "OS Age", modules.os_age),
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
                let lookup_key = key.to_uppercase();
                if let Some(value) = system_info.data.get(&lookup_key) {
    let trimmed_value = value.trim();
    // Only add non-empty, non-Unknown values
    if !trimmed_value.is_empty() && trimmed_value != "Unknown" {
        let line = format!(
            "{}{}{}",
            self.apply_color(display_name, &colors.title),
            self.apply_color(separator, &colors.separator),
            self.apply_color(trimmed_value, &colors.info)
        );
        lines.push(line);
    }
}
            }
        }
        
        lines
    }
    
    fn show_two_column_layout(&self, art_lines: &[String], info_lines: &[String]) {
        let max_lines = art_lines.len().max(info_lines.len());
        let art_width = 35; // Fixed width for ASCII art column
        
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
            
            // Only print line if at least one column has content
            if !art_line.is_empty() || !info_line.is_empty() {
                println!("{}{}{}", art_line, padding, info_line);
            }
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
