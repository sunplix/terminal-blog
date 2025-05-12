use chrono::Local;
use log::{LevelFilter, Log, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

pub struct Logger {
    file: Mutex<File>,
}

impl Logger {
    pub fn new(log_path: &Path) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        Ok(Logger {
            file: Mutex::new(file),
        })
    }

    pub fn init(log_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let logger = Self::new(log_path)?;
        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(LevelFilter::Debug);
        Ok(())
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LevelFilter::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let now = Local::now();
            let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f");
            let log_entry = format!(
                "[{}] [{}] [{}:{}] {}\n",
                timestamp,
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            );

            if let Ok(mut file) = self.file.lock() {
                let _ = file.write_all(log_entry.as_bytes());
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}
