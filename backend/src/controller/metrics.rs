use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_queue::SegQueue;
use opentelemetry::{
    global,
    metrics::{MeterProvider, ObservableGauge, UpDownCounter},
};
use opentelemetry_sdk::metrics::MeterProvider as SdkMeterProvider;

use crate::init::logger::PACKAGE_NAME;

pub struct MetricsController {
    pub user: UpDownCounter<i64>,
    pub submit: UpDownCounter<i64>,
    pub education: UpDownCounter<i64>,
    pub contest: UpDownCounter<i64>,
}

impl MetricsController {
    pub fn new(meter: &SdkMeterProvider) -> Self {
        Self {
            user: meter
                .meter(PACKAGE_NAME)
                .i64_up_down_counter("counts_user")
                .init(),
            submit: meter
                .meter(PACKAGE_NAME)
                .i64_up_down_counter("counts_submit")
                .init(),
            education: meter
                .meter(PACKAGE_NAME)
                .i64_up_down_counter("counts_education")
                .init(),
            contest: meter
                .meter(PACKAGE_NAME)
                .i64_up_down_counter("counts_contest")
                .init(),
        }
    }
}

pub struct RateMetrics<const S: usize> {
    meter: ObservableGauge<f64>,
    record: SegQueue<bool>,
    sets: AtomicUsize,
}

impl<const S: usize> RateMetrics<S> {
    pub fn new(name: &'static str) -> Self {
        let record = SegQueue::new();
        for _ in 0..S {
            record.push(true);
        }
        Self {
            meter: global::meter(PACKAGE_NAME)
                .f64_observable_gauge(name)
                .init(),
            record,
            sets: AtomicUsize::new(S),
        }
    }
    pub fn set(&self) {
        self.record.push(true);
        if !self.record.pop().unwrap() {
            let sets = self.sets.fetch_sub(1, Ordering::Acquire);
            self.meter.observe((sets as f64) / (S as f64), &[])
        }
    }
    pub fn unset(&self) {
        self.record.push(false);
        if self.record.pop().unwrap() {
            let sets = self.sets.fetch_add(1, Ordering::Acquire);
            self.meter.observe((sets as f64) / (S as f64), &[])
        }
    }
}