// Runtime wrapper module with tokio version selection

#[cfg(all(feature = "tt_02", feature = "tt_1"))]
compile_error!("Features \"tt_02\" and \"tt_1\" cannot be enabled at the same time.");

#[cfg(not(any(feature = "tt_02", feature = "tt_1")))]
compile_error!("Either feature \"tt_02\" or \"tt_1\" must be enabled for Tarantool support.");

#[cfg(feature = "tt_02")]
mod tokio_0_2;
#[cfg(feature = "tt_02")]
pub use tokio_0_2::RuntimeWrapper;

#[cfg(feature = "tt_1")]
mod tokio_1;
#[cfg(feature = "tt_1")]
pub use tokio_1::RuntimeWrapper;
