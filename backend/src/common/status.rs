// use crate::grpc::proto::prelude::JudgeResultState;

// #[repr(i32)]
// pub enum JudgeStatus {
//     AC = 0,
//     NA = 1,
//     WA = 2,
//     CE = 3,
//     RE = 4,
//     RF = 5,
//     TLE = 6,
//     MLE = 7,
//     OLE = 8,
// }

// impl JudgeStatus {
//     pub fn from_i32(i: i32) -> Self {
//         match i {
//             0 => Self::AC,
//             1 => Self::NA,
//             2 => Self::WA,
//             3 => Self::CE,
//             4 => Self::RE,
//             5 => Self::RF,
//             6 => Self::TLE,
//             7 => Self::MLE,
//             _ => Self::OLE,
//         }
//     }
//     pub fn success(&self) -> bool {
//         if let Self::AC = self {
//             true
//         } else {
//             false
//         }
//     }
// }

// impl From<JudgeResultState> for JudgeStatus {
//     fn from(value: JudgeResultState) -> Self {
//         match value {
//             JudgeResultState::Ac => Self::AC,
//             JudgeResultState::Na => Self::NA,
//             JudgeResultState::Wa => Self::WA,
//             JudgeResultState::Ce => Self::CE,
//             JudgeResultState::Re => Self::RE,
//             JudgeResultState::Rf => Self::RF,
//             JudgeResultState::Tle => Self::TLE,
//             JudgeResultState::Mle => Self::MLE,
//             JudgeResultState::Ole => Self::OLE,
//         }
//     }
// }
