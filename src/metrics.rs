#[derive(Debug)]
pub struct Metric {
    pub name: String,
    pub value: f64,
}

impl Metric {
    pub fn from_str(metric_str: &str) -> Option<Metric> {
        if metric_str.contains("{") || metric_str.contains("}") {
            return None;
        }

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

        Some(Metric {
            name: metric_name.unwrap().to_string(),
            value: parsed_metric_value.unwrap(),
        })
    }
}
