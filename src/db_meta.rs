use rusqlite::OptionalExtension;
use rusqlite::{Connection, Result, NO_PARAMS};

use crate::schema;

pub fn ensure_schema(conn: &mut Connection, schema: &str) -> Re