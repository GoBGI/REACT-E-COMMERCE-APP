use std::collections::{BTreeMap, HashMap};

use hyper::header::ToStrError;
use hyper::HeaderMap;

pub fn parse_cookies(headers: &HeaderMap) -> Result<HashMap<String, String>, ToStrError> {
    let mut cookies: HashMap<String, String> = HashMap::new();

    if let Some(cookie_header) = headers.get("Cookie") {
        match cookie_header.to_str() {
            Ok(cookie_headers) => {
                for c in cookie_headers.split(';') {
                    let mut parts = c.split(