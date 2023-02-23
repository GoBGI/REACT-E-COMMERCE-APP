
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

    pub fn filter_values(&mut self, clause: &str, value: Vec<Box<dyn ToSql>>) {
        self.clauses.push(clause.to_string());

        for v in value {
            self.values.push(Box::new(v));
        }
    }

    pub fn bind_filter_i64(&mut self, query: &HttpQuery, key: &str, clause: &str) {
        if let Some(value) = query.get_i64(key) {
            self.filter_value(clause, value);
        }
    }

    pub fn bind_filter_str(&mut self, query: &HttpQuery, key: &str, clause: &str) {
        if let Some(value) = query.get_str(key) {
            self.filter_value(clause, value.to_string());
        }
    }

    pub fn order_string(&mut self, order_string: &str) {
        self.order_string = Some(order_string.to_string());
    }

    pub fn limit(&mut self, limit: i64) {
        self.limit = Some(limit);
    }

    pub fn offset(&mut self, offset: i64) {
        self.offset = Some(offset);
    }

    pub fn bind_range(&mut self, query: &HttpQuery) {
        if let Some(limit) = query.get_i64("limit") {
            self.limit(limit)
        }

        if let Some(offset) = query.get_i64("offset") {
            self.offset(offset)
        }
    }

    pub fn get_total(&self, conn: &Connection, select_from: &str) -> Result<i64, rusqlite::Error> {
        let mut sql = select_from.to_string();

        if !self.clauses.is_empty() {
            sql += " WHERE ";
            sql += &self.clauses.join(" AND ");
        }

        let mut st = conn.prepare(&sql)?;

        Ok(st.query_row(&self.values, |row| row.get(0))?)