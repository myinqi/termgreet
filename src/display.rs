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
        // Show image or ASCII art first
        if self.show_images && self.config.display.show_image {
            self.show_image()?;
        } else if let Some(ref ascii_art) = self.config.display.ascii_art {
            self.show_ascii_art(ascii_art);
        }

        // Add padding
        for _ in 0..self.config.display.padding {
            println!();
        }

        // Show title if configured
        if let Some(ref title) = self.config.general.title {
            println!("{}", self.apply_color(title, &self.config.general.colors.title));
            println!();
        }

        // Show system information
        self.show_system_info(system_info);

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
        self.show_default_ascii_art();
        Ok(())
    }

    fn show_ascii_art(&self, ascii_art: &str) {
        println!("{}", self.apply_color(ascii_art, &self.config.general.colors.title));
    }

    fn show_default_ascii_art(&self) {
        let ascii_art = r#"
    ╭─────────────────────╮
    │                     │
    │     TermGreet       │
    │                     │
    ╰─────────────────────╯
        "#;
        println!("{}", self.apply_color(ascii_art, &self.config.general.colors.title));
    }

    fn show_system_info(&self, system_info: &SystemInfo) {
        let modules = &self.config.modules;
        let colors = &self.config.general.colors;
        let separator = &self.config.general.separator;

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
                    println!(
                        "{}{}{}",
                        self.apply_color(display_name, &colors.title),
                        self.apply_color(separator, &colors.separator),
                        self.apply_color(value, &colors.info)
                    );
                }
            }
        }
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
