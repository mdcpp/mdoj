// use crate::backend::{self, playground_result, PlaygroundResult};
// use crate::judger::{self, exec_result, ExecResult};

// impl From<ExecResult> for PlaygroundResult {
//     fn from(value: ExecResult) -> Self {
//         PlaygroundResult {
//             result: Some(match value.result.unwrap() {
//                 exec_result::Result::Output(x) => playground_result::Result::Output(x),
//                 exec_result::Result::Log(x) => playground_result::Result::Compile(x.into()),
//             }),
//         }
//     }
// }
// impl From<judger::Log> for backend::Log {
//     fn from(value: judger::Log) -> Self {
//         backend::Log {
//             level: value.level,
//             msg: value.msg,
//         }
//     }
// }

// impl From<judger::LangInfo> for backend::Language {
//     fn from(value: judger::LangInfo) -> Self {
//         backend::Language {
//             lang_uid: value.lang_uid,
//             lang_name: value.lang_name,
//             info: value.info,
//             lang_ext: value.lang_ext,
//         }
//     }
// }
