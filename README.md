# Kill-Switch: Secure Disk Wiping Tool

A secure disk wiping tool implementing the DoD 5220.22-M data sanitization standard. This tool provides a reliable way to permanently erase data from storage devices.

⚠️ **WARNING: THIS TOOL PERMANENTLY DESTROYS ALL DATA ON THE TARGET DEVICE** ⚠️

## Security Features

- Implements DoD 5220.22-M standard with multiple overwrite passes
- Enforces root privileges for disk operations
- Multi-step confirmation process to prevent accidents
- Comprehensive logging and audit trail
- Pattern-specific verification of wiped data
- Secure memory handling with automatic buffer sanitization
- Automatic filesystem unmounting
- SSD-aware wiping with secure erase support
- TRIM command support for modern SSDs
- Secure error handling and resource management

## Requirements

- Linux system (kernel 2.6 or later)
- Root privileges
- Rust toolchain

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/kill-switch.git
cd kill-switch

# Build the project
cargo build --release

# Install (optional)
sudo cp target/release/kill-switch /usr/local/bin/
```

## Usage

```bash
# Basic usage (requires root privileges)
sudo kill-switch --device /dev/sdX

# Specify number of passes (default is 3)
sudo kill-switch --device /dev/sdX --passes 7

# Enable verification after wiping
sudo kill-switch --device /dev/sdX --verify

# Skip confirmation prompts (USE WITH EXTREME CAUTION)
sudo kill-switch --device /dev/sdX --force
```

## Safety Precautions

1. **ALWAYS VERIFY THE CORRECT DEVICE PATH**
   - Double-check the device path before confirming
   - Use `lsblk` or `fdisk -l` to list available devices
   - Never wipe system drives or mounted partitions

2. **Ensure Proper Authorization**
   - Only wipe devices you own or have explicit permission to wipe
   - Keep audit logs for compliance purposes
   - Follow your organization's data destruction policies

3. **Data Recovery is Impossible**
   - All data will be permanently destroyed
   - No recovery tools can restore the data
   - Verify backups before proceeding

## How It Works

1. **Verification Phase**
   - Checks for root privileges
   - Validates device accessibility
   - Multiple user confirmations

2. **Wiping Phase**
   - Detects device type (SSD vs HDD)
   - For SSDs:
     - Attempts hardware secure erase if supported
     - Falls back to TRIM command
     - Follows with DoD pattern wiping
   - For HDDs (DoD 5220.22-M):
     - Pass 1: Writes all zeros
     - Pass 2: Writes all ones
     - Pass 3: Writes random data
     - Additional passes if specified

3. **Verification Phase**
   - Pattern-specific verification for each pass
   - Reads entire device for thorough checking
   - Verifies successful data destruction
   - Reports any verification failures
   - Ensures proper unmounting post-operation

## Logging

Audit logs are stored in `/var/log/kill-switch/` with timestamps and detailed operation information.

## Error Handling

- Graceful handling of I/O errors
- Safe resource cleanup on failure
- Clear error messages and logging
- Prevention of partial wipes

## Legal Disclaimer

This tool is provided "as is" without warranty. Users are responsible for:
- Ensuring proper authorization to wipe devices
- Complying with data protection laws
- Following organizational security policies
- Preventing accidental data loss

## License

MIT License
