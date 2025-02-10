use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rand::{thread_rng, RngCore};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

const BUFFER_SIZE: usize = 1024 * 1024; // 1MB buffer

pub struct DiskWiper {
    device: File,
    device_size: u64,
    passes: u8,
    progress: ProgressBar,
    current_pattern: Option<Vec<u8>>,
}

impl DiskWiper {
    pub fn new(device_path: &Path, passes: u8) -> Result<Self> {
        // Create temporary struct for pre-wipe checks
        let device_path_owned = device_path.to_owned();
        let tmp_wiper = DiskWiper {
            device: File::create("/dev/null")?,  // Temporary file handle
            device_size: 0,
            passes,
            progress: ProgressBar::hidden(),
            current_pattern: None,
        };

        // Ensure device is unmounted
        tmp_wiper.ensure_device_unmounted(&device_path_owned)?;

        // Check if device is an SSD and handle appropriately
        if tmp_wiper.is_ssd(&device_path_owned)? {
            info!("SSD detected - attempting secure erase/TRIM...");
            tmp_wiper.handle_ssd_secure_erase(&device_path_owned)?;
        }

        // Open device with direct I/O if possible
        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .open(device_path)
            .context("Failed to open device")?;

        // Get device size
        let device_size = device
            .metadata()
            .context("Failed to get device metadata")?
            .len();

        // Configure progress bar
        let progress = ProgressBar::new(device_size)
            .with_style(ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap());

        Ok(Self {
            device,
            device_size,
            passes,
            progress,
            current_pattern: None,
        })
    }

    pub fn execute_wipe(&mut self) -> Result<()> {
        let patterns = self.generate_patterns();
        
        for (pass, pattern) in patterns.iter().enumerate() {
            info!("Starting pass {}/{}", pass + 1, self.passes);
            self.current_pattern = Some(pattern.clone());
            self.write_pattern(pattern)?;
            self.progress.reset();
        }

        // Clear final pattern
        self.secure_clear_pattern();
        Ok(())
    }

    pub fn verify_wipe(&mut self) -> Result<()> {
        info!("Verifying device is properly wiped...");
        
        if let Some(ref pattern) = self.current_pattern {
            let mut buffer = vec![0u8; BUFFER_SIZE];
            let mut position = 0;
            
            self.device.seek(SeekFrom::Start(0))?;
            self.progress.reset();

            while position < self.device_size {
                let read_size = std::cmp::min(BUFFER_SIZE as u64, self.device_size - position) as usize;
                self.device.read_exact(&mut buffer[..read_size])?;

                // Pattern-specific verification
                if !buffer[..read_size].iter().zip(pattern.iter().cycle()).all(|(a, b)| a == b) {
                    return Err(anyhow::anyhow!("Verification failed: pattern mismatch found"));
                }

                position += read_size as u64;
                self.progress.set_position(position);
            }
        } else {
            // Fallback to zero-verification if no pattern is set
            let mut buffer = vec![0u8; BUFFER_SIZE];
            let mut position = 0;
            
            self.device.seek(SeekFrom::Start(0))?;
            self.progress.reset();

            while position < self.device_size {
                let read_size = std::cmp::min(BUFFER_SIZE as u64, self.device_size - position) as usize;
                self.device.read_exact(&mut buffer[..read_size])?;

                if buffer[..read_size].iter().any(|&b| b != 0) {
                    return Err(anyhow::anyhow!("Verification failed: non-zero data found"));
                }

                position += read_size as u64;
                self.progress.set_position(position);
            }
        }

        info!("Verification complete: device successfully wiped");
        Ok(())
    }

    /// Parse the block device name from a device path
    fn parse_device_name(device_path: &Path) -> Result<String> {
        let dev_name = device_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid device path"))?;

        if dev_name.starts_with("nvme") {
            // Handle NVMe devices (e.g., nvme2n1p1 -> nvme2n1)
            Ok(dev_name
                .split('p')
                .next()
                .ok_or_else(|| anyhow::anyhow!("Invalid NVMe device name format"))?
                .to_string())
        } else {
            // Handle traditional devices (e.g., sda1 -> sda)
            Ok(dev_name
                .trim_end_matches(char::is_numeric)
                .to_string())
        }
    }

    /// Check if the device is a solid state drive
    fn is_ssd(&self, device_path: &Path) -> Result<bool> {
        let device_name = Self::parse_device_name(device_path)
            .context("Failed to parse device name")?;

        // Check rotational flag in sysfs
        let rotational_path = format!("/sys/block/{}/queue/rotational", device_name);
        let rotational = std::fs::read_to_string(&rotational_path)
            .with_context(|| format!("Failed to read rotational flag at {}", rotational_path))?;

        // 0 indicates SSD, 1 indicates HDD
        Ok(u8::from_str(rotational.trim())
            .with_context(|| format!("Invalid rotational flag value for device {}", device_name))? == 0)
    }

    /// Handle secure erase for SSDs using hdparm or blkdiscard
    fn handle_ssd_secure_erase(&self, device_path: &Path) -> Result<()> {
        // Try hdparm secure erase first
        if let Ok(output) = Command::new("hdparm")
            .args(["-I", &device_path.display().to_string()])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("Security") && output_str.contains("supported") {
                info!("Using hdparm secure erase");
                Command::new("hdparm")
                    .args(["--user-master", "u", "--security-set-pass", "PasSw0rd", 
                          &device_path.display().to_string()])
                    .output()
                    .context("Failed to set security password")?;

                Command::new("hdparm")
                    .args(["--user-master", "u", "--security-erase", "PasSw0rd",
                          &device_path.display().to_string()])
                    .output()
                    .context("Failed to perform secure erase")?;

                return Ok(());
            }
        }

        // Fallback to blkdiscard for TRIM
        info!("Falling back to TRIM using blkdiscard");
        Command::new("blkdiscard")
            .arg(&device_path.display().to_string())
            .output()
            .context("Failed to TRIM device")?;

        Ok(())
    }

    /// Check if device is mounted and unmount if necessary
    fn ensure_device_unmounted(&self, device_path: &Path) -> Result<()> {
        let output = Command::new("mount")
            .output()
            .context("Failed to execute mount command")?;
            
        let mount_info = String::from_utf8_lossy(&output.stdout);
        
        if mount_info.contains(&device_path.display().to_string()) {
            warn!("Device is currently mounted. Attempting to unmount...");
            Command::new("umount")
                .arg(device_path)
                .output()
                .context("Failed to unmount device")?;
            
            info!("Device successfully unmounted");
        }
        
        Ok(())
    }

    /// Securely clear pattern buffer
    fn secure_clear_pattern(&mut self) {
        if let Some(mut pattern) = self.current_pattern.take() {
            for byte in pattern.iter_mut() {
                *byte = 0;
            }
            drop(pattern);
        }
    }

    fn generate_patterns(&self) -> Vec<Vec<u8>> {
        let mut patterns = Vec::with_capacity(self.passes as usize);

        for pass in 0..self.passes {
            match pass {
                0 => patterns.push(vec![0x00; BUFFER_SIZE]),    // First pass: all zeros
                1 => patterns.push(vec![0xFF; BUFFER_SIZE]),    // Second pass: all ones
                _ => {                                          // Subsequent passes: random data
                    let mut pattern = vec![0u8; BUFFER_SIZE];
                    thread_rng().fill_bytes(&mut pattern);
                    patterns.push(pattern);
                }
            }
        }

        patterns
    }

    fn write_pattern(&mut self, pattern: &[u8]) -> Result<()> {
        self.device.seek(SeekFrom::Start(0))?;
        let mut position = 0;

        while position < self.device_size {
            let write_size = std::cmp::min(BUFFER_SIZE as u64, self.device_size - position) as usize;
            self.device.write_all(&pattern[..write_size])?;
            self.device.flush()?;
            
            position += write_size as u64;
            self.progress.set_position(position);
        }

        // Force the OS to flush its buffers
        self.device.sync_all()?;
        Ok(())
    }
}

impl Drop for DiskWiper {
    fn drop(&mut self) {
        self.secure_clear_pattern();
        self.progress.finish_and_clear();
    }
}
