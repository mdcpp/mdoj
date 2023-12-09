use opentelemetry::{global, metrics::UpDownCounter};

use crate::init::logger::PACKAGE_NAME;

pub struct MetricsController {
    pub user: UpDownCounter<i64>,
    pub submit: UpDownCounter<i64>,
    pub education: UpDownCounter<i64>,
    pub contest: UpDownCounter<i64>,
}

impl MetricsController {
    pub fn new() -> Self {
        Self {
            user: global::meter(PACKAGE_NAME)
                .i64_up_down_counter("user_counts")
                .init(),
            submit: global::meter(PACKAGE_NAME)
                .i64_up_down_counter("submit_counts")
                .init(),
            education: global::meter(PACKAGE_NAME)
                .i64_up_down_counter("education_counts")
                .init(),
            contest: global::meter(PACKAGE_NAME)
                .i64_up_down_counter("contest_counts")
                .init(),
        }
    }
}
