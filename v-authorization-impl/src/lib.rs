#[macro_use]
extern crate log;

// Ensure exactly one backend is selected
#[cfg(all(feature = "lmdb", any(feature = "tt_2", feature = "tt_3")))]
compile_error!("Features \"lmdb\" and \"tt_2/tt_3\" cannot be enabled at the same time. Choose one backend.");

#[cfg(not(any(feature = "lmdb", feature = "tt_2", feature = "tt_3")))]
compile_error!("Either feature \"lmdb\" or \"tt_2/tt_3\" must be enabled. Choose one backend.");

#[cfg(feature = "lmdb")]
pub mod az_lmdb;
#[cfg(any(feature = "tt_2", feature = "tt_3"))]
pub mod az_tarantool;
#[cfg(any(feature = "tt_2", feature = "tt_3"))]
mod runtime_wrapper;
pub mod az_context;
mod stat_manager;

#[cfg(feature = "lmdb")]
pub use az_lmdb::LmdbAzContext;
#[cfg(any(feature = "tt_2", feature = "tt_3"))]
pub use az_tarantool::TarantoolAzContext;
pub use az_context::AzContext;
pub use v_authorization;

