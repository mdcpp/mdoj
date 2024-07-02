#[cfg(feature = "backend")]
mod ids;

#[cfg(feature = "backend")]
#[cfg(feature = "judger")]
mod bridge;

#[cfg(feature = "backend")]
#[allow(clippy::all, non_local_definitions)]
pub mod backend;

#[cfg(feature = "judger")]
pub mod judger;

#[cfg(not(any(feature = "backend", feature = "judger")))]
compile_error!("At least one of the features `backend` and `judger` must be enabled");