use std::{error::Error, fmt::Display};

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