use chrono::prelude::Utc;
use log;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};


pub struct FileLogger {
    log_file: PathBuf,
    level: log::Level,
}

impl FileLogger {
    ///
    /// Start a new log file with the time and date at the top.
    ///
    pub fn new<P: AsRef<Path>>(log_file: P, level: log::Level) -> FileLogger {
        FileLogger {
            log_file: log_file.as_ref().to_path_buf(),
            level: level,
        }
    }
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    /// Write a message to the log file.
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let file = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .create(true)
                        .open(&self.log_file);

            if file.is_err() {
                eprintln!(
                    "ERROR: Could not open the file {} for writing.",
                    self.log_file.display()
                );

                return;
            }

            let mut file = file.unwrap();
            let date = Utc::now();
            writeln!(file, "[{}] {}", date, record.args()).unwrap();
        }
    }

    /// Finish writing to a log. This function is used to place any final
    /// information in a log file before the logger goes out of scope.
    fn flush(&self) {
    }
}

pub fn init_with_level(log_file: &str, level: log::Level) -> Result<(), log::SetLoggerError> {
    let logger = FileLogger::new(log_file, level);
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(level.to_level_filter());
    Ok(())
}

pub fn init(log_file: &str) -> Result<(), log::SetLoggerError> {
    init_with_level(log_file, log::Level::Trace)
}
