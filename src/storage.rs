use std::{error::Error, fmt::Display};

use crate::metrics::Metric;

#[derive(Debug)]
pub struct StorageError {
    pub message: String,
    pub metric: String,
}

impl StorageError {
    pub fn new(message: String, metric: String) -> Self {
        Self {
            message: message,
            metric: metric
        }
    }
}

impl Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for StorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        &self.message
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

pub struct Storage {
    metrics: Vec<Metric>
}

impl Storage {
    pub fn new() -> Self {
        Self {
            metrics: vec!()
        }
    }

    pub fn store(&mut self, metric_str: String) -> Result<(), StorageError> {
        if let Some(metric) = Metric::from_str(&metric_str) {
            self.metrics.push(metric);
            Ok(())
        }
        else {
            Err(StorageError::new("Could not parse Metric".to_string(), metric_str))
        }
    }
}