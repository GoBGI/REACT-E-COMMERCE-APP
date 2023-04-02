
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
        };

        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;",
        )?;

        Ok(Store { conn, index })
    }
}

impl Store {
    pub fn synchronize(&mut self) -> Result<()> {
        debug!("synchronize");

        let store_conn = &self.conn;
        let index_conn = self.index.connection();

        index_conn.execute_batch(
            "DELETE FROM StoreListTrack;
            DELETE FROM StoreList;
            DELETE FROM StoreTrack;",
        )?;

        let mut st = store_conn.prepare(
            "SELECT store_track_id, title, artist_name, album_name, length, play_count, last_play
            FROM Track",
        )?;

        let mut rows = st.query(NO_PARAMS)?;

        let mut st = index_conn.prepare(
            "INSERT OR IGNORE INTO
                StoreTrack (track_id, store_track_id, play_count, last_play)
            VALUES
                (
                    (
                        SELECT track_id
                        FROM Track
                        WHERE
                            Track.title = ? AND
                            Track.artist_name = ? AND
                            Track.album_name = ?
                    ),
                    ?,
                    ?,
                    ?
                )",
        )?;

        while let Some(row) = rows.next()? {
            let store_track = StoreTrack {
                store_track_id: row.get(0)?,
                title: row.get(1)?,
                artist_name: row.get(2)?,
                album_name: row.get(3)?,
                length: row.get(4)?,
                play_count: row.get(5)?,
                last_play: row.get(6)?,
            };

            trace!("add to index {:?}", store_track);

            st.execute(params![
                store_track.title,
                store_track.artist_name,