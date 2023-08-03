pub mod prelude {
    tonic::include_proto!("oj.judger");
}

use std::fmt::Display;

use crate::init::config::CONFIG;

// #[async_trait::async_trait]
// pub trait Queryable {
//     async fn get() -> Self;
// }

// #[async_trait::async_trait]
// pub trait Listable<T = Self> {
//     async fn list() -> Vec<T>;
// }

// #[async_trait::async_trait]
// impl Queryable for prelude::JudgeInfo {
//     async fn get() -> Self {
//         let config = CONFIG.get().unwrap();

//         let accuracy: i64 = match config.kernel.tickless {
//             true => 1000 * 1000 * 1000 / (config.kernel.USER_HZ as i64),
//             false => 0,
//         };

//         Self {
//             memory: config.platform.available_memory,
//             accuracy,
//             langs: Some(prelude::Langs {
//                 list: prelude::LangInfo::list().await,
//             }),
//             cpu_factor: config.platform.cpu_time_multiplier as f32,
//         }
//     }
// }

// #[async_trait::async_trait]
// impl<T> Listable<T> for prelude::LangInfo {
//     async fn list() -> Vec<T> {
//         todo!()
//     }
// }

impl Display for prelude::JudgeResultState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            prelude::JudgeResultState::Ac => "Accepted",
            prelude::JudgeResultState::Na => "Unknown",
            prelude::JudgeResultState::Wa => "Wrong Answer",
            prelude::JudgeResultState::Ce => "Compile Error",
            prelude::JudgeResultState::Re => "Runtime Error",
            prelude::JudgeResultState::Rf => "idk",
            prelude::JudgeResultState::Tle => "Time Limit Excess",
            prelude::JudgeResultState::Mle => "Memory Limti Excess",
            prelude::JudgeResultState::Ole => "Output Limit Excess",
        };
        write!(f, "{}", message)
    }
}
