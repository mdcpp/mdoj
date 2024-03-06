use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_queue::SegQueue;
use opentelemetry::{
    global,
    metrics::{MeterProvider, ObservableCounter, ObservableGauge, UpDownCounter},
};
use opentelemetry_sdk::metrics::MeterProvider as SdkMeterProvider;

use crate::init::logger::PACKAGE_NAME;

macro_rules! impl_metrics {
    ($n:expr) => {
        paste::paste!{
            impl MetricsController {
                pub fn $n(&self,val:i64){
                    self.[<$n>].add(val,&[]);
                }
            }
        }
    };
    ($target:expr,$($ext:expr),+) => {
        impl_metrics!($target);
        impl_metrics!($($ext),+);
    };
}

/// collection of statful metrics
///
/// because metrics(opentelemetry) sdk is not yet GA,
/// stateful metrics is necessary in state of art.
pub struct MetricsController {
    user: UpDownCounter<i64>,
    submit: UpDownCounter<i64>,
    education: UpDownCounter<i64>,
    contest: UpDownCounter<i64>,
    chat: UpDownCounter<i64>,
    image: ObservableCounter<u64>,
}

impl MetricsController {
    pub fn new(meter: &SdkMeterProvider) -> Self {
        let package_meter = meter.meter(PACKAGE_NAME);

        Self {
            user: package_meter.i64_up_down_counter("counts_user").init(),
            submit: package_meter.i64_up_down_counter("counts_submit").init(),
            education: package_meter.i64_up_down_counter("counts_education").init(),
            contest: package_meter.i64_up_down_counter("counts_contest").init(),
            chat: package_meter.i64_up_down_counter("counts_chat").init(),
            image: package_meter.u64_observable_counter("counts_image").init(),
        }
    }
    pub fn image(&self, val: u64) {
        self.image.observe(val, &[]);
    }
}
impl_metrics!(user, submit, education, contest, chat);

/// because metrics(opentelemetry) sdk is not yet GA,
/// rate metrics(feature) is missing and we implement manually through [`ObservableGauge<f64>`]
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
