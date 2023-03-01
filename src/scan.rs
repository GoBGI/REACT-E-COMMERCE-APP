
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
        }

        if self
            .index
            .connection()
            .execute_batch(
                "
                INSERT INTO AlbumImagePattern (pattern)
                VALUES
                    ('album cover'),
                    ('albumcover'),
                    ('albumart'),
                    ('album'),
                    ('front'),
                    ('folder'),
                    ('front%'),
                    ('cover%'),
                    ('folder%'),
                    ('%front%'),
                    ('%cover%'),
                    ('%folder%'),
                    ('%albumart%'),
                    ('%album%'),
                    ('%jacket%'),
                    ('%card%')",
            )
            .is_err()
        {
            return stat;
        }

        let roots: Vec<(String, PathBuf)> = self
            .index
            .roots()
            .iter()
            .map(|r| (r.name.to_string(), r.path.to_path_buf()))
            .collect();

        let start_instant = Instant::now();

        for (name, path) in roots {
            if self.interrupted() {
                return stat;
            }

            debug!("root '{}' = '{}'", name, path.to_string_lossy());

            match self.scan_node_unprepared(None, Path::new(OsStr::from_bytes(name.as_bytes()))) {
                Ok(s) => {
                    if let Some(s) = s {
                        stat.add(&s);
                    }
                }
                Err(e) => {
                    error!(
                        "can't scan root '{}' -> '{}': {}",
                        name,
                        path.to_string_lossy(),
                        e.description()
                    );
                }
            }
        }

        info!("done in {}s: {:?}", start_instant.elapsed().as_secs(), stat);

        stat
    }

    fn scan_node_unprepared(
        &mut self,
        parent: Option<&Node>,
        name: &Path,
    ) -> Result<Option<ScanStat>> {
        let scan_node = self.prepare_node(parent, NodeArg::Name(name))?;
        self.scan_node(scan_node)
    }

    fn scan_node(&mut self, scan_node: ScanNode) -> Result<Option<ScanStat>> {
        let ScanNode {
            parent,
            node,
            fs_path,
            modified,
        } = scan_node;

        let result = if node.node_type == NodeType::Directory {
            let result = self.process_directory_node(&node, &fs_path, node.modified != modified)?;

            if let Some(result) = &result {
                if result.changed() {
                    self.index.process_node_updates(node.node_id)?;
                }
            }