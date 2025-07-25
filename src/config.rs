use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub display: DisplayConfig,
    pub modules: ModulesConfig,
    pub show_motd: bool,
    pub motd_file: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeneralConfig {
    pub show_title: bool,
    pub title: Option<String>,
    pub separator: SeparatorConfig,
    pub colors: ColorsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SeparatorConfig {
    pub symbol: String,
    pub space_before: u8,
    pub space_after: u8,
    pub align_separator: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ColorsConfig {
    pub title: String,
    pub module: String,
    pub info: String,
    pub separator: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DisplayConfig {
    pub show_image: bool,
    pub image_path: Option<PathBuf>,
    pub image_size: ImageSize,
    pub prefer_kitty_graphics: bool,
    pub padding: u8,
    pub layout: String,
    pub show_border: bool,
    pub border_top: String,
    pub border_bottom: String,
    pub border_color: String,
    pub block_rendering: BlockRenderingConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockRenderingConfig {
    pub block_style: String,           // "default", "ascii", "braille", "custom"
    pub custom_blocks: Vec<String>,    // Custom block characters (used when block_style = "custom")
    pub brightness_thresholds: Vec<f32>, // Brightness thresholds for block selection (0.0-1.0)
    pub color_mode: String,            // "truecolor", "256color", "16color", "monochrome"
    pub contrast: f32,                 // Contrast adjustment (0.5-2.0)
    pub brightness_boost: f32,         // Brightness boost (-0.5 to +0.5)
    pub sampling_method: String,       // "average", "dominant", "weighted"
    pub enable_dithering: bool,        // Enable dithering for better quality
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
    pub cell_width: u32,  // Pixel pro Terminal-Zeichen (Breite)
    pub cell_height: u32, // Pixel pro Terminal-Zeichen (Höhe)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModulesConfig {
    pub show_versions: bool,
    pub os: bool,
    pub kernel: bool,
    pub linux: bool,
    pub uptime: bool,
    pub os_age: bool,
    pub packages: bool,
    pub flatpak_packages: bool,
    pub packages_combined: bool,
    pub shell: bool,
    pub resolution: bool,
    pub network: bool,
    pub public_ip: bool,
    pub de: bool,
    pub wm: bool,
    pub theme: bool,
    pub icons: bool,
    pub terminal: bool,
    pub terminal_shell_combined: bool,
    pub font: bool,
    pub user: bool,
    pub hostname: bool,
    pub user_at_host: bool,
    pub cpu: bool,
    pub cpu_temp: bool,
    pub gpu: bool,
    pub gpu_temp: bool,
    pub temp_combined: bool,
    pub gpu_driver: bool,
    pub memory: bool,
    pub disk: bool,
    pub dysk: bool,
    pub battery: bool,
    pub locale: bool,
    pub display_names: ModuleDisplayConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModuleDisplayConfig {
    pub user_at_host: Option<String>,
    pub os: Option<String>,
    pub kernel: Option<String>,
    pub linux: Option<String>,
    pub uptime: Option<String>,
    pub os_age: Option<String>,
    pub packages: Option<String>,
    pub shell: Option<String>,
    pub resolution: Option<String>,
    pub network: Option<String>,
    pub public_ip: Option<String>,
    pub de: Option<String>,
    pub wm: Option<String>,
    pub theme: Option<String>,
    pub icons: Option<String>,
    pub terminal: Option<String>,
    pub terminal_shell_combined: Option<String>,
    pub font: Option<String>,
    pub user: Option<String>,
    pub hostname: Option<String>,
    pub cpu: Option<String>,
    pub cpu_temp: Option<String>,
    pub gpu: Option<String>,
    pub gpu_temp: Option<String>,
    pub temp_combined: Option<String>,
    pub gpu_driver: Option<String>,
    pub memory: Option<String>,
    pub disk: Option<String>,
    pub dysk: Option<String>,
    pub battery: Option<String>,
    pub locale: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MotdConfig {
    pub enabled: bool,
    pub messages: Vec<String>,
    pub random: bool,
    pub color: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                show_title: true,
                title: Some("System Information".to_string()),
                separator: SeparatorConfig {
                    symbol: "->".to_string(),
                    space_before: 1,
                    space_after: 1,
                    align_separator: false,
                },
                colors: ColorsConfig {
                    title: "bright_cyan".to_string(),
                    module: "bright_cyan".to_string(),
                    info: "bright_white".to_string(),
                    separator: "bright_blue".to_string(),
                },
            },
            display: DisplayConfig {
                show_image: true,
                image_path: None,
                image_size: ImageSize {
                    width: 40,
                    height: 20,
                    cell_width: 10,  // Standard-Schätzung für moderne Terminals
                    cell_height: 20, // Kann per config angepasst werden
                },
                prefer_kitty_graphics: true,
                padding: 2,
                layout: "horizontal".to_string(),
                show_border: false,
                border_top: "┌──────────────────────────────────────────────┐".to_string(),
                border_bottom: "└──────────────────────────────────────────────┘".to_string(),
                border_color: "bright_blue".to_string(),
                block_rendering: BlockRenderingConfig {
                    block_style: "default".to_string(),
                    custom_blocks: vec!["█".to_string(), "▓".to_string(), "▒".to_string(), "░".to_string(), " ".to_string()],
                    brightness_thresholds: vec![0.8, 0.6, 0.4, 0.2], 
                    color_mode: "truecolor".to_string(),
                    contrast: 1.0,
                    brightness_boost: 0.0,
                    sampling_method: "average".to_string(),
                    enable_dithering: false,
                },
            },
            modules: ModulesConfig {
                show_versions: true,
                os: true,
                kernel: true,
                linux: true,
                uptime: true,
                os_age: true,
                packages: false,
                flatpak_packages: false,
                packages_combined: true,
                shell: true,
                resolution: true,
                network: true,
                public_ip: false,
                de: true,
                wm: true,
                theme: false,
                icons: false,
                terminal: true,
                terminal_shell_combined: false,
                font: true,
                user: true,
                hostname: true,
                user_at_host: true,
                cpu: true,
                cpu_temp: true,
                gpu: true,
                gpu_temp: true,
                temp_combined: false,
                gpu_driver: true,
                memory: true,
                disk: true,
                dysk: true,
                battery: true,
                locale: false,
                display_names: ModuleDisplayConfig {
                    user_at_host: None,
                    os: None,
                    kernel: None,
                    linux: None,
                    uptime: None,
                    os_age: None,
                    packages: None,
                    shell: None,
                    resolution: None,
                    network: None,
                    public_ip: None,
                    de: None,
                    wm: None,
                    theme: None,
                    icons: None,
                    terminal: None,
                    terminal_shell_combined: None,
                    font: None,
                    user: None,
                    hostname: None,
                    cpu: None,
                    cpu_temp: None,
                    gpu: None,
                    gpu_temp: None,
                    temp_combined: None,
                    gpu_driver: None,
                    memory: None,
                    disk: None,
                    dysk: None,
                    battery: None,
                    locale: None,
                },
            },
            show_motd: true,
            motd_file: dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from(".config"))
                .join("termgreet")
                .join("motd.toml"),
        }
    }
}

impl Default for MotdConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            messages: vec![
                "Welcome to your system!".to_string(),
                "Have a great day!".to_string(),
                "Ready to code!".to_string(),
            ],
            random: true,
            color: "bright_green".to_string(),
        }
    }
}

impl Config {
    fn setup_default_assets(config_dir: &Path) -> Result<()> {
        // Create pngs directory if it doesn't exist
        let pngs_dir = config_dir.join("pngs");
        if !pngs_dir.exists() {
            fs::create_dir_all(&pngs_dir)
                .with_context(|| format!("Failed to create pngs directory: {}", pngs_dir.display()))?;
        }
        
        // Copy default logo if it doesn't exist
        let default_logo_path = pngs_dir.join("termgreet_logo.png");
        if !default_logo_path.exists() {
            // Try to find the assets directory relative to the executable
            let exe_path = std::env::current_exe()
                .with_context(|| "Failed to get executable path")?;
            let exe_dir = exe_path.parent()
                .with_context(|| "Failed to get executable directory")?;
            
            // Look for assets in common locations
            let possible_asset_paths = [
                exe_dir.join("../../../assets/termgreet_logo.png"), // Development (target/debug/)
                exe_dir.join("../../assets/termgreet_logo.png"),    // Development (target/release/)
                exe_dir.join("../assets/termgreet_logo.png"),       // Installed relative
                exe_dir.join("assets/termgreet_logo.png"),          // Same directory
                std::env::current_dir().unwrap_or_default().join("assets/termgreet_logo.png"), // Current working directory
            ];
            
            let mut logo_copied = false;
            for asset_path in &possible_asset_paths {
                if asset_path.exists() {
                    fs::copy(asset_path, &default_logo_path)
                        .with_context(|| format!("Failed to copy logo from {} to {}", 
                                                asset_path.display(), default_logo_path.display()))?;
                    logo_copied = true;
                    break;
                }
            }
            
            if !logo_copied {
                // If we can't find the asset, create a placeholder file or just warn
                eprintln!("Warning: Could not find termgreet_logo.png in assets directory. Please manually copy it to {}", default_logo_path.display());
            }
        }
        
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            // Create default config if it doesn't exist
            let config = Self::default();
            config.save(path)?;
            
            // Setup default assets (pngs directory and logo)
            if let Some(config_dir) = path.parent() {
                if let Err(e) = Self::setup_default_assets(config_dir) {
                    eprintln!("Warning: Failed to setup default assets: {}", e);
                }
            }
            
            return Ok(config);
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        
        Ok(())
    }
}

impl MotdConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            let config = Self::default();
            config.save(path)?;
            return Ok(config);
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read MOTD config file: {}", path.display()))?;
        
        let config: MotdConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse MOTD config file: {}", path.display()))?;
        
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize MOTD config")?;
        
        fs::write(path, content)
            .with_context(|| format!("Failed to write MOTD config file: {}", path.display()))?;
        
        Ok(())
    }
}
