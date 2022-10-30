#[macro_use]
extern crate log;

mod audio_stream;
mod cache;
mod cue;
mod db_meta;
mod http_api;
mod http_util;
mod index;
mod logger;
mod lyrics;
mod media;
mod musicd_c;
mod query;
mod scan;
mod schema;
mod store;

use std::ffi::OsStr;
use std::net::SocketAddr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::Arg;

use cache::{Cache, CacheSource};
use index::{Index, IndexSource};
use scan::ScanThread;
use store::{Store, StoreSource};

pub struct Musicd {
    cache_source: CacheSource,
    index_source: IndexSource,
    store_source: StoreSource,
    scan_thread: ScanThread,
    password: String,
}

pub struct Root {
    pub name: String,
    pub path: PathBuf,
}

pub const MUSICD_VERSION: &str = env!("CARGO_PKG_VERSION");

impl Musicd {
    pub fn cache(&self) -> Box<dyn Cache> {
        self.cache_source.get().expect("can't open cache")
    }

    pub fn index(&self) -> Index {
        self.index_source.get().expect("can't open index")
    }

    pub fn store(&self) -> Store {
        self.store_source
            .get(self.index())
            .expect("can't open store")
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = clap::App::new("musicd2")
        .version(MUSICD_VERSION)
        .arg(
            Arg::with_name("bind")
                .long("bind")
                .help("HTTP server address and port")
                .default_value("127.0.0.1:6801"),
        )
        .arg(
            Arg::with_name("cache-limit")
                .long("cache-limit")
                .help("Maximum cache size in bytes")
                .default_value("104857600"),
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Database directory")
                .default_value("~/.musicd2"),
    