// src/runtime_wrapper/mod.rs

// Убедимся, что включена ровно одна фича
#[cfg(not(any(feature = "tokio-0_2", feature = "tokio-1")))]
compile_error!("Either feature \"tokio-0_2\" or \"tokio-1\" must be enabled for this crate.");

#[cfg(all(feature = "tokio-0_2", feature = "tokio-1"))]
compile_error!("Features \"tokio-0_2\" and \"tokio-1\" cannot be enabled at the same time.");

#[cfg(feature = "tokio-0_2")]
pub mod tokio_0_2;
#[cfg(feature = "tokio-0_2")]
pub use tokio_0_2::RuntimeWrapper;

#[cfg(feature = "tokio-1")]
pub mod tokio_1;
#[cfg(feature = "tokio-1")]
pub use tokio_1::RuntimeWrapper;
