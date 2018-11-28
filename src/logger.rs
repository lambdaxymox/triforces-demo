use chrono::prelude::Utc;
use log;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::fmt;


pub struct Logger {
    log_file: String,
}

impl Logger {
    ///
    /// Start a new log file with the time and date at the top.
    ///
    fn new(log_file: &str) -> Logger {
        Logger {
            log_file: String::from(log_file),
        }
    }

    ///
    /// Start a new log file with the time and date at the top.
    ///
    pub fn restart(&self) -> bool {
        let file = File::create(&self.log_file);
        if file.is_err() {
            eprintln!(
                "ERROR: The OpenGL log file \"{}\" could not be opened for writing.", self.log_file
            );

            return false;
        }

        let mut file = file.unwrap();

        let date = Utc::now();
        writeln!(file, "OpenGL application log.\nStarted at local time {}", date).unwrap();
        writeln!(file, "build version: ??? ?? ???? ??:??:??\n\n").unwrap();

        true
    }
}

impl<'a> From<&'a str> for Logger {
    fn from(log_file: &'a str) -> Logger {
        Logger::new(log_file)
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        <Logger as log::Log>::flush(self);
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        true
    }

    /// Write a message to the log file.
    fn log(&self, record: &log::Record) {
        let file = OpenOptions::new().write(true).append(true).open(&self.log_file);
        if file.is_err() {
            eprintln!("ERROR: Could not open GL_LOG_FILE {} file for appending.", &self.log_file);
        }

        let mut file = file.unwrap();
        writeln!(file, "{}", record.args()).unwrap();
    }

    /// Finish writing to a log. This function is used to place any final
    /// information in a log file before the logger goes out of scope.
    fn flush(&self) {
        let file = OpenOptions::new().write(true).append(true).open(&self.log_file);
        if file.is_err() {
            eprintln!("ERROR: Could not open GL_LOG_FILE {} file for appending.", &self.log_file);
        }

        let mut file = file.unwrap();
        let date = Utc::now();
        writeln!(file, "Logging finished at local time {}", date).unwrap();
        writeln!(file, "END LOG").unwrap();
    }
}

pub fn init_with_level(log_file: &str, level: log::Level) -> Result<(), log::SetLoggerError> {
    let logger = Logger::new(log_file);
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(level.to_level_filter());
    Ok(())
}

pub fn init(log_file: &str) -> Result<(), log::SetLoggerError> {
    init_with_level(log_file, log::Level::Trace)
}
