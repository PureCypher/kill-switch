use anyhow::Result;
use clap::Parser;
use log::{error, info, warn};
use std::path::PathBuf;

mod wiping;
mod verification;
mod logging;

#[derive(Parser)]
#[command(name = "kill-switch")]
#[command(about = "Secure disk wiping tool implementing DoD 5220.22-M standard")]
struct Cli {
    /// Path to the device or file to wipe
    #[arg(short, long)]
    device: PathBuf,

    /// Number of passes (default is 3 for DoD 5220.22-M standard)
    #[arg(short, long, default_value = "3")]
    passes: u8,

    /// Skip confirmation prompts (USE WITH EXTREME CAUTION)
    #[arg(short, long)]
    force: bool,

    /// Enable verification after wiping
    #[arg(short, long)]
    verify: bool,
}

fn main() -> Result<()> {
    // Initialize logging
    logging::init_logging()?;
    info!("Kill-Switch initializing...");

    // Parse command line arguments
    let cli = Cli::parse();

    // Check for root privileges
    if !verification::check_root_privileges() {
        error!("This program requires root privileges to perform disk operations");
        std::process::exit(1);
    }

    // Display warning and get confirmation
    if !cli.force {
        warn!("WARNING: This operation will permanently destroy all data on {}", 
              cli.device.display());
        warn!("This action is IRREVERSIBLE and should only be used on devices you are authorized to wipe");
        
        if !verification::confirm_operation(&cli.device)? {
            info!("Operation cancelled by user");
            return Ok(());
        }
    }

    // Initialize the wiper
    let mut wiper = wiping::DiskWiper::new(&cli.device, cli.passes)?;

    // Perform the wipe operation
    info!("Starting secure wipe operation...");
    wiper.execute_wipe()?;

    // Verify if requested
    if cli.verify {
        info!("Verifying wipe operation...");
        wiper.verify_wipe()?;
    }

    info!("Wipe operation completed successfully");
    Ok(())
}
