#[allow(clippy::all, non_local_definitions)]
mod judger {
    tonic::include_proto!("oj.judger");
}

pub use judger::*;

impl std::hash::Hash for LangInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.lang_uid.hash(state);
    }
}

impl PartialOrd for LangInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.lang_uid.cmp(&other.lang_uid))
    }
}

impl Ord for LangInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.lang_uid.cmp(&other.lang_uid)
    }
}

impl Eq for LangInfo {}
