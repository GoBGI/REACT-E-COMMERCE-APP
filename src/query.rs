
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
    }

    pub fn into_items_query<'a>(
        mut self,
        conn: &'a Connection,
        select_from: &str,
    ) -> Result<(Statement<'a>, Vec<Box<dyn ToSql>>), rusqlite::Error> {
        let mut sql = select_from.to_string();

        if !self.clauses.is_empty() {
            sql += " WHERE ";
            sql += &self.clauses.join(" AND ");
        }

        if let Some(order) = self.order_string {
            sql += " ORDER BY ";
            sql += &order;
        }

        if let Some(limit) = self.limit {
            sql += " LIMIT ?";
            self.values.push(Box::new(limit));
        }

        if let Some(offset) = self.offset {
            sql += " OFFSET ?";
            self.values.push(Box::new(offset));
        }

        let st = conn.prepare(&sql)?;

        Ok((st, self.values))
    }
}

#[derive(Serialize)]
pub struct NodeItem {
    node_id: i64,
    parent_id: Option<i64>,
    node_type: NodeType,
    name: String,
    path: String,
    track_count: i64,
    image_count: i64,
    all_track_count: i64,
    all_image_count: i64,
}

pub fn query_nodes(
    index: &Index,
    query: &HttpQuery,
) -> Result<(i64, Vec<NodeItem>), rusqlite::Error> {
    let mut opts = QueryOptions::new();

    let mut parent_id_filter = false;

    if let Some(parent_id) = query.get_str("parent_id") {
        if let Ok(parent_id) = parent_id.parse::<i64>() {
            opts.filter_value("Node.parent_id = ?", parent_id);
            parent_id_filter = true;
        } else if parent_id == "null" {
            opts.filter("Node.parent_id IS NULL");
            parent_id_filter = true;
        }
    }

    opts.bind_range(&query);

    let conn = index.connection();

    let total = opts.get_total(&conn, "SELECT COUNT(Node.node_id) FROM Node")?;

    let (mut st, values) = opts.into_items_query(
        &conn,
        if parent_id_filter {
            "SELECT
                Node.node_id,
                Node.parent_id,
                Node.node_type,
                Node.name,
                Node.path,

                (
                    SELECT COUNT(track_id)
                    FROM Track
                    INNER JOIN Node track_node ON track_node.node_id = Track.node_id
                    WHERE track_node.parent_id = Node.node_id
                ) AS track_count,
                (
                    SELECT COUNT(image_id)
                    FROM Image
                    INNER JOIN Node image_node ON image_node.node_id = Image.node_id
                    WHERE image_node.parent_id = Node.node_id
                ) AS image_count,

                (
                    SELECT COUNT(track_id)
                    FROM Node AS child_node
                    INNER JOIN Track ON Track.node_id = child_node.node_id
                    WHERE child_node.path LIKE Node.path || '/%'
                ) AS all_track_count,
                (
                    SELECT COUNT(image_id)
                    FROM Node AS child_node
                    INNER JOIN Image ON Image.node_id = child_node.node_id
                    WHERE child_node.path LIKE Node.path || '/%'
                ) AS all_image_count

            FROM Node"
        } else {
            "SELECT
                Node.node_id,