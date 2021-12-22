use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum Command {
    Store(Vec<String>),
    Query(String),
    QueryResults(Vec<(i64, f64)>)
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Store(metrics) => {
                write!(f, "STORE {} metrics", metrics.len())
            },
            Command::Query(query) => {
                write!(f, "QUERY {}", query)
            },
            Command::QueryResults(metrics) => {
                write!(f, "QUERY RESULTS - Returning {} metrics", metrics.len())
            },
        }
    }
}