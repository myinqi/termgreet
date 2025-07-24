use anyhow::Result;
use colored::*;
use unicode_width::UnicodeWidthStr;
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
        // Show title if configured and enabled
        if self.config.general.show_title {
            if let Some(ref title) = self.config.general.title {
                println!("{}", self.apply_color(title, &self.config.general.colors.title));
                println!();
            }
        }
        
        // Prepare system info lines
        let info_lines = self.prepare_system_info_lines(system_info);
        
        // Display PNG image if configured and available
        if self.show_images && self.config.display.show_image {
            let mut image_to_use: Option<std::path::PathBuf> = None;
            
            // First, check if a specific image path is configured
            if let Some(ref image_path) = self.config.display.image_path {
                if image_path.exists() {
                    image_to_use = Some(image_path.clone());
                }
            }
            
            // If no specific image or it doesn't exist, try default logo
            if image_to_use.is_none() {
                if let Some(config_dir) = dirs::config_dir() {
                    let default_logo = config_dir.join("termgreet").join("pngs").join("termgreet_logo.png");
                    if default_logo.exists() {
                        image_to_use = Some(default_logo);
                    }
                }
            }
            
            // Display the image if we found one
            if let Some(image_path) = image_to_use {
                // Check layout configuration
                match self.config.display.layout.as_str() {
                    "horizontal" => {
                        self.show_horizontal_layout(&image_path, &info_lines)?;
                    },
                    _ => { // "vertical" or any other value defaults to vertical
                        self.show_image_with_info(&image_path, &info_lines)?;
                    }
                }
                self.show_motd_if_enabled()?;
                return Ok(());
            }
        }
        
        // No image configured or available - show info only
        self.show_info_only(&info_lines);
        self.show_motd_if_enabled()?;
        
        Ok(())
    }

    fn is_kitty_terminal(&self) -> bool {
        // Check if we're in a terminal that supports Kitty graphics protocol
        std::env::var("TERM").unwrap_or_default().contains("kitty") ||
        std::env::var("TERM_PROGRAM").unwrap_or_default().contains("ghostty") ||
        std::env::var("TERM_PROGRAM").unwrap_or_default().contains("iTerm")
    }
    
    fn render_image_as_text_blocks(&self, image_path: &std::path::PathBuf) -> Result<Vec<String>> {
        // Calculate effective dimensions with cell size adjustments (same as render_image_to_terminal)
        let width = self.config.display.image_size.width;
        let height = self.config.display.image_size.height;
        let cell_width = self.config.display.image_size.cell_width;
        let cell_height = self.config.display.image_size.cell_height;
        
        let effective_width = if cell_width > 20 { width / 2 } else { width };
        let effective_height = if cell_height < 15 { height / 2 } else { height };
        
        // Get block rendering configuration
        let block_config = &self.config.display.block_rendering;
        let mut output_lines = Vec::new();
        
        // Try to load and process the image
        match image::open(image_path) {
            Ok(img) => {
                // Resize the image to fit our dimensions
                let resized = img.resize_exact(
                    effective_width * 2, // Each character represents 2 pixels horizontally
                    effective_height,     // Each character represents 1 pixel vertically
                    image::imageops::FilterType::Lanczos3
                );
                
                // Convert to RGB for easier processing
                let rgb_img = resized.to_rgb8();
                let (img_width, img_height) = rgb_img.dimensions();
                
                // Convert image to colored block characters
                for y in 0..effective_height as u32 {
                    let mut line = String::new();
                    
                    for x in 0..effective_width as u32 {
                        // Sample pixels based on sampling method
                        let (avg_r, avg_g, avg_b, brightness) = self.sample_pixels(
                            &rgb_img, x, y, img_width, img_height, block_config
                        );
                        
                        // Apply brightness boost and contrast
                        let adjusted_brightness = self.adjust_brightness_contrast(
                            brightness, block_config.brightness_boost, block_config.contrast
                        );
                        
                        // Choose appropriate block character based on brightness and style
                        let block_char = self.select_block_character(adjusted_brightness, block_config);
                        
                        // Apply color based on color mode
                        let colored_char = self.apply_color_mode(
                            &block_char, avg_r, avg_g, avg_b, block_config
                        );
                        
                        line.push_str(&colored_char);
                    }
                    
                    // Reset color at end of line if needed
                    if block_config.color_mode != "monochrome" && !line.is_empty() {
                        line.push_str("\x1b[0m");
                    }
                    output_lines.push(line);
                }
            },
            Err(_) => {
                // If image loading fails, create placeholder lines
                for _ in 0..effective_height as usize {
                    output_lines.push(" ".repeat(effective_width as usize));
                }
            }
        }
        
        // Ensure we have the right number of lines
        while output_lines.len() < effective_height as usize {
            output_lines.push(" ".repeat(effective_width as usize));
        }
        output_lines.truncate(effective_height as usize);
        
        Ok(output_lines)
    }
    
    fn get_visible_width(&self, text: &str) -> usize {
        // Calculate visible width of text, ignoring ANSI escape codes
        let mut width = 0;
        let mut in_escape = false;
        
        for ch in text.chars() {
            if ch == '\x1b' {
                in_escape = true;
            } else if in_escape && ch == 'm' {
                in_escape = false;
            } else if !in_escape {
                width += 1;
            }
        }
        
        width
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
        // For consistency, use the same rendering approach as horizontal layout
        // but display image above info instead of side-by-side
        
        // Try Kitty Graphics Protocol if preferred and supported
        if self.config.display.prefer_kitty_graphics && self.is_kitty_terminal() {
            match self.render_image_to_terminal(image_path) {
                Ok(_) => {
                    println!(); // Add spacing after image
                    
                    // Show system info below the image
                    for line in info_lines.iter() {
                        println!("{}", line);
                    }
                }
                Err(_) => {
                    // Failed to render Kitty graphics, fall back to block rendering
                    eprintln!("[Warning] Kitty Graphics failed, falling back to block rendering");
                    self.render_vertical_with_blocks(image_path, info_lines)?;
                }
            }
        } else {
            // Use consistent block-based rendering (same as horizontal layout)
            self.render_vertical_with_blocks(image_path, info_lines)?;
        }
        
        // Add padding
        for _ in 0..self.config.display.padding {
            println!();
        }
        
        Ok(())
    }

    fn show_horizontal_layout(&self, image_path: &std::path::PathBuf, info_lines: &[String]) -> Result<()> {
        // Get terminal dimensions
        let _term_width = 120; // Conservative estimate for wide terminals
        let image_width = self.config.display.image_size.width as usize;
        let padding = self.config.display.padding as usize;
        
        // For Kitty Graphics Protocol, we need to implement true side-by-side layout
        if self.config.display.prefer_kitty_graphics && self.is_kitty_terminal() {
            // Calculate dimensions
            let image_height = self.config.display.image_size.height as usize;
            let info_start_col = image_width + padding;
            let total_content_height = std::cmp::max(image_height, info_lines.len());
            
            // Strategy: Reserve space for all content first, then render image and modules
            // This ensures the terminal has enough space allocated
            
            // Print empty lines to reserve space for all content
            for _ in 0..total_content_height {
                println!();
            }
            
            // Move cursor back to the beginning of our reserved space
            print!("\x1b[{}A", total_content_height);
            
            // Render the image with Kitty Graphics Protocol at current position
            self.render_image_to_terminal(image_path)?;
            
            // Move cursor back up to align with top of image for module output
            print!("\x1b[{}A", image_height);
            print!("\x1b[{}C", info_start_col); // Move cursor right to info column
            
            // Print each info line with proper cursor positioning
            for (i, line) in info_lines.iter().enumerate() {
                if i > 0 {
                    // Move to next line and position cursor at info column
                    print!("\x1b[1B"); // Move cursor down one line
                    print!("\x1b[{}G", info_start_col + 1); // Move cursor to specific column (1-indexed)
                }
                print!("{}", line);
            }
            
            // Move cursor to the end of our reserved space
            let remaining_lines = if total_content_height > info_lines.len() {
                total_content_height - info_lines.len()
            } else {
                0
            };
            
            if remaining_lines > 0 {
                print!("\x1b[{}B", remaining_lines);
            }
            print!("\x1b[1G"); // Move cursor to beginning of line
            println!(); // Add blank line before MOTD
            
        } else {
            // Use block-based rendering for true side-by-side layout
            return self.show_horizontal_layout_with_blocks(image_path, info_lines);
        }
        
        Ok(())
    }
    
    fn show_horizontal_layout_with_blocks(&self, image_path: &std::path::Path, info_lines: &[String]) -> Result<()> {
        // Use block-based rendering for true side-by-side layout
        // This works in all terminals, including Kitty/Ghostty
        let image_lines = self.render_image_as_text_blocks(&image_path.to_path_buf())?;
        
        // Calculate layout dimensions
        let image_width = self.config.display.image_size.width as usize;
        let padding = self.config.display.padding;
        
        // Always show ALL info lines, even if they exceed image height
        let max_lines = std::cmp::max(image_lines.len(), info_lines.len());
        
        // Print side-by-side layout
        for i in 0..max_lines {
            let mut line_output = String::new();
            
            // Add image part (left side)
            if i < image_lines.len() {
                line_output.push_str(&image_lines[i]);
                // Pad to ensure consistent spacing
                let visible_width = self.get_visible_width(&image_lines[i]);
                if visible_width < image_width {
                    line_output.push_str(&" ".repeat(image_width - visible_width));
                }
            } else {
                // Empty space where image would be
                line_output.push_str(&" ".repeat(image_width));
            }
            
            // Add padding between image and info
            line_output.push_str(&" ".repeat(padding as usize));
            
            // Add info part (right side)
            if i < info_lines.len() {
                line_output.push_str(&info_lines[i]);
            }
            
            println!("{}", line_output);
        }
        
        println!(); // Add blank line before MOTD
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
    
    fn get_module_display_name(&self, module_key: &str, default_name: &str) -> String {
        let display_names = &self.config.modules.display_names;
        let custom_name = match module_key {
            "user_at_host" => &display_names.user_at_host,
            "os" => &display_names.os,
            "kernel" => &display_names.kernel,
            "linux" => &display_names.linux,
            "uptime" => &display_names.uptime,
            "os_age" => &display_names.os_age,
            "packages" | "packages_combined" => &display_names.packages,
            "shell" => &display_names.shell,
            "resolution" => &display_names.resolution,
            "network" => &display_names.network,
            "public_ip" => &display_names.public_ip,
            "de" => &display_names.de,
            "wm" => &display_names.wm,
            "theme" => &display_names.theme,
            "icons" => &display_names.icons,
            "terminal" => &display_names.terminal,
            "font" => &display_names.font,
            "user" => &display_names.user,
            "hostname" => &display_names.hostname,
            "cpu" => &display_names.cpu,
            "cpu_temp" => &display_names.cpu_temp,
            "gpu" => &display_names.gpu,
            "gpu_temp" => &display_names.gpu_temp,
            "temp_combined" => &display_names.temp_combined,
            "gpu_driver" => &display_names.gpu_driver,
            "memory" => &display_names.memory,
            "disk" => &display_names.disk,
            "dysk" => &display_names.dysk,
            "battery" => &display_names.battery,
            "locale" => &display_names.locale,
            _ => &None,
        };
        
        custom_name.as_ref().unwrap_or(&default_name.to_string()).clone()
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
            ("user_at_host", "Login", modules.user_at_host),
            ("user", "User", modules.user),
            ("hostname", "Hostname", modules.hostname),            
            ("os", "OS", modules.os),
            ("kernel", "Kernel", modules.kernel),
            ("linux", "Linux", modules.linux),
            ("uptime", "Uptime", modules.uptime),
            ("os_age", "OS Age", modules.os_age),
            ("packages", "Packages", modules.packages),
            ("flatpak_packages", "Flatpak", modules.flatpak_packages),
            ("packages_combined", "Packages", modules.packages_combined),
            ("shell", "Shell", modules.shell),
            ("terminal", "Terminal", modules.terminal),            
            ("resolution", "Resolution", modules.resolution),
            ("de", "DE", modules.de),
            ("wm", "WM", modules.wm),
            ("theme", "Theme", modules.theme),
            ("icons", "Icons", modules.icons),
            ("font", "Font", modules.font),
            ("locale", "Locale", modules.locale),            
            ("cpu", "CPU", modules.cpu),
            ("cpu_temp", "CPU Temp", modules.cpu_temp),
            ("gpu", "GPU", modules.gpu),
            ("gpu_temp", "GPU Temp", modules.gpu_temp),
            ("gpu_driver", "GPU Driver", modules.gpu_driver),
            ("temp_combined", "Temperatures", modules.temp_combined),            
            ("memory", "Memory", modules.memory),
            ("battery", "Battery", modules.battery),
            ("network", "Network", modules.network),
            ("public_ip", "Public IP", modules.public_ip),
            ("disk", "Disk", modules.disk),            
            ("dysk", "Drives", modules.dysk),
        ];
        
        // Calculate maximum module name width for alignment if enabled
        let max_name_width = if separator_config.align_separator {
            module_order
                .iter()
                .filter(|(_, _, enabled)| *enabled)
                .map(|(key, default_name, _)| {
                    let display_name = self.get_module_display_name(key, default_name);
                    // Use standard unicode width for Nerd Font icons (consistent width)
                    display_name.width()
                })
                .max()
                .unwrap_or(0)
        } else {
            0
        };

        for (key, default_name, enabled) in module_order {
            if enabled {
                let lookup_key = key.to_uppercase();
                if let Some(value) = system_info.data.get(&lookup_key) {
                    let trimmed_value = value.trim();
                    // Only add non-empty, non-Unknown values
                    if !trimmed_value.is_empty() && trimmed_value != "Unknown" {
                        // Get custom display name (with potential icon)
                        let display_name = self.get_module_display_name(key, default_name);
                        
                        // Pad module name for alignment if enabled
                        let padded_name = if separator_config.align_separator {
                            // Calculate visual width and pad accordingly
                            let visual_width = display_name.width();
                            let padding_needed = max_name_width.saturating_sub(visual_width);
                            format!("{}{}", display_name, " ".repeat(padding_needed))
                        } else {
                            display_name
                        };
                        
                        // Handle multi-line modules (like dysk)
                        let value_lines: Vec<&str> = trimmed_value.lines().collect();
                        
                        if value_lines.len() > 1 {
                            // Multi-line module: first line with module name, subsequent lines indented
                            for (i, value_line) in value_lines.iter().enumerate() {
                                if i == 0 {
                                    // First line with module name
                                    let line = format!(
                                        "{}{}{}",
                                        self.apply_color(&padded_name, &colors.module),
                                        self.apply_color(&separator, &colors.separator),
                                        self.apply_color(value_line, &colors.info)
                                    );
                                    lines.push(line);
                                } else {
                                    // Subsequent lines: indent to align with the value column
                                    let indent_width = padded_name.width() + separator.width();
                                    let indented_line = format!(
                                        "{}{}",
                                        " ".repeat(indent_width),
                                        self.apply_color(value_line, &colors.info)
                                    );
                                    lines.push(indented_line);
                                }
                            }
                        } else {
                            // Single-line module
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
    
    fn render_vertical_with_blocks(&self, image_path: &std::path::Path, info_lines: &[String]) -> Result<()> {
        // Use the same block rendering as horizontal layout for consistency
        let image_path_buf = image_path.to_path_buf();
        let image_lines = self.render_image_as_text_blocks(&image_path_buf)?;
        
        // Print image above info (vertical layout)
        for line in image_lines {
            println!("{}", line);
        }
        
        println!(); // Add spacing between image and info
        
        // Print system info below the image
        for line in info_lines {
            println!("{}", line);
        }
        
        Ok(())
    }
    
    // Helper methods for configurable block rendering
    
    fn sample_pixels(
        &self,
        rgb_img: &image::RgbImage,
        x: u32,
        y: u32,
        img_width: u32,
        img_height: u32,
        block_config: &crate::config::BlockRenderingConfig,
    ) -> (u16, u16, u16, f32) {
        let px1_x = (x * 2).min(img_width - 1);
        let px2_x = (x * 2 + 1).min(img_width - 1);
        let py = y.min(img_height - 1);
        
        let pixel1 = rgb_img.get_pixel(px1_x, py);
        let pixel2 = rgb_img.get_pixel(px2_x, py);
        
        let (avg_r, avg_g, avg_b) = match block_config.sampling_method.as_str() {
            "dominant" => {
                // Use the brighter pixel as dominant
                let brightness1 = (pixel1[0] as f32 * 0.299 + pixel1[1] as f32 * 0.587 + pixel1[2] as f32 * 0.114) / 255.0;
                let brightness2 = (pixel2[0] as f32 * 0.299 + pixel2[1] as f32 * 0.587 + pixel2[2] as f32 * 0.114) / 255.0;
                
                if brightness1 > brightness2 {
                    (pixel1[0] as u16, pixel1[1] as u16, pixel1[2] as u16)
                } else {
                    (pixel2[0] as u16, pixel2[1] as u16, pixel2[2] as u16)
                }
            },
            "weighted" => {
                // Weight by distance from center
                let weight1 = 0.6; // Left pixel gets more weight
                let weight2 = 0.4; // Right pixel gets less weight
                
                let r = (pixel1[0] as f32 * weight1 + pixel2[0] as f32 * weight2) as u16;
                let g = (pixel1[1] as f32 * weight1 + pixel2[1] as f32 * weight2) as u16;
                let b = (pixel1[2] as f32 * weight1 + pixel2[2] as f32 * weight2) as u16;
                
                (r, g, b)
            },
            _ => {
                // Default: "average"
                let r = (pixel1[0] as u16 + pixel2[0] as u16) / 2;
                let g = (pixel1[1] as u16 + pixel2[1] as u16) / 2;
                let b = (pixel1[2] as u16 + pixel2[2] as u16) / 2;
                
                (r, g, b)
            }
        };
        
        // Calculate brightness
        let brightness = (avg_r as f32 * 0.299 + avg_g as f32 * 0.587 + avg_b as f32 * 0.114) / 255.0;
        
        (avg_r, avg_g, avg_b, brightness)
    }
    
    fn adjust_brightness_contrast(&self, brightness: f32, brightness_boost: f32, contrast: f32) -> f32 {
        // Apply brightness boost
        let boosted = (brightness + brightness_boost).clamp(0.0, 1.0);
        
        // Apply contrast adjustment
        let contrasted = ((boosted - 0.5) * contrast + 0.5).clamp(0.0, 1.0);
        
        contrasted
    }
    
    fn select_block_character(&self, brightness: f32, block_config: &crate::config::BlockRenderingConfig) -> String {
        let blocks = match block_config.block_style.as_str() {
            "ascii" => vec!["#", "*", ":", ".", " "],
            "braille" => vec!["☣", "☢", "☖", "☄", " "],
            "custom" => {
                if block_config.custom_blocks.is_empty() {
                    vec!["█", "▓", "▒", "░", " "]
                } else {
                    block_config.custom_blocks.iter().map(|s| s.as_str()).collect()
                }
            },
            _ => vec!["█", "▓", "▒", "░", " "], // Default
        };
        
        // Use brightness thresholds to select block character
        let thresholds = &block_config.brightness_thresholds;
        
        for (i, &threshold) in thresholds.iter().enumerate() {
            if brightness > threshold {
                return blocks.get(i).unwrap_or(&" ").to_string();
            }
        }
        
        // Return the last (darkest) block character
        blocks.last().unwrap_or(&" ").to_string()
    }
    
    fn apply_color_mode(
        &self,
        block_char: &str,
        r: u16,
        g: u16,
        b: u16,
        block_config: &crate::config::BlockRenderingConfig,
    ) -> String {
        if block_char == " " {
            return block_char.to_string();
        }
        
        match block_config.color_mode.as_str() {
            "monochrome" => block_char.to_string(),
            "16color" => {
                // Convert to nearest 16-color ANSI code
                let ansi_code = self.rgb_to_ansi16(r as u8, g as u8, b as u8);
                format!("\x1b[{}m{}", ansi_code, block_char)
            },
            "256color" => {
                // Convert to 256-color ANSI code
                let ansi_code = self.rgb_to_ansi256(r as u8, g as u8, b as u8);
                format!("\x1b[38;5;{}m{}", ansi_code, block_char)
            },
            _ => {
                // Default: "truecolor"
                format!("\x1b[38;2;{};{};{}m{}", r, g, b, block_char)
            }
        }
    }
    
    fn rgb_to_ansi16(&self, r: u8, g: u8, b: u8) -> u8 {
        // Simple conversion to 16-color ANSI
        let r_bright = r > 127;
        let g_bright = g > 127;
        let b_bright = b > 127;
        
        let mut code = 30; // Base foreground color code
        
        if r_bright { code += 1; }
        if g_bright { code += 2; }
        if b_bright { code += 4; }
        
        // If it's a bright color, add 60 to get bright variants
        if r > 191 || g > 191 || b > 191 {
            code += 60;
        }
        
        code
    }
    
    fn rgb_to_ansi256(&self, r: u8, g: u8, b: u8) -> u8 {
        // Convert RGB to 256-color palette
        // This is a simplified conversion
        if r == g && g == b {
            // Grayscale
            if r < 8 {
                16
            } else if r > 248 {
                231
            } else {
                (((r as u16 - 8) * 24) / 240) as u8 + 232
            }
        } else {
            // Color cube: 16 + 36*r + 6*g + b
            let r_idx = (r as u16 * 5 / 255) as u8;
            let g_idx = (g as u16 * 5 / 255) as u8;
            let b_idx = (b as u16 * 5 / 255) as u8;
            
            16 + 36 * r_idx + 6 * g_idx + b_idx
        }
    }
}
