pub const SCHEMA_VERSION: u32 = 1;

pub const META_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS Musicd (
    key TEXT PRIMARY KEY,
    value);
";

pub const CACHE_SCHEMA: &str = "
CREATE TABLE Cache (
    key TEXT PRIMARY KEY,
    value BLOB,
    size INTEGER,
    last_access INTEGER);
";

pub cons