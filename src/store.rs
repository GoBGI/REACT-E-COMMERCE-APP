
use std::error::Error as StdError;
use std::path::PathBuf;

use rusqlite::{params, Connection, Result, NO_PARAMS};

use crate::db_meta;
use crate::index::Index;
use crate::schema;

#[derive(Debug, Clone)]
struct StoreTrack {
    store_track_id: i64,
    title: String,
    artist_name: String,
    album_name: String,
    length: i64,
    play_count: Option<i64>,
    last_play: Option<i64>,
}

pub struct StoreSource {
    db_path: PathBuf,
}

pub struct Store {
    conn: Connection,
    index: Index,
}

impl StoreSource {
    pub fn create(db_path: PathBuf, index: Index) -> Result<Option<StoreSource>> {
        info!("using '{}'", db_path.to_string_lossy());

        let source = StoreSource { db_path };

        let mut store = source.get(index)?;
        if !db_meta::ensure_schema(&mut store.conn, schema::STORE_SCHEMA)? {
            return Ok(None);
        }

        Ok(Some(source))
    }

    pub fn get(&self, index: Index) -> Result<Store> {
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