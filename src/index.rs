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
    pub