use std::collections::{BTreeMap, HashMap};

use hyper::header::ToStrError;
use hyper::HeaderMap;

pub fn parse_cookies(headers: &HeaderMap) -> Result<HashMap<String, String>, ToStrError> {
    let mut cookies: HashMap<String, String> = HashMap::new();

    if let So