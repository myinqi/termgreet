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
        
        // Display PNG image if configured and available
        if self.show_images && self.config.display.show_image {
            if let Some(ref image_path) = self.config.display.image_path {
                if image_path.exists() {
                    self.show_image_with_info(image_path, &info_lines)?;
                    self.show_motd_if_enabled()?;
                    return Ok(());
                }
            }
        }
        
        // No image configured or available - show info only
        self.show_info_only(&info_lines);
        self.show_motd_if_enabled()?;
        
        Ok(())
    }

    fn show_motd_if_enabled(&self) -> Result<()> {
        if !self.config.show_motd {
            return Ok(());
        }

        match MotdConfig::load(&self.config.motd_file) {
            Ok(motd_config) => {
                if motd_config.enabled && !motd_config.messages.is_empty() {
                    Self::show_motd(&motd_config);
                }
            }
            Err(_) => {
                // Silently ignore MOTD loading errors to avoid disrupting main display
            }
        }
        
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
            
            // Create a seed based on current time for randomness
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let mut hasher = DefaultHasher::new();
            now.hash(&mut hasher);
            
            let index = (hasher.finish() as usize) % motd_config.messages.len();
            &motd_config.messages[index]
        } else {
            &motd_config.messages[0]
        };

        // Create a temporary display instance for color formatting
        let temp_display = Display {
            config: Config::default(),
            show_images: false,
            kitty_graphics: KittyGraphics::new(),
        };
        
        println!("{}", temp_display.apply_color(message, &motd_config.color));
    }

    
    fn render_image_to_terminal(&self, image_path: &std::path::Path) -> Result<()> {
        let width = self.config.display.image_size.width;
        let height = self.config.display.image_size.height;
        
        // Try Kitty Graphics Protocol if preferred and supported
        if self.config.display.prefer_kitty_graphics && self.kitty_graphics.supports_kitty {
            let cell_width = self.config.display.image_size.cell_width;
            let cell_height = self.config.display.image_size.cell_height;
            match self.kitty_graphics.render_image(image_path, width, height, cell_width, cell_height) {
                Ok(_) => {
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("[Warning] Kitty Graphics failed: {}, falling back to viuer", e);
                }
            }
        }
        
        // Fallback to viuer for terminals that don't support Kitty Graphics Protocol
        // Apply cell dimensions scaling for consistent behavior
        let cell_width = self.config.display.image_size.cell_width;
        let cell_height = self.config.display.image_size.cell_height;
        
        // Calculate effective dimensions based on cell size (similar to Kitty Graphics logic)
        let effective_width = if cell_width > 20 { width / 2 } else { width };
        let effective_height = if cell_height < 15 { height / 2 } else { height };
        
        let viuer_config = ViuerConfig {
            transparent: true,
            absolute_offset: false,
            width: Some(effective_width),
            height: Some(effective_height),
            use_kitty: true,  // viuer's own kitty support (different from our implementation)
            use_iterm: true,  // Enable iTerm2 graphics protocol
            truecolor: true,  // Use 24-bit colors for better color accuracy
            ..Default::default()
        };

        print_from_file(image_path, &viuer_config)
            .map(|_| ()) // Ignore the returned dimensions
            .map_err(|e| anyhow::anyhow!("Failed to display image: {}", e))
    }
    
    fn show_image_with_info(&self, image_path: &std::path::Path, info_lines: &[String]) -> Result<()> {
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
                // Failed to render image, show info only
                eprintln!("[Warning] Image rendering failed, showing system info only");
                self.show_info_only(info_lines);
            }
        }
        
        // Add padding
        for _ in 0..self.config.display.padding {
            println!();
        }
        
        Ok(())
    }
    

    
    fn show_info_only(&self, info_lines: &[String]) {
        // Display system information in a clean single column
        for line in info_lines.iter() {
            println!("{}", line);
        }
        
        // Add padding
        for _ in 0..self.config.display.padding {
            println!();
        }
    }
    
    fn prepare_system_info_lines(&self, system_info: &SystemInfo) -> Vec<String> {
        let modules = &self.config.modules;
        let colors = &self.config.general.colors;
        let separator_config = &self.config.general.separator;
        let mut lines = Vec::new();
        
        // Build the complete separator string with configurable spacing
        let separator = format!(
            "{}{}{}",
            " ".repeat(separator_config.space_before as usize),
            separator_config.symbol,
            " ".repeat(separator_config.space_after as usize)
        );

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
            ("font", "Font", modules.font),
            ("user", "User", modules.user),
            ("hostname", "Hostname", modules.hostname),
            ("cpu", "CPU", modules.cpu),
            ("gpu", "GPU", modules.gpu),
            ("memory", "Memory", modules.memory),
            ("disk", "Disk", modules.disk),
            ("battery", "Battery", modules.battery),
            ("locale", "Locale", modules.locale),
        ];
        
        // Calculate maximum module name width for alignment if enabled
        let max_name_width = if separator_config.align_separator {
            module_order
                .iter()
                .filter(|(_, _, enabled)| *enabled)
                .map(|(_, display_name, _)| display_name.len())
                .max()
                .unwrap_or(0)
        } else {
            0
        };

        for (key, display_name, enabled) in module_order {
            if enabled {
                let lookup_key = key.to_uppercase();
                if let Some(value) = system_info.data.get(&lookup_key) {
                    let trimmed_value = value.trim();
                    // Only add non-empty, non-Unknown values
                    if !trimmed_value.is_empty() && trimmed_value != "Unknown" {
                        // Pad module name for alignment if enabled
                        let padded_name = if separator_config.align_separator {
                            format!("{:<width$}", display_name, width = max_name_width)
                        } else {
                            display_name.to_string()
                        };
                        
                        let line = format!(
                            "{}{}{}",
                            self.apply_color(&padded_name, &colors.module),
                            self.apply_color(&separator, &colors.separator),
                            self.apply_color(trimmed_value, &colors.info)
                        );
                        lines.push(line);
                    }
                }
            }
        }
        
        lines
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
