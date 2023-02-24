
use std::convert::From;
use std::error::Error as StdError;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use crate::cue;
use crate::index::{Image, Index, Node, NodeType, Track};
use crate::media;

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),