use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod config;
mod system_info;
mod display;

use config::{Config, MotdConfig};
use system_info::SystemInfo;
use display::Display;

#[derive(Parser)]
#[command(name = "termgreet")]
#[command(about = "A configurable system information tool")]
struct Cli {
    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,
    
    /// Show MOTD only
    #[arg(short, long)]
    motd: bool,
    
    /// Disable image display
    #[arg(long)]
    no_image: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let config_path = cli.config.unwrap_or_else(|| {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("termgreet")
            .join("config.toml")
    });
    
    let config = Config::load(&config_path)?;
    
    if cli.motd {
        let motd_config = MotdConfig::load(&config.motd_file)?;
        Display::show_motd(&motd_config);
        return Ok(());
    }
    
    let system_info = SystemInfo::gather();
    let display = Display::new(config, !cli.no_image);
    
    display.show(&system_info)?;
    
    Ok(())
}
