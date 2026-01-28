// Runtime wrapper module with tokio version selection

#[cfg(all(feature = "tt_2", feature = "tt_3"))]
compile_error!("Features \"tt_2\" and \"tt_3\" cannot be enabled at the same time.");

#[cfg(not(any(feature = "tt_2", feature = "tt_3")))]
compile_error!("Either feature \"tt_2\" or \"tt_3\" must be enabled for Tarantool support.");

#[cfg(feature = "tt_2")]
mod tokio_0_2;
#[cfg(feature = "tt_2")]
pub use tokio_0_2::RuntimeWrapper;

#[cfg(feature = "tt_3")]
mod tokio_1;
#[cfg(feature = "tt_3")]
pub use tokio_1::RuntimeWrapper;
