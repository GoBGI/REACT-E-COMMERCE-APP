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
        trace!("set node node_id={} master_id={}", node_id, master_id);

        self.conn.execute(
            "UPDATE Node SET master_id = ? WHERE node_id = ?",
            params![master_id, node_id],
        )?;
        Ok(())
    }

    pub fn clear_node(&self, node_id: i64) -> Result<()> {
        trace!("clear node node_id={}", node_id);

        self.conn
            .execute("DELETE FROM Track WHERE node_id = ?", &[node_id])?;

        self.conn
            .execute("DELETE FROM Image WHERE node_id = ?", &[node_id])?;

        Ok(())
    }

    fn _get_track(row: &Row) -> Result<Track> {
        Ok(Track {
            track_id: row.get(0)?,
            node_id: row.get(1)?,
            stream_index: row.get(2)?,
            track_index: row.get(3)?,
            start: row.get(4)?,
            number: row.get(5)?,
            title: row.get(6)?,
            artist_id: row.get(7)?,
            artist_name: row.get(8)?,
            album_id: row.get(9)?,
            album_name: row.get(10)?,
            album_artist_id: row.get(11)?,
            album_artist_name: row.get(12)?,
            length: row.get(13)?,
        })
    }

    pub fn track(&self, track_id: i64) -> Result<Option<Track>> {
        trace!("get track track_id={}", track_id);

        let mut st = self.conn
            .prepare(
                "SELECT track_id, node_id, stream_index, track_index, start, number, title, artist_id, artist_name, album_id, album_name, album_artist_id, album_artist_name, length
                FROM Track
                WHERE track_id = ?"
            )?;

        let mut rows = st.query(&[track_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::_get_track(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn create_track(&self, track: &Track) -> Result<Track> {
        let mut st = self.conn
            .prepare(
                "INSERT INTO Track (node_id, stream_index, track_index, start, number, title, artist_id, artist_name, album_id, album_name, album_artist_id, album_artist_name, length)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )?;

        st.execute(params![
            track.node_id,
            track.stream_index,
            track.track_index,
            track.start,
            track.number,
            track.title,
            track.artist_id,
            track.artist_name,
            track.album_id,
            track.album_name,
            track.album_artist_id,
            track.album_artist_name,
            track.length,
        ])?;

        let result = self.track(self.conn.last_insert_rowid())?.unwrap();

        debug!("create {:?}", result);

        Ok(result)
    }

    fn _get_image(row: &Row) -> Result<Image> {
        Ok(Image {
            image_id: row.get(0)?,
            node_id: row.get(1)?,
            stream_index: row.get(2)?,
            description: row.get(3)?,
            width: row.get(4)?,
            height: row.get(5)?,
        })
    }

    pub fn image(&self, image_id: i64) -> Result<Option<Image>> {
        trace!("get image image_id={}", image_id);

        let mut st = self.conn.prepare(
            "SELECT image_id, node_id, stream_index, description, width, height
            FROM Image
            WHERE image_id = ?",
        )?;

        let mut rows = st.query(&[image_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::_get_image(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn create_image(&self, image: &Image) -> Result<Image> {
        let mut st = self.conn.prepare(
            "INSERT INTO Image (node_id, stream_index, description, width, height)
            VALUES (?, ?, ?, ?, ?)",
        )?;

        st.execute(params![
            image.node_id,
            image.stream_index,
            image.description,
            image.width,
            image.height
        ])?;

        let result = self.image(self.conn.last_insert_rowid())?.unwrap();

        debug!("create {:?}", result);

        Ok(result)
    }

    fn _get_artist(row: &Row) -> Result<Artist> {
        Ok(Artist {
            artist_id: row.get(0)?,
            name: row.get(1)?,
        })
    }

    pub fn artist(&self, artist_id: i64) -> Result<Option<Artist>> {
        trace!("get artist artist_id={}", artist_id);

        let mut st = self.conn.prepare(
            "SELECT artist_id, name
            FROM Artist
            WHERE artist_id = ?",
        )?;

        let mut rows = st.query(&[artist_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::_get_artist(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn artist_by_name(&self, name: &str) -> Result<Option<Artist>> {
        trace!("get artist name={}", name);

        le