#[cfg(feature = "judger")]
#[allow(clippy::all, non_local_definitions)]
pub mod judger {
    tonic::include_proto!("oj.judger");
}

pub use judger::*;

impl std::hash::Hash for judger::LangInfo {
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
