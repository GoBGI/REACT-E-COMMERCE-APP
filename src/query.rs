
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

use rusqlite::types::ToSql;
use rusqlite::{Connection, Statement};
use serde::Serialize;

use crate::http_util::HttpQuery;
use crate::index::{Index, NodeType};

struct QueryOptions {
    clauses: Vec<String>,
    values: Vec<Box<dyn ToSql>>,
    order_string: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl QueryOptions {
    pub fn new() -> QueryOptions {
        QueryOptions {
            clauses: Vec::new(),
            values: Vec::new(),
            order_string: None,
            limit: None,
            offset: None,
        }
    }

    pub fn filter(&mut self, clause: &str) {
        self.clauses.push(clause.to_string());
    }

    pub fn filter_value<T>(&mut self, clause: &str, value: T)
    where
        T: ToSql,
        T: 'static,
    {
        self.clauses.push(clause.to_string());
        self.values.push(Box::new(value));
    }
