use anyhow::Result;
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

pub struct AuditLogger {
    log_file: Mutex<File>,
}

impl AuditLogger {
    pub fn init(log_path: PathBuf) -> Result<(), SetLoggerError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open log file");

        let logger = Box::new(AuditLogger {
            log_file: Mutex::new(file),
        });

        log::set_boxed_logger(logger)
            .map(|()| log::set_max_level(LevelFilter::Info))
    }
}

impl log::Log for AuditLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let log_line = format!(
                "[{timestamp}] [{level}] {message}\n",
                timestamp = timestamp,
                level = record.level(),
                message = record.args()
            );

            if let Ok(mut file) = self.log_file.lock() {
                let _ = file.write_all(log_line.as_bytes());
                let _ = file.flush();
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.log_file.lock() {
            let _ = file.flush();
        }
    }
}

// Helper function to initialize audit logging
pub fn init_logging() -> Result<()> {
    let log_dir = PathBuf::from("/var/log/kill-switch");
    std::fs::create_dir_all(&log_dir)?;
    
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let log_file = log_dir.join(format!("wipe-{}.log", timestamp));
    
    AuditLogger::init(log_file)?;
    Ok(())
}
