use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::GenericImageView;
use std::env;
use std::io::Write;
use std::path::Path;

/// Kitty Graphics Protocol implementation for pixel-perfect image rendering
pub struct KittyGraphics {
    pub supports_kitty: bool,
    pub in_tmux: bool,
}

impl KittyGraphics {
    pub fn new() -> Self {
        let supports_kitty = Self::detect_kitty_support();
        let in_tmux = env::var("TMUX").is_ok();
        
        Self {
            supports_kitty,
            in_tmux,
        }
    }

    /// Detect if the terminal supports Kitty Graphics Protocol
    fn detect_kitty_support() -> bool {
        // Check for known terminals that support Kitty Graphics Protocol
        if let Ok(term) = env::var("TERM") {
            if term.contains("kitty") {
                return true;
            }
        }
        
        // Check for Ghostty
        if env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
            return true;
        }
        
        // Check for other terminals that might support it
        if let Ok(term_program) = env::var("TERM_PROGRAM") {
            match term_program.as_str() {
                "iTerm.app" => return true,
                "WezTerm" => return true,
                _ => {}
            }
        }
        
        false
    }

    /// Render image using Kitty Graphics Protocol (Direct Mode)
    pub fn render_image_direct(&self, image_path: &Path, width: u32, height: u32) -> Result<()> {
        if !self.supports_kitty {
            return Err(anyhow::anyhow!("Terminal doesn't support Kitty Graphics Protocol"));
        }

        let path_str = image_path.to_string_lossy();
        let base64_path = STANDARD.encode(path_str.as_bytes());

        let mut output = Vec::new();
        
        // Tmux passthrough if needed
        if self.in_tmux {
            output.extend_from_slice(b"\x1bPtmux;\x1b");
        }

        // Kitty Graphics Protocol Direct Mode
        // a=T (transmit), f=100 (format), t=f (file), c=columns, r=rows
        write!(
            &mut output,
            "\x1b_Ga=T,f=100,t=f,c={},r={};{}\x1b\\",
            width, height, base64_path
        )?;

        if self.in_tmux {
            output.extend_from_slice(b"\x1b\\");
        }

        // Add newline for proper spacing
        output.push(b'\n');

        print!("{}", String::from_utf8_lossy(&output));
        std::io::stdout().flush()?;

        Ok(())
    }

    /// Render image using Kitty Graphics Protocol (Standard Mode with pixel data)
    pub fn render_image_standard(&self, image_path: &Path, width: u32, height: u32, cell_width: u32, cell_height: u32) -> Result<()> {
        if !self.supports_kitty {
            return Err(anyhow::anyhow!("Terminal doesn't support Kitty Graphics Protocol"));
        }

        // Load and process image
        let img = image::open(image_path)
            .with_context(|| format!("Failed to open image: {}", image_path.display()))?;

        // Get original image dimensions
        let (orig_width, orig_height) = img.dimensions();
        let aspect_ratio = orig_width as f32 / orig_height as f32;
        
        // Calculate target pixel dimensions while preserving aspect ratio
        // Use configurable cell dimensions from user config
        
        let target_pixel_width = width * cell_width;
        let target_pixel_height = height * cell_height;
        let target_aspect_ratio = target_pixel_width as f32 / target_pixel_height as f32;
        
        // Adjust dimensions to preserve original aspect ratio
        let (final_width, final_height) = if aspect_ratio > target_aspect_ratio {
            // Image is wider than target - fit to width
            (target_pixel_width, (target_pixel_width as f32 / aspect_ratio) as u32)
        } else {
            // Image is taller than target - fit to height  
            ((target_pixel_height as f32 * aspect_ratio) as u32, target_pixel_height)
        };

        // Resize image preserving aspect ratio
        let resized = img.resize(
            final_width,
            final_height,
            image::imageops::FilterType::Lanczos3,
        );

        // Convert to RGBA format
        let rgba_img = resized.to_rgba8();
        let raw_data = rgba_img.as_raw();

        // Encode as base64
        let base64_data = STANDARD.encode(raw_data);

        // Send in chunks (Kitty protocol supports up to 4096 bytes per chunk)
        const CHUNK_SIZE: usize = 4096;
        let chunks: Vec<&str> = base64_data
            .as_bytes()
            .chunks(CHUNK_SIZE)
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect();

        let mut output = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            if self.in_tmux {
                output.extend_from_slice(b"\x1bPtmux;\x1b");
            }

            if i == 0 {
                // First chunk: include image parameters
                write!(
                    &mut output,
                    "\x1b_Ga=T,f=32,s={},v={},c={},r={}",
                    final_width, final_height, width, height
                )?;
                
                if chunks.len() > 1 {
                    output.extend_from_slice(b",m=1"); // More chunks follow
                }
                
                write!(&mut output, ";{}\x1b\\", chunk)?;
            } else {
                // Continuation chunks
                output.extend_from_slice(b"\x1b_G");
                if i < chunks.len() - 1 {
                    output.extend_from_slice(b"m=1"); // More chunks follow
                } else {
                    output.extend_from_slice(b"m=0"); // Last chunk
                }
                write!(&mut output, ";{}\x1b\\", chunk)?;
            }

            if self.in_tmux {
                output.extend_from_slice(b"\x1b\\");
            }
        }

        // Add newline for proper spacing
        output.push(b'\n');

        print!("{}", String::from_utf8_lossy(&output));
        std::io::stdout().flush()?;

        Ok(())
    }

    /// Try to render image with best available method
    pub fn render_image(&self, image_path: &Path, width: u32, height: u32, cell_width: u32, cell_height: u32) -> Result<()> {
        if !self.supports_kitty {
            return Err(anyhow::anyhow!("Terminal doesn't support Kitty Graphics Protocol"));
        }

        // Try Direct Mode first (faster and more efficient)
        if let Ok(()) = self.render_image_direct(image_path, width, height) {
            return Ok(());
        }

        // Fallback to Standard Mode
        self.render_image_standard(image_path, width, height, cell_width, cell_height)
    }


}

impl Default for KittyGraphics {
    fn default() -> Self {
        Self::new()
    }
}
