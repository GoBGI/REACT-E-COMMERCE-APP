
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
    }

    fn get_connection(db_path: &PathBuf) -> Result<Connection> {
        match Connection::open(db_path) {
            Ok(c) => Ok(c),
            Err(e) => {
                error!(
                    "can't open sqlite database '{}': {}",
                    db_path.to_string_lossy(),
                    e.description()
                );
                Err(e)
            }
        }
    }

    pub fn get(&self) -> Result<Box<dyn Cache>> {
        match &self.db_path {
            Some(p) => Ok(Box::new(SqliteCache {
                conn: Self::get_connection(&p)?,
                max_size: self.max_size,
            })),
            None => Ok(Box::new(DummyCache {})),
        }
    }
}

impl Cache for DummyCache {
    fn get_blob(&self, key: &str) -> Result<Option<Vec<u8>>> {
        trace!("dummy get blob '{}'", key);
        Ok(None)
    }

    fn set_blob(&self, key: &str, _value: &[u8]) -> Result<()> {
        trace!("dummy set blob '{}'", key);
        Ok(())
    }
}

impl Cache for SqliteCache {