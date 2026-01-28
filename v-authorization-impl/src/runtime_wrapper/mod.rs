// Runtime wrapper module with tokio version selection

#[cfg(all(feature = "az_tt_2", feature = "az_tt_3"))]
compile_error!("Features \"tt_2\" and \"tt_3\" cannot be enabled at the same time.");

#[cfg(feature = "az_tt_2")]
mod tokio_0_2;
#[cfg(feature = "az_tt_2")]
pub use tokio_0_2::RuntimeWrapper;

#[cfg(feature = "az_tt_3")]
mod tokio_1;
#[cfg(feature = "az_tt_3")]
pub use tokio_1::RuntimeWrapper;
