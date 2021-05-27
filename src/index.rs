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

    pub fn map_fs_path(&self, path: &Path) -> O