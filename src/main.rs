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
use inde