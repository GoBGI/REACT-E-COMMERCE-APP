
use std::collections::HashMap;
use std::error::Error as StdError;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde_json::json;

use crate::audio_stream::AudioStream;
use crate::http_util::HttpQuery;
use crate::index::TrackLyrics;
use crate::lyrics;
use crate::media;
use crate::Musicd;

#[derive(Debug)]
pub enum Error {
    HyperError(hyper::Error),
    IoError(std::io::Error),
    DatabaseError(rusqlite::Error),
    ImageError(image::ImageError),
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Error {
        Error::HyperError(err)
    }
}

impl From<std::io::Error> for Error {