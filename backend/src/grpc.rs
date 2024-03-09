use std::hash::Hash;

pub mod judger {
    tonic::include_proto!("oj.judger");
}
pub mod backend {
    tonic::include_proto!("oj.backend");
}
/// convert chrono's time to prost_types's
pub fn into_prost(time: chrono::NaiveDateTime) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: time.and_utc().timestamp(),
        nanos: time.timestamp_subsec_nanos() as i32,
    }
}
/// convert prost_types's time to chrono's
pub fn into_chrono(time: prost_types::Timestamp) -> chrono::NaiveDateTime {
    match chrono::DateTime::from_timestamp(time.seconds, time.nanos as u32) {
        Some(x) => x.naive_utc(),
        None => chrono::NaiveDateTime::default(),
    }
}
/// server side stream in tonic
pub type TonicStream<T> =
    std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<T, tonic::Status>> + Send>>;

impl Hash for judger::LangInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.lang_uid.hash(state);
    }
}

impl PartialOrd for judger::LangInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.lang_uid.cmp(&other.lang_uid))
    }
}

impl Ord for judger::LangInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.lang_uid.cmp(&other.lang_uid)
    }
}

impl Eq for judger::LangInfo {}
