
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

            Ok(result)
        } else if node.node_type == NodeType::File && node.modified != modified {
            let parent = match parent {
                Some(n) => n,
                None => {
                    error!(
                        "root node '{}' isn't directory",
                        node.name.to_string_lossy()
                    );
                    return Err(Error::OtherError);
                }
            };

            let result = if let Some(_master_id) = node.master_id {
                // TODO should this trigger master rescan?
                None
            } else {
                self.index.clear_node(node.node_id)?;

                self.process_file_node(parent, &node, &fs_path)?
            };

            Ok(result)
        } else {
            Ok(None)
        };

        if node.modified != modified {
            self.index.set_node_modified(node.node_id, modified)?;
        }

        result
    }

    fn prepare_node<'a>(
        &mut self,
        parent: Option<&'a Node>,
        node_arg: NodeArg,
    ) -> Result<ScanNode<'a>> {
        let parent_id = match parent {
            Some(node) => Some(node.node_id),
            None => None,
        };

        let (name, path, mut node) = match node_arg {
            NodeArg::Node(node) => (node.name.clone(), node.path.clone(), Some(node)),
            NodeArg::Name(name) => (
                PathBuf::from(name),
                match parent {
                    Some(parent_node) => PathBuf::from(&parent_node.path).join(name),
                    None => PathBuf::from(name),
                },
                None,
            ),
        };

        let fs_path = match self.index.map_fs_path(&path) {
            Some(p) => p,
            None => {
                error!("can't map path '{}'", path.display());
                return Err(Error::OtherError);
            }
        };

        if node.is_none() {
            node = self.index.node_by_name(parent_id, &name)?;
        }

        let metadata = match fs::metadata(&fs_path) {
            Ok(m) => m,
            Err(e) => {
                error!(
                    "metadata error '{}': {}",
                    fs_path.to_string_lossy(),
                    e.description()
                );

                if let Some(node) = node {
                    self.index.delete_node(node.node_id)?;
                }

                return Err(Error::OtherError);
            }
        };

        let modified = match metadata
            .modified()?
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
        {
            Ok(n) => n.as_secs(),
            Err(_) => {
                error!("invalid modified '{}'", fs_path.to_string_lossy());

                if let Some(node) = node {
                    self.index.delete_node(node.node_id)?;
                }

                return Err(Error::OtherError);
            }
        } as i64;

        let node_type = if metadata.is_dir() {
            NodeType::Directory
        } else if metadata.is_file() {
            NodeType::File
        } else {
            NodeType::Other
        };

        if let Some(n) = &node {
            if n.node_type != node_type {
                trace!(
                    "node '{}' type has changed ({:?} => {:?}), recreating node",
                    fs_path.to_string_lossy(),
                    n.node_type,
                    node_type
                );
                self.index.delete_node(n.node_id)?;

                node = None;
            }
        }

        let node = match node {
            Some(n) => n,
            None => {
                let node = Node {
                    node_id: 0,
                    node_type,
                    parent_id: match parent {
                        Some(p) => Some(p.node_id),
                        None => None,
                    },
                    master_id: None,
                    name: name.to_path_buf(),
                    path,
                    modified: 0,
                };

                self.index.create_node(&node)?
            }
        };

        // trace!("prepare_node {} = {}", node.node_id, fs_path.to_string_lossy());

        Ok(ScanNode {
            parent,
            node,
            fs_path,
            modified,
        })
    }

    fn process_directory_node(
        &mut self,
        node: &Node,
        fs_path: &Path,
        modified: bool,
    ) -> Result<Option<ScanStat>> {
        debug!("directory '{}'", fs_path.to_string_lossy());

        let mut stat = ScanStat {
            ..Default::default()
        };

        let mut fs_entries: Vec<_> = Vec::new();

        if modified {
            for entry in fs::read_dir(fs_path)? {
                fs_entries.push(entry?.file_name());
            }
        } else {
            trace!("directory was not modified, not reading file system entries");
        }

        let index_nodes = self.index.nodes_by_parent(Some(node.node_id))?;
        for index_node in index_nodes {
            if let Some(pos) = fs_entries.iter().position(|e| e == &index_node.name) {
                fs_entries.remove(pos);
            }

            if let Ok(scan_node) = self.prepare_node(Some(node), NodeArg::Node(index_node)) {
                if let Ok(Some(node_stat)) = self.scan_node(scan_node) {
                    stat.add(&node_stat);
                }
            }
        }

        for entry in fs_entries {
            if self.interrupted() {
                return Ok(Some(stat));
            }

            if let Ok(Some(node_stat)) = self.scan_node_unprepared(Some(&node), Path::new(&entry)) {
                stat.add(&node_stat);
            }
        }

        Ok(Some(stat))
    }

    fn process_file_node(
        &mut self,
        parent: &Node,
        node: &Node,
        fs_path: &Path,
    ) -> Result<Option<ScanStat>> {
        let extension = match fs_path.extension().and_then(|e| e.to_str()) {
            Some(e) => e.to_ascii_lowercase(),
            None => {
                return Ok(None);
            }
        };

        if let Some(stat) = self.try_process_cue_file(&extension, parent, node, &fs_path)? {
            return Ok(Some(stat));
        }