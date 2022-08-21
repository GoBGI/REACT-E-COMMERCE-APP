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
            target = target.