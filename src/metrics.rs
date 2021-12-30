use chrono::prelude::*;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Label {
    pub name: String,
    pub value: String
}

impl Label {
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metric {
    pub value: f64,
    pub timestamp: i64,
    pub labels: Vec<Label>
}

impl Metric {
    pub fn from_str(metric_str: &str) -> Option<Metric> {
        if metric_str.starts_with("#") {
            return None;
        }
        if metric_str.contains("{") || metric_str.contains("}") {
            return parse_metric_with_labels(metric_str)
        }
        parse_metric_without_labels(metric_str)
    }

    pub fn name(&self) -> &str {
        let label_name = self.labels.iter()
        .find(|label| label.name == "__name__")
        .unwrap();

        &label_name.value
    }
}

fn parse_metric_with_labels(metric_str: &str) -> Option<Metric> {
    let metric_name_regex = Regex::new(r"^(\w+)").unwrap();
    let s = r#"(\w+)="([/\w\d\.*:*]*)""#;
    let label_name_regex = Regex::new(s).unwrap();
    let metric_value_regex = Regex::new(r"(\d+)$").unwrap();

    let mut metric_name_capture = metric_name_regex.captures_iter(metric_str);
    let metric_value_capture = metric_value_regex.captures_iter(metric_str).next();

    let metric_name = metric_name_capture.next().unwrap()[0].to_string();
    let metric_value = metric_value_capture.unwrap()[0].parse::<f64>().unwrap();

    let mut labels = vec![
        Label::new("__name__", &metric_name)
    ];
    for label_match in label_name_regex.captures_iter(metric_str) {
        labels.push(Label::new(&label_match[1], &label_match[2]));
    }

    let utc: DateTime<Utc> = Utc::now();
    Some(Metric {
        value: metric_value,
        timestamp: utc.timestamp(),
        labels: labels
    })
}

fn parse_metric_without_labels(metric_str: &str) -> Option<Metric> {
    let mut iter = metric_str.split_whitespace();
    let metric_name = iter.next();
    let metric_value_str = iter.next();

    if metric_name.is_none() || metric_value_str.is_none() {
        return None;
    }

    let parsed_metric_value = metric_value_str.unwrap().parse::<f64>();
    if parsed_metric_value.is_err() {
        return None;
    }

    let labels = vec![
        Label::new("__name__", metric_name.unwrap())
    ];

    let utc: DateTime<Utc> = Utc::now();

    Some(Metric {
        value: parsed_metric_value.unwrap(),
        timestamp: utc.timestamp(),
        labels: labels
    })
}

impl Into<(i64, f64)> for Metric {
    fn into(self) -> (i64, f64) {
        (self.timestamp, self.value)
    }
}

#[cfg(test)]
mod tests {
    use crate::metrics::Label;

    use super::Metric;
    const METRIC_WITHOUT_LABEL: &'static str = "promhttp_metric_handler_errors_total 0";
    const METRIC_WITH_LABEL: &'static str = "promhttp_metric_handler_errors_total{cause=\"encoding\"} 0";

    #[test]
    fn test_metric_without_labels_converts_name() {
        let metric = Metric::from_str(METRIC_WITHOUT_LABEL).unwrap();
        assert_eq!(0.0, metric.value);
    }

    #[test]
    fn test_metric_without_labels_converts_value() {
        let metric = Metric::from_str(METRIC_WITHOUT_LABEL).unwrap();
        assert_eq!("promhttp_metric_handler_errors_total", metric.name());
    }

    #[test]
    fn test_metric_with_labels_converts_value() {
        let metric = Metric::from_str(METRIC_WITH_LABEL).unwrap();
        assert_eq!(0.0, metric.value);
    }


    #[test]
    fn test_metric_with_labels_converts_name() {
        let metric = Metric::from_str(METRIC_WITH_LABEL).unwrap();
        assert_eq!("promhttp_metric_handler_errors_total", metric.name());
    }

    #[test]
    fn test_metric_with_labels_converts_labels() {
        let metric = Metric::from_str(METRIC_WITH_LABEL).unwrap();
        let labels = metric.labels;
        let labels: &Vec<Label> = labels.as_ref();
        assert_eq!(labels[1].name, "cause");
        assert_eq!(labels[1].value, "encoding");
    }
}