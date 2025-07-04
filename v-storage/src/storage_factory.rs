use crate::common::{Storage, StorageMode};
use std::fmt;

/// Абстрактная фабрика для создания различных типов хранилищ
pub trait StorageFactory {
    fn create_storage(&self) -> Result<Box<dyn Storage>, StorageError>;
}

/// Ошибки создания хранилищ
#[derive(Debug)]
pub enum StorageError {
    ConnectionFailed(String),
    InvalidConfiguration(String),
    IoError(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            StorageError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            StorageError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

/// Конфигурация для различных типов хранилищ
#[derive(Debug, Clone)]
pub enum StorageConfig {
    Memory,
    Lmdb {
        path: String,
        mode: StorageMode,
        max_read_counter_reopen: Option<u64>,
    },
    Remote {
        address: String,
    },
    #[cfg(any(feature = "tt_2", feature = "tt_3"))]
    Tarantool {
        uri: String,
        login: String,
        password: String,
    },
}

/// Билдер для создания хранилищ через фабрику
pub struct StorageBuilder {
    config: Option<StorageConfig>,
}

impl StorageBuilder {
    pub fn new() -> Self {
        Self { config: None }
    }

    pub fn memory(mut self) -> Self {
        self.config = Some(StorageConfig::Memory);
        self
    }

    pub fn lmdb(mut self, path: &str, mode: StorageMode, max_read_counter_reopen: Option<u64>) -> Self {
        self.config = Some(StorageConfig::Lmdb {
            path: path.to_string(),
            mode,
            max_read_counter_reopen,
        });
        self
    }

    pub fn remote(mut self, address: &str) -> Self {
        self.config = Some(StorageConfig::Remote {
            address: address.to_string(),
        });
        self
    }

    #[cfg(any(feature = "tt_2", feature = "tt_3"))]
    pub fn tarantool(mut self, uri: &str, login: &str, password: &str) -> Self {
        self.config = Some(StorageConfig::Tarantool {
            uri: uri.to_string(),
            login: login.to_string(),
            password: password.to_string(),
        });
        self
    }

    pub fn build(self) -> Result<Box<dyn Storage>, StorageError> {
        let config = self.config.ok_or_else(|| {
            StorageError::InvalidConfiguration("No storage type specified".to_string())
        })?;

        DefaultStorageFactory::new().create_storage_from_config(config)
    }

    // ========================================================================================
    // НОВЫЕ МЕТОДЫ ДЛЯ СОЗДАНИЯ GENERIC ВЕРСИЙ
    // ========================================================================================

    /// Создает generic память хранилище
    pub fn build_memory_generic(self) -> Result<crate::vstorage::VMemoryStorage, StorageError> {
        if let Some(StorageConfig::Memory) = self.config {
            Ok(crate::vstorage::VMemoryStorage::new(crate::memory_storage::MemoryStorage::new()))
        } else {
            Err(StorageError::InvalidConfiguration(
                "Builder is not configured for memory storage".to_string()
            ))
        }
    }

    /// Создает generic LMDB хранилище
    pub fn build_lmdb_generic(self) -> Result<crate::vstorage::VLMDBStorage, StorageError> {
        if let Some(StorageConfig::Lmdb { path, mode, max_read_counter_reopen }) = self.config {
            Ok(crate::vstorage::VLMDBStorage::new(crate::lmdb_storage::LMDBStorage::new(&path, mode, max_read_counter_reopen)))
        } else {
            Err(StorageError::InvalidConfiguration(
                "Builder is not configured for LMDB storage".to_string()
            ))
        }
    }

    /// Создает generic удаленное хранилище
    pub fn build_remote_generic(self) -> Result<crate::vstorage::VRemoteStorage, StorageError> {
        if let Some(StorageConfig::Remote { address }) = self.config {
            Ok(crate::vstorage::VRemoteStorage::new(crate::remote_storage_client::StorageROClient::new(&address)))
        } else {
            Err(StorageError::InvalidConfiguration(
                "Builder is not configured for remote storage".to_string()
            ))
        }
    }

    /// Создает generic Tarantool хранилище
    #[cfg(any(feature = "tt_2", feature = "tt_3"))]
    pub fn build_tarantool_generic(self) -> Result<crate::vstorage::VTTStorage, StorageError> {
        if let Some(StorageConfig::Tarantool { uri, login, password }) = self.config {
            Ok(crate::vstorage::VTTStorage::new(crate::tt_storage::TTStorage::new(uri, &login, &password)))
        } else {
            Err(StorageError::InvalidConfiguration(
                "Builder is not configured for Tarantool storage".to_string()
            ))
        }
    }
}

impl Default for StorageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ========================================================================================
// STORAGE PROVIDER - ФАБРИЧНЫЕ МЕТОДЫ ДЛЯ УДОБНОГО СОЗДАНИЯ
// ========================================================================================

/// Провайдер хранилищ - централизованное место для создания всех типов хранилищ
/// 
/// Ответственности:
/// - Создание конкретных экземпляров хранилищ
/// - Логирование процесса создания
/// - Конфигурация параметров по умолчанию
pub struct StorageProvider;

impl StorageProvider {
    /// Создает новое хранилище в памяти (dynamic dispatch)
    pub fn memory() -> Box<dyn Storage> {
        log::info!("Creating in-memory storage");
        Box::new(crate::memory_storage::MemoryStorage::new())
    }

    /// Создает новое LMDB хранилище (dynamic dispatch)
    pub fn lmdb(db_path: &str, mode: StorageMode, max_read_counter_reopen: Option<u64>) -> Box<dyn Storage> {
        log::info!("Trying to connect to [LMDB], path: {}", db_path);
        Box::new(crate::lmdb_storage::LMDBStorage::new(db_path, mode, max_read_counter_reopen))
    }

    /// Создает новое удаленное хранилище (dynamic dispatch)
    pub fn remote(addr: &str) -> Box<dyn Storage> {
        log::info!("Trying to connect to [remote], addr: {}", addr);
        Box::new(crate::remote_storage_client::StorageROClient::new(addr))
    }

    /// Создает новое Tarantool хранилище (dynamic dispatch)
    #[cfg(any(feature = "tt_2", feature = "tt_3"))]
    pub fn tarantool(tt_uri: String, login: &str, pass: &str) -> Box<dyn Storage> {
        log::info!("Trying to connect to [Tarantool], addr: {}", tt_uri);
        Box::new(crate::tt_storage::TTStorage::new(tt_uri, login, pass))
    }

    /// Создает VStorage с памятью
    pub fn vstorage_memory() -> crate::vstorage::VStorage {
        crate::vstorage::VStorage::new(Self::memory())
    }

    /// Создает VStorage с LMDB
    pub fn vstorage_lmdb(db_path: &str, mode: StorageMode, max_read_counter_reopen: Option<u64>) -> crate::vstorage::VStorage {
        crate::vstorage::VStorage::new(Self::lmdb(db_path, mode, max_read_counter_reopen))
    }

    /// Создает VStorage с удаленным хранилищем
    pub fn vstorage_remote(addr: &str) -> crate::vstorage::VStorage {
        crate::vstorage::VStorage::new(Self::remote(addr))
    }

    /// Создает VStorage с Tarantool
    #[cfg(any(feature = "tt_2", feature = "tt_3"))]
    pub fn vstorage_tarantool(tt_uri: String, login: &str, pass: &str) -> crate::vstorage::VStorage {
        crate::vstorage::VStorage::new(Self::tarantool(tt_uri, login, pass))
    }

    // ========================================================================================
    // ФАБРИЧНЫЕ МЕТОДЫ ДЛЯ GENERIC ВЕРСИЙ (static dispatch)
    // ========================================================================================

    /// Создает generic хранилище в памяти
    pub fn memory_generic() -> crate::vstorage::VMemoryStorage {
        log::info!("Creating generic in-memory storage");
        crate::vstorage::VMemoryStorage::new(crate::memory_storage::MemoryStorage::new())
    }

    /// Создает generic LMDB хранилище
    pub fn lmdb_generic(db_path: &str, mode: StorageMode, max_read_counter_reopen: Option<u64>) -> crate::vstorage::VLMDBStorage {
        log::info!("Creating generic LMDB storage, path: {}", db_path);
        crate::vstorage::VLMDBStorage::new(crate::lmdb_storage::LMDBStorage::new(db_path, mode, max_read_counter_reopen))
    }

    /// Создает generic удаленное хранилище
    pub fn remote_generic(addr: &str) -> crate::vstorage::VRemoteStorage {
        log::info!("Creating generic remote storage, addr: {}", addr);
        crate::vstorage::VRemoteStorage::new(crate::remote_storage_client::StorageROClient::new(addr))
    }

    /// Создает generic Tarantool хранилище
    #[cfg(any(feature = "tt_2", feature = "tt_3"))]
    pub fn tarantool_generic(tt_uri: String, login: &str, pass: &str) -> crate::vstorage::VTTStorage {
        log::info!("Creating generic Tarantool storage, addr: {}", tt_uri);
        crate::vstorage::VTTStorage::new(crate::tt_storage::TTStorage::new(tt_uri, login, pass))
    }
}

/// Реализация фабрики по умолчанию
pub struct DefaultStorageFactory;

impl DefaultStorageFactory {
    pub fn new() -> Self {
        Self
    }

    pub fn create_storage_from_config(&self, config: StorageConfig) -> Result<Box<dyn Storage>, StorageError> {
        match config {
            StorageConfig::Memory => {
                Ok(StorageProvider::memory())
            }
            StorageConfig::Lmdb { path, mode, max_read_counter_reopen } => {
                Ok(StorageProvider::lmdb(&path, mode, max_read_counter_reopen))
            }
            StorageConfig::Remote { address } => {
                Ok(StorageProvider::remote(&address))
            }
            #[cfg(any(feature = "tt_2", feature = "tt_3"))]
            StorageConfig::Tarantool { uri, login, password } => {
                Ok(StorageProvider::tarantool(uri, &login, &password))
            }
        }
    }
}

impl Default for DefaultStorageFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_builder_memory() {
        let storage = StorageBuilder::new()
            .memory()
            .build();

        assert!(storage.is_ok());
    }

    #[test]
    fn test_storage_builder_no_config() {
        let storage = StorageBuilder::new().build();
        assert!(storage.is_err());
    }

    #[test]
    fn test_generic_memory_builder() {
        let storage = StorageBuilder::new()
            .memory()
            .build_memory_generic();

        assert!(storage.is_ok());
    }

    #[test]
    fn test_generic_lmdb_builder() {
        let storage = StorageBuilder::new()
            .lmdb("/tmp/test", StorageMode::ReadOnly, None)
            .build_lmdb_generic();

        assert!(storage.is_ok());
    }

    #[test]
    fn test_generic_remote_builder() {
        let storage = StorageBuilder::new()
            .remote("127.0.0.1:8080")
            .build_remote_generic();

        assert!(storage.is_ok());
    }

    #[cfg(any(feature = "tt_2", feature = "tt_3"))]
    #[test]
    fn test_generic_tarantool_builder() {
        let storage = StorageBuilder::new()
            .tarantool("127.0.0.1:3301", "user", "pass")
            .build_tarantool_generic();

        assert!(storage.is_ok());
    }

    #[test]
    fn test_generic_builder_wrong_config() {
        let storage = StorageBuilder::new()
            .lmdb("/tmp/test", StorageMode::ReadOnly, None)
            .build_memory_generic();

        assert!(storage.is_err());
    }

    // ========================================================================================
    // ТЕСТЫ ДЛЯ STORAGE PROVIDER
    // ========================================================================================

    #[test]
    fn test_storage_provider_memory() {
        let _storage = StorageProvider::memory();
        // Проверяем что создание прошло без panic
    }

    #[test]
    fn test_storage_provider_vstorage_memory() {
        let mut storage = StorageProvider::vstorage_memory();
        assert!(!storage.is_empty());
        
        // Проверяем базовые операции с новым API
        assert!(storage.put_value(crate::common::StorageId::Individuals, "test", "value").is_ok());
        let get_result = storage.get_value(crate::common::StorageId::Individuals, "test");
        assert!(get_result.is_ok(), "Expected Ok, got: {:?}", get_result);
        if let crate::common::StorageResult::Ok(value) = get_result {
            assert_eq!(value, "value");
        }
    }

    #[test]
    fn test_storage_provider_generic_memory() {
        let mut storage = StorageProvider::memory_generic();
        assert!(!storage.is_empty());
        
        // Проверяем базовые операции с новым API
        assert!(storage.put_value(crate::common::StorageId::Individuals, "test", "value").is_ok());
        let get_result = storage.get_value(crate::common::StorageId::Individuals, "test");
        assert!(get_result.is_ok(), "Expected Ok, got: {:?}", get_result);
        if let crate::common::StorageResult::Ok(value) = get_result {
            assert_eq!(value, "value");
        }
    }

    #[test]
    fn test_storage_provider_lmdb() {
        let _storage = StorageProvider::lmdb("/tmp/test", StorageMode::ReadOnly, None);
        // Проверяем что создание прошло без panic
    }

    #[test]
    fn test_storage_provider_remote() {
        let _storage = StorageProvider::remote("127.0.0.1:8080");
        // Проверяем что создание прошло без panic
    }

    #[cfg(any(feature = "tt_2", feature = "tt_3"))]
    #[test]
    fn test_storage_provider_tarantool() {
        let _storage = StorageProvider::tarantool("127.0.0.1:3301".to_string(), "user", "pass");
        // Проверяем что создание прошло без panic
    }
} 