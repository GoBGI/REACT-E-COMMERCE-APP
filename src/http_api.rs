
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
}

static OK: &[u8] = b"OK";
static BAD_REQUEST: &[u8] = b"Bad Request";
static UNAUTHORIZED: &[u8] = b"Unauthorized";
static NOT_FOUND: &[u8] = b"Not Found";
static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";

fn bad_request() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(BAD_REQUEST.into())
        .unwrap()
}

fn unauthorized() -> Response<Body> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(UNAUTHORIZED.into())
        .unwrap()
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(NOT_FOUND.into())
        .unwrap()
}

fn server_error() -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(INTERNAL_SERVER_ERROR.into())
        .unwrap()
}

fn json_ok(json: &str) -> Response<Body> {
    Response::builder()
        .header("Content-Type", "application/json; charset=utf8")
        .body(json.to_string().into())
        .unwrap()
}

struct ApiRequest {
    request: Request<Body>,
    musicd: Arc<Musicd>,
    query: HttpQuery,
    cookies: HashMap<String, String>,
}

async fn process_request(
    request: Request<Body>,
    musicd: Arc<Musicd>,
) -> Result<Response<Body>, hyper::Error> {
    debug!("request {}", request.uri());

    let query = HttpQuery::from(request.uri().query().unwrap_or_default());

    let cookies = match crate::http_util::parse_cookies(request.headers()) {
        Ok(c) => c,
        Err(e) => {
            debug!("invalid cookies {}", e);
            return Ok(bad_request());
        }
    };

    let api_request = ApiRequest {
        request,
        musicd,
        query,
        cookies,
    };

    let result = match (
        api_request.request.method(),
        api_request.request.uri().path(),
    ) {
        (&Method::GET, "/api/musicd") => Some(api_musicd(&api_request)),
        (&Method::GET, "/api/auth") => Some(api_auth(&api_request)),
        _ => None,
    };

    if let Some(result) = result {
        return match result {
            Ok(res) => Ok(res),
            Err(_e) => Ok(server_error()),
        };
    }

    if let Some(auth_password) = api_request.cookies.get("musicd2-auth") {
        if !api_request.musicd.password.is_empty() && api_request.musicd.password != *auth_password
        {
            debug!("invalid auth");
            return Ok(unauthorized());
        }
    }

    let result = match (
        api_request.request.method(),
        api_request.request.uri().path(),
    ) {
        (&Method::GET, "/api/audio_stream") => api_audio_stream(&api_request),
        (&Method::GET, "/api/image_file") => api_image_file(&api_request),
        (&Method::GET, "/api/track_lyrics") => api_track_lyrics(&api_request).await,
        (&Method::GET, "/api/nodes") => api_nodes(&api_request),
        (&Method::GET, "/api/tracks") => api_tracks(&api_request),
        (&Method::GET, "/api/artists") => api_artists(&api_request),
        (&Method::GET, "/api/albums") => api_albums(&api_request),
        (&Method::GET, "/api/images") => api_images(&api_request),
        (&Method::GET, "/api/scan") => api_scan(&api_request),
        (&Method::POST, "/api/scan") => api_scan(&api_request),
        (&Method::GET, "/share") => res_share(&api_request),
        _ => Ok(not_found()),
    };

    match result {
        Ok(res) => Ok(res),
        Err(_e) => Ok(server_error()),
    }
}