use std::hash::Hash;

pub mod judger {
    tonic::include_proto!("oj.judger");
}
pub mod backend {
    tonic::include_proto!("oj.backend");
}

impl Default for self::judger::judge_response::Task {
    fn default() -> Self {
        log::warn!("judge_response::Task::default() is call becuase oj.judger is outdated");
        Self::Case(0)
    }
}

pub fn into_prost(time: chrono::NaiveDateTime) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: time.timestamp(),
        nanos: time.timestamp_subsec_nanos() as i32,
    }
}

pub fn into_chrono(time: prost_types::Timestamp) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::from_timestamp_opt(time.seconds, time.nanos as u32).unwrap_or_default()
}

pub type TonicStream<T> =
    std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<T, tonic::Status>> + Send>>;

impl Hash for judger::LangInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.lang_uid.hash(state);
    }
}

impl PartialOrd for judger::LangInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.lang_uid.partial_cmp(&other.lang_uid)
    }
}

impl Ord for judger::LangInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.lang_uid.cmp(&other.lang_uid)
    }
}

impl Eq for judger::LangInfo {}
