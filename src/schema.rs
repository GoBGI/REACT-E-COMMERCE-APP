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
CREATE INDEX Node_master_id ON Node (master_id);
    
CREATE TABLE Track (
    track_id INTEGER PRIMARY KEY AUTOINCREMENT, 
    node_id INTEGER NOT NULL,
    stream_index INTEGER NOT NULL,
    track_index INTEGER,
    start REAL,
    number INTEGER NOT NULL,
    title TEXT NOT NULL,
    artist_id INTEGER NOT NULL,
    artist_name TEXT NOT NULL,
    album_id INTEGER NOT NULL,
    album_name TEXT NOT NULL,
    album_artist_id INTEGER,
    album_artist_name TEXT,
    length REAL NOT NULL,
    FOREIGN KEY(node_id)