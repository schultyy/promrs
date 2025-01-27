use std::collections::HashMap;

use tracing::instrument;

use crate::{metrics::Metric, storage_error::StorageError};

#[derive(Debug)]
pub struct Storage {
    metrics: HashMap<String, Vec<(i64, f64)>>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::default(),
        }
    }

    #[instrument(skip(self))]
    pub fn store(&mut self, metric_str: String) -> Result<(), StorageError> {
        if let Some(metric) = Metric::from_str(&metric_str) {
            if self.metrics.contains_key(&metric.name) {
                let time_series = self.metrics.get_mut(&metric.name).unwrap();
                time_series.push(metric.into());
            } else {
                self.metrics
                    .insert(metric.name.to_string(), vec![metric.into()]);
            }
            Ok(())
        } else {
            Err(StorageError::new(
                "Could not parse Metric".to_string(),
                metric_str,
            ))
        }
    }

    #[instrument]
    pub(crate) fn query(&self, query: String) -> Vec<(i64, f64)> {
        if let Some((_, time_series)) = self.metrics.get_key_value(&query) {
            time_series.to_owned()
        } else {
            vec![]
        }
    }
}
