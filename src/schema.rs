pub const SCHEMA_VERSION: u32 = 1;

pub const META_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS Musicd (
    key TEXT PRIMARY KEY,
