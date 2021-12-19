use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum Command {
    Store(String),
    Query(String),
    QueryResults(Vec<(i64, f64)>)
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Store(metric) => {
                write!(f, "STORE {}", metric)
            },
            Command::Query(query) => {
                write!(f, "QUERY {}", query)
            },
            Command::QueryResults(metrics) => {
                write!(f, "QUERY RESULTS {:?}", metrics)
            },
        }
    }
}