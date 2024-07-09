#[cfg(feature = "backend")]
#[cfg(feature = "judger")]
mod bridge;

#[cfg(feature = "backend")]
pub mod backend;

#[cfg(feature = "judger")]
pub mod judger;

#[cfg(not(any(feature = "backend", feature = "judger")))]
compile_error!("At least one of the features `backend` and `judger` must be enabled");
