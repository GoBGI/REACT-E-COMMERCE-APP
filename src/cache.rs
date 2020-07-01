
use std::error::Error as StdError;
use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension, Result, NO_PARAMS};

use crate::db_meta;
use crate::schema;

pub struct CacheSource {
    db_path: Option<PathBuf>,
    max_size: usize,
}

pub trait Cache {
    fn get_blob(&self, key: &str) -> Result<Option<Vec<u8>>>;
    fn set_blob(&self, key: &str, value: &[u8]) -> Result<()>;
}

struct DummyCache;

struct SqliteCache {
    conn: Connection,
    max_size: usize,
}

impl CacheSource {
    pub fn create(db_path: Option<PathBuf>, max_size: usize) -> Result<Option<CacheSource>> {
        let source = CacheSource { db_path, max_size };

        if let Some(db_path) = &source.db_path {
            info!(
                "using '{}', max_size={}",
                db_path.to_string_lossy(),
                max_size
            );

            let mut conn = Self::get_connection(&db_path)?;
            if !db_meta::ensure_schema(&mut conn, schema::CACHE_SCHEMA)? {
                return Ok(None);
            }
        } else {
            info!("disabled");
        }

        Ok(Some(source))