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

pub const INDEX_SCHEMA: &str = "
CREATE TABLE Node (
    node_id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_type INTEGER NOT NULL,
    parent_id INTEGER,
    master_id INTEGER,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    modified INTEGER NOT NULL,
    FOREIGN KEY(parent_id) REFERENCES Node(node_id) ON DELETE CASCADE,
    FOREIGN KEY(master_id) REFERENCES Node(node_id) ON DELETE SET NULL);

CREATE INDEX Node_parent_id ON Node (parent_id);
CREATE INDEX Node_master_id ON Node (master_id)