#[macro_use]
extern crate log;

pub mod common;
pub mod memory_storage;
pub mod lmdb_storage;
pub mod remote_storage_client;
pub mod vstorage;
#[cfg(any(feature = "tt_2", feature = "tt_3"))]
pub mod tt_storage;
#[cfg(any(feature = "tt_2", feature = "tt_3"))]
pub mod tt_wrapper;
#[cfg(any(feature = "tokio_0_2", feature = "tokio_1"))]
pub mod runtime_wrapper;
pub mod storage_factory;

// Re-export main types
pub use common::{Storage, StorageId, StorageMode, StorageResult, StorageDispatcher};
pub use memory_storage::MemoryStorage;
pub use lmdb_storage::LMDBStorage;
pub use remote_storage_client::StorageROClient;
pub use vstorage::{VStorage, VStorageGeneric, VStorageEnum, VMemoryStorage, VLMDBStorage, VRemoteStorage};
#[cfg(any(feature = "tt_2", feature = "tt_3"))]
pub use tt_storage::TTStorage;
#[cfg(any(feature = "tt_2", feature = "tt_3"))]
pub use vstorage::VTTStorage;
pub use storage_factory::{StorageBuilder, StorageConfig, StorageError, StorageFactory, StorageProvider, DefaultStorageFactory};
#[cfg(feature = "tokio_0_2")]
pub use runtime_wrapper::RuntimeWrapper;
#[cfg(feature = "tokio_1")]
pub use runtime_wrapper::RuntimeWrapper;

// Re-export for backward compatibility - удалено для полной унификации
// #[deprecated(since = "0.1.0", note = "Use common::StorageResult instead")]
// pub use v_result_code::ResultCode;
