use rusqlite::OptionalExtension;
use rusqlite::{Connection, Result, NO_PARAMS};

use crate::schema;

pub fn ensure_schema(conn: &mut Connection, schema: &str) -> Result<bool> {
    trace!("trying to get schema version");

    conn.execute_batch(schema::META_SCHEMA)?;

    let schema_version: Option<u32> = conn
        .query_row(
            "SELECT value FROM Musicd WHERE key = 'schema'",
            NO_PARAMS,
            |row| row.get(0),
        )
        .optional()