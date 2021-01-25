
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
    fn from(err: std::io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Error {
        Error::DatabaseError(err)
    }
}

impl From<image::ImageError> for Error {
    fn from(err: image::ImageError) -> Error {
        Error::ImageError(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::HyperError(ref e) => e.description(),
            Error::IoError(ref e) => e.description(),
            Error::DatabaseError(ref e) => e.description(),
            Error::ImageError(ref e) => e.description(),
        }
    }
}

pub async fn run_api(musicd: Arc<crate::Musicd>, bind: SocketAddr) {
    let make_service = make_service_fn(move |_socket: &AddrStream| {
        let musicd = musicd.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                process_request(req, musicd.clone())
            }))
        }
    });

    info!("listening on {}", bind);

    Server::bind(&bind)
        .serve(make_service)
        .await
        .expect("running server failed");