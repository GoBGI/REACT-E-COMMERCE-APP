
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
    DatabaseError(rusqlite::Error),
    OtherError,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Error {
        Error::DatabaseError(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IoError(ref e) => e.description(),
            Error::DatabaseError(ref e) => e.description(),
            Error::OtherError => "Other error",
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct ScanThread {
    stop: Arc<AtomicBool>,
    join_handle: Mutex<Option<JoinHandle<ScanStat>>>,
}

impl ScanThread {
    pub fn new() -> ScanThread {
        ScanThread {
            stop: Arc::new(AtomicBool::new(false)),
            join_handle: Mutex::new(None),
        }
    }

    pub fn is_running(&self) -> bool {
        self.join_handle.lock().unwrap().is_some()
    }

    pub fn start(&self, index: Index) {
        {
            if self.join_handle.lock().unwrap().is_some() {
                return;
            }
        }

        let stop = self.stop.clone();

        let mut join_handle = self.join_handle.lock().unwrap();

        self.stop.store(false, Ordering::Relaxed);

        *join_handle = Some(std::thread::spawn(move || {
            let mut scan = Scan {
                stop,
                stop_detected: false,
                index,
            };

            scan.scan_core()
        }));
    }

    pub fn stop(&self) {
        let mut join_handle = self.join_handle.lock().unwrap();

        self.stop.store(true, Ordering::Relaxed);

        if let Some(handle) = join_handle.take() {
            handle.join().unwrap();
        }
    }
}

struct Scan {
    stop: Arc<AtomicBool>,
    stop_detected: bool,
    index: Index,
}

enum NodeArg<'a> {
    Node(Node),
    Name(&'a Path),
}

struct ScanNode<'a> {
    parent: Option<&'a Node>,
    node: Node,
    fs_path: PathBuf,
    modified: i64,
}

#[derive(Debug, Default)]
struct ScanStat {
    tracks: i32,
    images: i32,
}

impl ScanStat {
    fn add(&mut self, other: &ScanStat) {
        self.tracks += other.tracks;
        self.images += other.images;
    }

    fn changed(&self) -> bool {
        self.tracks > 0 || self.images > 0
    }
}

impl Scan {
    fn interrupted(&mut self) -> bool {
        let stop = self.stop.load(Ordering::Relaxed);

        if stop && !self.stop_detected {
            self.stop_detected = true;
            debug!("interrupt noted, stopping");
        }

        stop
    }

    fn scan_core(&mut self) -> ScanStat {
        info!("started");

        let mut stat = ScanStat {
            ..Default::default()
        };

        if self
            .index
            .connection()
            .execute_batch("DELETE FROM AlbumImagePattern;")
            .is_err()
        {
            return stat;