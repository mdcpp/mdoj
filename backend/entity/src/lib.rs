pub mod announcement;
pub mod chat;
pub mod contest;
pub mod education;
pub mod problem;
pub mod submit;
pub mod test;
pub mod token;
pub mod user;
pub mod user_contest;

pub trait DebugName{
    const DEBUG_NAME: &'static str = "TEMPLATE_DEBUG_NAME";
}
