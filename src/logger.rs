use std::cell::RefCell;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use chrono::prelude::*;
use log::{Level, Metadata, Record};

use crate::