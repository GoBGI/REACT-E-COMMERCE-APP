use std::cell::RefCell;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use chrono::prelude::*;
use log::{Level, Metadata, Record};

use crate::musicd_c::{self, LogLevel};

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.target().starts_with("musicd2")
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let mut target = record.target();
        if !target.starts_with("musicd2") {
            return;
        }

        if target.starts_with("musicd2::") {
            target = target.get(("musicd2::").len()..).unwrap();
        }

        eprintln!(
            "{} {:05} [{}] {}",
            Local::now().format("%F %T"),
            record.level(),
            target,
            record.args()
        );
    }

    fn flush(&self) {}
}

thread_local!(static LOG_C_BUF: RefCell<String> = RefCell::new(String::new()));

extern "C" fn log_c_callback(level: c_int, message: *const c_char) {
    let log_level = if level == LogLevel::LogLevelError as i32 {
        Level::Error
    } else if level == LogLevel::LogLevelWarn as i32 {
        Level::Warn
    } else if level == LogLevel::LogLevelInfo as i32 {
        Leve