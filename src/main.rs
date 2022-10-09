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

    p