use anyhow::Result;
use log::warn;
use nix::unistd::Uid;
use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

/// Check if the program is running with root privileges
pub fn check_root_privileges() -> bool {
    Uid::effective().is_root()
}

/// Confirm the wipe operation with the user through multiple steps
pub fn confirm_operation(device: &Path) -> Result<bool> {
    println!("\nYou are about to PERMANENTLY DESTROY ALL DATA on: {}", device.display());
    println!("This operation cannot be undone!");
    println!("\nPlease read carefully:");
    println!("1. Ensure you have the proper authorization to wipe this device");
    println!("2. Verify you have selected the correct device");
    println!("3. Confirm all important data has been backed up");
    println!("\nThere will be a 10-second countdown before confirmation.");
    
    // Countdown
    for i in (1..=10).rev() {
        print!("\rConfirmation available in {} seconds...", i);
        io::stdout().flush()?;
        thread::sleep(Duration::from_secs(1));
    }
    println!("\n");

    // First confirmation
    if !get_confirmation("Type 'YES' (all caps) to confirm you understand this is irreversible: ", "YES")? {
        return Ok(false);
    }

    // Second confirmation
    if !get_confirmation("Type 'I ACCEPT' (all caps) to confirm you have authorization: ", "I ACCEPT")? {
        return Ok(false);
    }

    // Final warning
    warn!("FINAL WARNING: Last chance to abort!");
    if !get_confirmation("Type 'PROCEED' (all caps) to begin the wiping process: ", "PROCEED")? {
        return Ok(false);
    }

    Ok(true)
}

/// Get user confirmation with specific expected input
fn get_confirmation(prompt: &str, expected: &str) -> Result<bool> {
    print!("{}", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim() == expected)
}
