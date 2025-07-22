use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub display: DisplayConfig,
    pub modules: ModulesConfig,
    pub motd_file: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeneralConfig {
    pub title: Option<String>,
    pub separator: String,
    pub colors: ColorsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ColorsConfig {
    pub title: String,
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
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModulesConfig {
    pub os: bool,
    pub kernel: bool,
    pub uptime: bool,
    pub os_age: bool,
    pub packages: bool,
    pub shell: bool,
    pub resolution: bool,
    pub de: bool,
    pub wm: bool,
    pub theme: bool,
    pub icons: bool,
    pub terminal: bool,
    pub cpu: bool,
    pub gpu: bool,
    pub memory: bool,
    pub disk: bool,
    pub battery: bool,
    pub locale: bool,
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
                title: Some("System Information".to_string()),
                separator: " -> ".to_string(),
                colors: ColorsConfig {
                    title: "bright_cyan".to_string(),
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
                },
                prefer_kitty_graphics: true,
                padding: 2,
            },
            modules: ModulesConfig {
                os: true,
                kernel: true,
                uptime: true,
                os_age: true,
                packages: false,
                shell: true,
                resolution: true,
                de: true,
                wm: true,
                theme: false,
                icons: false,
                terminal: true,
                cpu: true,
                gpu: true,
                memory: true,
                disk: true,
                battery: true,
                locale: false,
            },
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
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            // Create default config if it doesn't exist
            let config = Self::default();
            config.save(path)?;
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
