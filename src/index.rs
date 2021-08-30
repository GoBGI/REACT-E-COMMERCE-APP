use std::error::Error as StdError;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{params, Connection, Result, Row, NO_PARAMS};
use serde::Serialize;

use crate::db_meta;
use crate::schema;
use crate::Root;

#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub enum NodeType {
    Other = 0,
    Directory = 1,
    File = 2,
}

impl NodeType {
    pub fn from_i64(v: i64) -> NodeType {
        match v {
            1 => NodeType::Directory,
            2 => NodeType::File,
            _ => NodeType::Other,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub node_id: i64,
    pub node_type: NodeType,
    pub parent_id: Option<i64>,
    pub master_id: Option<i64>,
    pub name: PathBuf,
    pub path: PathBuf,
    pub modified: i64,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub track_id: i64,
    pub node_id: i64,
    pub stream_index: i64,
    pub track_index: Option<i64>,
    pub start: Option<f64>,
    pub number: i64,
    pub title: String,
    pub artist_id: i64,
    pub artist_name: String,
    pub album_id: i64,
    pub album_name: String,
    pub album_artist_id: Option<i64>,
    pub album_artist_name: Option<String>,
    pub length: f64,
}

#[derive(Debug, Clone)]
pub struct Image {
    pub image_id: i64,
    pub node_id: i64,
    pub stream_index: Option<i64>,
    pub description: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Clone)]
pub struct Album {
    pub album_id: i64,
    pub name: String,
    pub artist_id: Option<i64>,
    pub artist_name: Option<String>,
    pub image_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Artist {
    pub artist_id: i64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TrackLyrics {
    pub track_id: i64,
    pub lyrics: Option<String>,
    pub provider: Option<String>,
    pub source: Option<String>,
    pub modified: i64,
}

pub struct IndexSource {
    db_path: PathBuf,
    roots: Arc<Vec<Root>>,
}

pub struct Index {
    conn: Connection,
    roots: Arc<Vec<Root>>,
}

impl IndexSource {
    pub fn create(db_path: PathBuf, roots: Arc<Vec<Root>>) -> Result<Option<IndexSource>> {
        info!("using '{}'", db_path.to_string_lossy());

        let source = IndexSource { db_path, roots };

        let mut index = source.get()?;
        if !db_meta::ensure_schema(&mut index.conn, schema::INDEX_SCHEMA)? {
            return Ok(None);
        }

        Ok(Some(source))
    }

    pub fn get(&self) -> Result<Index> {
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(e) => {
                error!(
                    "can't open sqlite database '{}': {}",
                    self.db_path.to_string_lossy(),
                    e.description()
                );
                return Err(e);
            }
        };

        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;",
        )?;

        Ok(Index {
            conn,
            roots: self.roots.clone(),
        })
    }
}

impl Index {
    pub fn roots(&self) -> &Vec<Root> {
        &self.roots
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn map_fs_path(&self, path: &Path) -> Option<PathBuf> {
        let mut iter = path.iter();

        let root_name = match iter.next() {
            Some(name) => match name.to_str() {
                Some(name) => name,
                None => return None,
            },
            None => return None,
        };

        let root_dir = match self.roots.iter().find(|&r| r.name == root_name) {
            Some(name) => name,
            None => return None,
        };

        let mut result = PathBuf::from(&root_dir.path);

        for component in iter {
            result.push(component);
        }

        Some(result)
    }

    fn _get_node(row: &Row) -> Result<Node> {
        let node_type: i64 = row.get(1)?;
        let name_bytes: Vec<u8> = row.get(4)?;
        let path_bytes: Vec<u8> = row.get(5)?;

        Ok(Node {
            node_id: row.get(0)?,
            node_type: NodeType::from_i64(node_type as i64),
            parent_id: row.get(2)?,
            master_id: row.get(3)?,
            name: Path::new(OsStr::from_bytes(&name_bytes)).to_path_buf(),
            path: Path::new(OsStr::from_bytes(&path_bytes)).to_path_buf(),
            modified: row.get(6)?,
        })
    }

    pub fn node(&self, node_id: i64) -> Result<Option<Node>> {
        trace!("get node node_id={}", node_id);

        let mut st = self.conn.prepare(
            "SELECT node_id, node_type, parent_id, master_id, name, path, modified
            FROM Node
            WHERE node_id = ?",
        )?;

        let mut rows = st.query(&[node_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::_get_node(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn node_by_name(&self, parent_id: Option<i64>, name: &Path) -> Result<Option<Node>> {
        trace!(
            "get node parent_id={:?} name='{}'",
            parent_id,
            name.to_string_lossy()
        );

        let mut st = self.conn.prepare(match parent_id {
            Some(_) => "
                SELECT node_id, node_type, parent_id, master_id, name, path, modified
                FROM Node
                WHERE name = ? AND parent_id = ?",
            None => "
                SELECT node_id, node_type, parent_id, master_id, name, path, modified
                FROM Node
                WHERE name = ? AND parent_id IS NULL",
        })?;

        let name_bytes = name.as_os_str().as_bytes();

        let mut rows = match parent_id {
            Some(id) => st.query(params![name_bytes, id])?,
            None => st.query(&[name_bytes])?,
        };

        if let Some(row) = rows.next()? {
            Ok(Some(Self::_get_node(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn node_by_path(&self, path: &Path) -> Result<Option<Node>> {
        trace!("get node path='{}'", path.to_string_lossy());

        let mut st = self.conn.prepare(
            "SELECT node_id, node_type, parent_id, master_id, name, path, modified
            FROM Node
            WHERE path = ?",
        )?;

        let path_bytes = path.as_os_str().as_bytes();

        let mut rows = st.query(&[path_bytes])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::_get_node(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn nodes_by_parent(&self, parent_id: Option<i64>) -> Result<Vec<Node>> {
        trace!("list nodes by parent_id={:?}", parent_id);

        let mut st = self.conn.prepare(match parent_id {
            Some(_) => "
                SELECT node_id, node_type, parent_id, master_id, name, path, modified
                FROM Node
                WHERE parent_id = ?",
            None => "
                SELECT node_id, node_type, parent_id, master_id, name, path, modified
                FROM Node
                WHERE parent_id IS NULL",
        })?;

        let mut rows = match parent_id {
            Some(id) => st.query(&[id])?,
            None => st.query(NO_PARAMS)?,
        };

        let mut result = Vec::new();

        while let Some(row) = rows.next()? {
            result.push(Self::_get_node(row)?);
        }

        Ok(result)
    }

    pub fn create_node(&self, node: &Node) -> Result<Node> {
        let mut st = self.conn.prepare(
            "INSERT INTO Node (node_type, parent_id, master_id, name, path, modified)
            VALUES (?, ?, ?, ?, ?, ?)",
        )?;

        st.execute(params![
            node.node_type as i64,
            node.parent_id,
            node.master_id,
            node.name.as_os_str().as_bytes(),
            node.path.as_os_str().as_bytes(),
            node.modified,
        ])?;

        let result = self.node(self.conn.last_insert_rowid())?.unwrap();

        debug!("create {:?}", result);

        Ok(result)
    }

    pub fn delete_node(&self, node_id: i64) -> Result<()> {
        trace!("delete node node_id={}", node_id);

        self.conn
            .execute("DELETE FROM Node WHERE node_id = ?", &[node_id])?;
        Ok(())
    }

    pub fn set_node_modified(&self, node_id: i64, modified: i64) -> Result<()> {
        trace!("set node node_id={} modified={}", node_id, modified);

        self.conn.execute(
            "UPDATE Node SET modified = ? WHERE node_id = ?",
            params![modified, node_id],
        )?;
        Ok(())
    }

    pub fn set_node_master(&self, node_id: i64, master_id: i64) -> Result<()> {
        trace!("set node node_id={} master_id={}", node_id, master_id