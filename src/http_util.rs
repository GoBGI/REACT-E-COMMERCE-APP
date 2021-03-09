use std::collections::{BTreeMap, HashMap};

use hyper::header::ToStrError;
use hyper::HeaderMap;

pub fn parse_cookies(headers: &HeaderMap) -> Result<HashMap<String, String>, ToStrError> {
    let mut cookies: HashMap<String, String> = HashMap::new();

    if let Some(cookie_header) = headers.get("Cookie") {
        match cookie_header.to_str() {
            Ok(cookie_headers) => {
                for c in cookie_headers.split(';') {
                    let mut parts = c.split('=');
                    cookies.insert(
                        parts.next().unwrap().to_string(),
                        parts.next().unwrap_or_default().to_string(),
                    );
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(cookies)
}

#[derive(Debug)]
pub struct HttpQuery {
    value: BTreeMap<String, String>,
}

impl HttpQuery {
    pub fn from(s: &str) -> HttpQuery {
        let mut que