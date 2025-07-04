use v_individual_model::onto::individual::Individual;
use crate::common::{Storage, StorageId, StorageResult, StorageDispatcher};

// ========================================================================================
// ОПТИМИЗИРОВАННАЯ ENUM-BASED ВЕРСИЯ ДЛЯ КРИТИЧНЫХ ПО ПРОИЗВОДИТЕЛЬНОСТИ СЛУЧАЕВ
// ========================================================================================

/// Enum-based хранилище для максимальной производительности
/// 
/// Преимущества:
/// - Нет vtable lookups - все вызовы статически диспетчируются
/// - Компилятор может лучше оптимизировать код
/// - Нет heap allocations для storage
/// - Лучшая производительность чем trait objects
/// 
/// Рекомендуется для:
/// - Горячих путей в приложении
/// - Batch операций
/// - Высокопроизводительных сценариев
pub enum VStorageEnum {
    Memory(crate::memory_storage::MemoryStorage),
    Lmdb(crate::lmdb_storage::LMDBStorage),
    Remote(crate::remote_storage_client::StorageROClient),
    #[cfg(any(feature = "tt_2", feature = "tt_3"))]
    Tarantool(crate::tt_storage::TTStorage),
    None,
}

impl Default for VStorageEnum {
    fn default() -> Self {
        VStorageEnum::None
    }
}

impl VStorageEnum {
    /// Создает память хранилище
    pub fn memory() -> Self {
        VStorageEnum::Memory(crate::memory_storage::MemoryStorage::new())
    }

    /// Создает LMDB хранилище
    pub fn lmdb(path: &str, mode: crate::common::StorageMode, max_read_counter_reopen: Option<u64>) -> Self {
        VStorageEnum::Lmdb(crate::lmdb_storage::LMDBStorage::new(path, mode, max_read_counter_reopen))
    }

    /// Создает удаленное хранилище
    pub fn remote(address: &str) -> Self {
        VStorageEnum::Remote(crate::remote_storage_client::StorageROClient::new(address))
    }

    /// Создает Tarantool хранилище
    #[cfg(any(feature = "tt_2", feature = "tt_3"))]
    pub fn tarantool(uri: String, login: &str, password: &str) -> Self {
        VStorageEnum::Tarantool(crate::tt_storage::TTStorage::new(uri, login, password))
    }

    /// Проверяет, пусто ли хранилище
    pub fn is_empty(&self) -> bool {
        matches!(self, VStorageEnum::None)
    }
}

impl Storage for VStorageEnum {
    fn get_individual(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> StorageResult<()> {
        match self {
            VStorageEnum::Memory(s) => s.get_individual(storage, id, iraw),
            VStorageEnum::Lmdb(s) => s.get_individual(storage, id, iraw),
            VStorageEnum::Remote(s) => s.get_individual(storage, id, iraw),
            #[cfg(any(feature = "tt_2", feature = "tt_3"))]
            VStorageEnum::Tarantool(s) => s.get_individual(storage, id, iraw),
            VStorageEnum::None => StorageResult::NotReady,
        }
    }

    fn get_value(&mut self, storage: StorageId, key: &str) -> StorageResult<String> {
        match self {
            VStorageEnum::Memory(s) => s.get_value(storage, key),
            VStorageEnum::Lmdb(s) => s.get_value(storage, key),
            VStorageEnum::Remote(s) => s.get_value(storage, key),
            #[cfg(any(feature = "tt_2", feature = "tt_3"))]
            VStorageEnum::Tarantool(s) => s.get_value(storage, key),
            VStorageEnum::None => StorageResult::NotReady,
        }
    }

    fn get_raw_value(&mut self, storage: StorageId, key: &str) -> StorageResult<Vec<u8>> {
        match self {
            VStorageEnum::Memory(s) => s.get_raw_value(storage, key),
            VStorageEnum::Lmdb(s) => s.get_raw_value(storage, key),
            VStorageEnum::Remote(s) => s.get_raw_value(storage, key),
            #[cfg(any(feature = "tt_2", feature = "tt_3"))]
            VStorageEnum::Tarantool(s) => s.get_raw_value(storage, key),
            VStorageEnum::None => StorageResult::NotReady,
        }
    }

    fn put_value(&mut self, storage: StorageId, key: &str, val: &str) -> StorageResult<()> {
        match self {
            VStorageEnum::Memory(s) => s.put_value(storage, key, val),
            VStorageEnum::Lmdb(s) => s.put_value(storage, key, val),
            VStorageEnum::Remote(s) => s.put_value(storage, key, val),
            #[cfg(any(feature = "tt_2", feature = "tt_3"))]
            VStorageEnum::Tarantool(s) => s.put_value(storage, key, val),
            VStorageEnum::None => StorageResult::NotReady,
        }
    }

    fn put_raw_value(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> StorageResult<()> {
        match self {
            VStorageEnum::Memory(s) => s.put_raw_value(storage, key, val),
            VStorageEnum::Lmdb(s) => s.put_raw_value(storage, key, val),
            VStorageEnum::Remote(s) => s.put_raw_value(storage, key, val),
            #[cfg(any(feature = "tt_2", feature = "tt_3"))]
            VStorageEnum::Tarantool(s) => s.put_raw_value(storage, key, val),
            VStorageEnum::None => StorageResult::NotReady,
        }
    }

    fn remove_value(&mut self, storage: StorageId, key: &str) -> StorageResult<()> {
        match self {
            VStorageEnum::Memory(s) => s.remove_value(storage, key),
            VStorageEnum::Lmdb(s) => s.remove_value(storage, key),
            VStorageEnum::Remote(s) => s.remove_value(storage, key),
            #[cfg(any(feature = "tt_2", feature = "tt_3"))]
            VStorageEnum::Tarantool(s) => s.remove_value(storage, key),
            VStorageEnum::None => StorageResult::NotReady,
        }
    }

    fn count(&mut self, storage: StorageId) -> StorageResult<usize> {
        match self {
            VStorageEnum::Memory(s) => s.count(storage),
            VStorageEnum::Lmdb(s) => s.count(storage),
            VStorageEnum::Remote(s) => s.count(storage),
            #[cfg(any(feature = "tt_2", feature = "tt_3"))]
            VStorageEnum::Tarantool(s) => s.count(storage),
            VStorageEnum::None => StorageResult::NotReady,
        }
    }
}

// ========================================================================================
// ОСНОВНОЙ VSTORAGE - КОНТЕЙНЕР И ДИСПЕТЧЕР  
// ========================================================================================

/// Контейнер для хранилища с динамической диспетчеризацией
/// 
/// Ответственности:
/// - Хранение экземпляра Storage
/// - Диспетчеризация вызовов к хранилищу
/// - Обработка состояния "не инициализировано"
pub struct VStorage {
    storage: Option<Box<dyn Storage>>,
}

impl StorageDispatcher for VStorage {
    type Storage = Box<dyn Storage>;

    fn with_storage<T, F>(&mut self, default_value: T, operation: F) -> T 
    where 
        F: FnOnce(&mut Self::Storage) -> T,
    {
        match self.storage.as_mut() {
            Some(storage) => operation(storage),
            None => default_value,
        }
    }
}

impl VStorage {
    /// Создает пустое хранилище (не инициализированное)
    pub fn none() -> VStorage {
        VStorage {
            storage: None,
        }
    }

    /// Проверяет, пусто ли хранилище
    pub fn is_empty(&self) -> bool {
        self.storage.is_none()
    }

    /// Основной конструктор принимающий готовое хранилище
    pub fn new(storage: Box<dyn Storage>) -> VStorage {
        VStorage {
            storage: Some(storage),
        }
    }

    /// Получает ссылку на Builder для создания хранилищ
    pub fn builder() -> crate::storage_factory::StorageBuilder {
        crate::storage_factory::StorageBuilder::new()
    }

    /// Создание через конфигурацию
    pub fn from_config(config: crate::storage_factory::StorageConfig) -> Result<VStorage, crate::storage_factory::StorageError> {
        let storage = crate::storage_factory::DefaultStorageFactory::new()
            .create_storage_from_config(config)?;
        Ok(VStorage::new(storage))
    }

    // ========================================================================================
    // ПУБЛИЧНЫЕ МЕТОДЫ API - УНИФИЦИРОВАННОЕ ИМЕНОВАНИЕ
    // ========================================================================================

    pub fn get_individual(&mut self, id: &str, iraw: &mut Individual) -> StorageResult<()> {
        self.with_storage(StorageResult::NotReady, |s| s.get_individual(StorageId::Individuals, id, iraw))
    }

    pub fn get_individual_from_storage(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> StorageResult<()> {
        self.with_storage(StorageResult::NotReady, |s| s.get_individual(storage, id, iraw))
    }

    pub fn get_value(&mut self, storage: StorageId, id: &str) -> StorageResult<String> {
        self.with_storage_value(|s| s.get_value(storage, id))
    }

    pub fn get_raw_value(&mut self, storage: StorageId, id: &str) -> StorageResult<Vec<u8>> {
        self.with_storage_value(|s| s.get_raw_value(storage, id))
    }

    pub fn put_value(&mut self, storage: StorageId, key: &str, val: &str) -> StorageResult<()> {
        self.with_storage_result(|s| s.put_value(storage, key, val))
    }

    pub fn put_raw_value(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> StorageResult<()> {
        self.with_storage_result(|s| s.put_raw_value(storage, key, val))
    }

    pub fn remove_value(&mut self, storage: StorageId, key: &str) -> StorageResult<()> {
        self.with_storage_result(|s| s.remove_value(storage, key))
    }

    pub fn count(&mut self, storage: StorageId) -> StorageResult<usize> {
        self.with_storage_value(|s| s.count(storage))
    }

    // ========================================================================================
    // DEPRECATED МЕТОДЫ ДЛЯ ОБРАТНОЙ СОВМЕСТИМОСТИ
    // ========================================================================================

    #[deprecated(since = "0.1.0", note = "Use get_individual_from_storage instead")]
    pub fn get_individual_from_db(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> StorageResult<()> {
        self.get_individual_from_storage(storage, id, iraw)
    }

    #[deprecated(since = "0.1.0", note = "Use get_value instead")]
    pub fn get_v(&mut self, storage: StorageId, id: &str) -> Option<String> {
        match self.get_value(storage, id) {
            StorageResult::Ok(value) => Some(value),
            _ => None,
        }
    }

    #[deprecated(since = "0.1.0", note = "Use get_raw_value instead")]
    pub fn get_raw(&mut self, storage: StorageId, id: &str) -> Vec<u8> {
        self.get_raw_value(storage, id).unwrap_or_default()
    }

    #[deprecated(since = "0.1.0", note = "Use put_value instead")]
    pub fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> bool {
        self.put_value(storage, key, val).is_ok()
    }

    #[deprecated(since = "0.1.0", note = "Use put_raw_value instead")]
    pub fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> bool {
        self.put_raw_value(storage, key, val).is_ok()
    }

    #[deprecated(since = "0.1.0", note = "Use remove_value instead")]
    pub fn remove(&mut self, storage: StorageId, key: &str) -> bool {
        self.remove_value(storage, key).is_ok()
    }
}

// ========================================================================================
// GENERIC ВЕРСИЯ - VStorageGeneric<S>
// ========================================================================================

/// Generic версия VStorage с статической диспетчеризацией
/// 
/// Преимущества:
/// - Нет накладных расходов на динамическую диспетчеризацию
/// - Нет heap allocations для storage
/// - Лучшая производительность
/// - Больше возможностей для оптимизации компилятором
pub struct VStorageGeneric<S: Storage> {
    storage: Option<S>,
}

impl<S: Storage> StorageDispatcher for VStorageGeneric<S> {
    type Storage = S;

    fn with_storage<T, F>(&mut self, default_value: T, operation: F) -> T 
    where 
        F: FnOnce(&mut Self::Storage) -> T,
    {
        match self.storage.as_mut() {
            Some(storage) => operation(storage),
            None => default_value,
        }
    }
}

impl<S: Storage> VStorageGeneric<S> {
    /// Создает новое generic хранилище с конкретным типом
    pub fn new(storage: S) -> Self {
        Self {
            storage: Some(storage),
        }
    }

    /// Создает пустое хранилище
    pub fn none() -> Self {
        Self {
            storage: None,
        }
    }

    /// Проверяет, пусто ли хранилище
    pub fn is_empty(&self) -> bool {
        self.storage.is_none()
    }

    /// Берет хранилище из структуры, оставляя None
    pub fn take_storage(mut self) -> Option<S> {
        self.storage.take()
    }

    /// Возвращает ссылку на хранилище
    pub fn storage(&self) -> Option<&S> {
        self.storage.as_ref()
    }

    /// Возвращает мутабельную ссылку на хранилище
    pub fn storage_mut(&mut self) -> Option<&mut S> {
        self.storage.as_mut()
    }

    // ========================================================================================
    // ПУБЛИЧНЫЕ МЕТОДЫ API - ИДЕНТИЧНЫЕ VStorage
    // ========================================================================================

    pub fn get_individual(&mut self, id: &str, iraw: &mut Individual) -> StorageResult<()> {
        self.with_storage(StorageResult::NotReady, |s| s.get_individual(StorageId::Individuals, id, iraw))
    }

    pub fn get_individual_from_storage(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> StorageResult<()> {
        self.with_storage(StorageResult::NotReady, |s| s.get_individual(storage, id, iraw))
    }

    pub fn get_value(&mut self, storage: StorageId, id: &str) -> StorageResult<String> {
        self.with_storage_value(|s| s.get_value(storage, id))
    }

    pub fn get_raw_value(&mut self, storage: StorageId, id: &str) -> StorageResult<Vec<u8>> {
        self.with_storage_value(|s| s.get_raw_value(storage, id))
    }

    pub fn put_value(&mut self, storage: StorageId, key: &str, val: &str) -> StorageResult<()> {
        self.with_storage_result(|s| s.put_value(storage, key, val))
    }

    pub fn put_raw_value(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> StorageResult<()> {
        self.with_storage_result(|s| s.put_raw_value(storage, key, val))
    }

    pub fn remove_value(&mut self, storage: StorageId, key: &str) -> StorageResult<()> {
        self.with_storage_result(|s| s.remove_value(storage, key))
    }

    pub fn count(&mut self, storage: StorageId) -> StorageResult<usize> {
        self.with_storage_value(|s| s.count(storage))
    }
}

// Реализация Default для случаев, когда S реализует Default
impl<S: Storage + Default> Default for VStorageGeneric<S> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

// Реализация Clone для случаев, когда S реализует Clone
impl<S: Storage + Clone> Clone for VStorageGeneric<S> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
        }
    }
}

// Реализация Debug для случаев, когда S реализует Debug
impl<S: Storage + std::fmt::Debug> std::fmt::Debug for VStorageGeneric<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VStorageGeneric")
            .field("storage", &self.storage)
            .finish()
    }
}

// ========================================================================================
// TYPE ALIASES ДЛЯ УДОБСТВА
// ========================================================================================

pub type VMemoryStorage = VStorageGeneric<crate::memory_storage::MemoryStorage>;
pub type VLMDBStorage = VStorageGeneric<crate::lmdb_storage::LMDBStorage>;
pub type VRemoteStorage = VStorageGeneric<crate::remote_storage_client::StorageROClient>;
#[cfg(any(feature = "tt_2", feature = "tt_3"))]
pub type VTTStorage = VStorageGeneric<crate::tt_storage::TTStorage>;

// ========================================================================================
// ТЕСТЫ
// ========================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage_factory::StorageConfig;

    #[test]
    fn test_enum_storage_performance() {
        let mut enum_storage = VStorageEnum::memory();
        
        // Test enum dispatch operations
        assert!(enum_storage.put_value(StorageId::Individuals, "key1", "value1").is_ok());
        let get_result = enum_storage.get_value(StorageId::Individuals, "key1");
        assert!(get_result.is_ok(), "Expected Ok, got: {:?}", get_result);
        if let StorageResult::Ok(value) = get_result {
            assert_eq!(value, "value1");
        }
        
        // Test count
        let count_result = enum_storage.count(StorageId::Individuals);
        assert!(count_result.is_ok(), "Expected Ok count, got: {:?}", count_result);
        if let StorageResult::Ok(count) = count_result {
            assert_eq!(count, 1);
        }
    }

    #[test]
    fn test_enum_storage_empty() {
        let mut storage = VStorageEnum::default();
        assert!(storage.is_empty());
        
        // Operations on empty storage should return NotReady
        let get_result = storage.get_value(StorageId::Individuals, "key");
        assert_eq!(get_result, StorageResult::NotReady, "Expected NotReady, got: {:?}", get_result);
        
        // Test other operations on empty storage
        let put_result = storage.put_value(StorageId::Individuals, "key", "value");
        assert_eq!(put_result, StorageResult::NotReady, "Expected NotReady for put, got: {:?}", put_result);
        
        let count_result = storage.count(StorageId::Individuals);
        assert_eq!(count_result, StorageResult::NotReady, "Expected NotReady for count, got: {:?}", count_result);
    }

    #[test]
    fn test_memory_storage_builder() {
        let storage_result = VStorage::builder()
            .memory()
            .build();
        
        assert!(storage_result.is_ok(), "Failed to build storage");
        if let Ok(storage_box) = storage_result {
            let mut storage = VStorage::new(storage_box);

            // Test new unified API
            assert!(storage.put_value(StorageId::Individuals, "test_key", "test_value").is_ok());
            let get_result = storage.get_value(StorageId::Individuals, "test_key");
            assert!(get_result.is_ok(), "Expected Ok, got: {:?}", get_result);
            if let StorageResult::Ok(value) = get_result {
                assert_eq!(value, "test_value");
            }
        }
    }

    #[test]
    fn test_storage_from_config() {
        let config = StorageConfig::Memory;
        let storage_result = VStorage::from_config(config);
        
        assert!(storage_result.is_ok(), "Failed to create storage from config");
        if let Ok(mut storage) = storage_result {
            // Test unified API
            assert!(storage.put_value(StorageId::Individuals, "test_key", "test_value").is_ok());
            let get_result = storage.get_value(StorageId::Individuals, "test_key");
            assert!(get_result.is_ok(), "Expected Ok, got: {:?}", get_result);
            if let StorageResult::Ok(value) = get_result {
                assert_eq!(value, "test_value");
            }
        }
    }

    #[test]
    fn test_empty_storage() {
        let storage = VStorage::none();
        assert!(storage.is_empty());
    }

    #[test]
    fn test_individual_operations() {
        let storage_box = VStorage::builder()
            .memory()
            .build()
            .unwrap();
        let mut storage = VStorage::new(storage_box);
        let mut individual = Individual::default();

        // Test with non-existent individual
        assert_eq!(storage.get_individual("non-existent", &mut individual), StorageResult::NotFound);
    }

    #[test]
    fn test_backward_compatibility() {
        let storage_box = VStorage::builder()
            .memory()
            .build()
            .unwrap();
        let mut storage = VStorage::new(storage_box);

        // Test deprecated methods still work
        #[allow(deprecated)]
        {
            assert!(storage.put_kv(StorageId::Individuals, "key", "value"));
            assert_eq!(storage.get_v(StorageId::Individuals, "key"), Some("value".to_string()));
            assert!(storage.remove(StorageId::Individuals, "key"));
        }
    }

    // ========================================================================================
    // ТЕСТЫ ДЛЯ GENERIC ВЕРСИИ
    // ========================================================================================

    #[test]
    fn test_generic_memory_storage() {
        let mut storage = VMemoryStorage::new(crate::memory_storage::MemoryStorage::new());

        // Test unified API
        assert!(storage.put_value(StorageId::Individuals, "test_key", "test_value").is_ok());
        let get_result = storage.get_value(StorageId::Individuals, "test_key");
        assert!(get_result.is_ok(), "Expected Ok, got: {:?}", get_result);
        if let StorageResult::Ok(value) = get_result {
            assert_eq!(value, "test_value");
        }
        
        // Test count
        let count_result = storage.count(StorageId::Individuals);
        assert!(count_result.is_ok(), "Expected Ok count, got: {:?}", count_result);
        if let StorageResult::Ok(count) = count_result {
            assert_eq!(count, 1);
        }
        
        // Test remove
        assert!(storage.remove_value(StorageId::Individuals, "test_key").is_ok());
        let count_after_remove = storage.count(StorageId::Individuals);
        assert!(count_after_remove.is_ok(), "Expected Ok count after remove, got: {:?}", count_after_remove);
        if let StorageResult::Ok(count) = count_after_remove {
            assert_eq!(count, 0);
        }
        
        // Test edge case: remove non-existent key
        let remove_result = storage.remove_value(StorageId::Individuals, "non-existent");
        assert_eq!(remove_result, StorageResult::NotFound, "Expected NotFound for non-existent key");
    }

    #[test]
    fn test_generic_storage_creation() {
        // Создание через конструктор
        let memory_storage = crate::memory_storage::MemoryStorage::new();
        let mut generic_storage = VStorageGeneric::new(memory_storage);
        
        assert!(!generic_storage.is_empty());
        assert!(generic_storage.put_value(StorageId::Individuals, "key", "value").is_ok());
        
        // Извлечение хранилища
        let extracted = generic_storage.take_storage();
        assert!(extracted.is_some());
    }

    #[test]
    fn test_generic_storage_none() {
        let storage: VStorageGeneric<crate::memory_storage::MemoryStorage> = VStorageGeneric::none();
        assert!(storage.is_empty());
    }

    #[test]
    fn test_generic_storage_individual_operations() {
        let mut storage = VMemoryStorage::new(crate::memory_storage::MemoryStorage::new());
        let mut individual = Individual::default();

        // Test with non-existent individual
        assert_eq!(storage.get_individual("non-existent", &mut individual), StorageResult::NotFound);
    }

    #[test]
    fn test_unified_api_consistency() {
        // Проверяем что все три типа хранилищ имеют одинаковый API
        let mut dynamic_storage = VStorage::new(Box::new(crate::memory_storage::MemoryStorage::new()));
        let mut generic_storage = VMemoryStorage::new(crate::memory_storage::MemoryStorage::new());
        let mut enum_storage = VStorageEnum::memory();

        // Одинаковые операции должны работать одинаково
        assert!(dynamic_storage.put_value(StorageId::Individuals, "key", "value").is_ok());
        assert!(generic_storage.put_value(StorageId::Individuals, "key", "value").is_ok());
        assert!(enum_storage.put_value(StorageId::Individuals, "key", "value").is_ok());

        let dynamic_result = dynamic_storage.get_value(StorageId::Individuals, "key");
        let generic_result = generic_storage.get_value(StorageId::Individuals, "key");
        let enum_result = enum_storage.get_value(StorageId::Individuals, "key");
        
        // Все результаты должны быть Ok
        assert!(dynamic_result.is_ok(), "Dynamic storage should return Ok");
        assert!(generic_result.is_ok(), "Generic storage should return Ok");
        assert!(enum_result.is_ok(), "Enum storage should return Ok");
        
        // Все значения должны быть одинаковыми
        if let (StorageResult::Ok(val1), StorageResult::Ok(val2), StorageResult::Ok(val3)) = 
            (dynamic_result, generic_result, enum_result) {
            assert_eq!(val1, val2, "Dynamic and generic storage values should match");
            assert_eq!(val2, val3, "Generic and enum storage values should match");
        }
        
        // Test count consistency
        let count1 = dynamic_storage.count(StorageId::Individuals);
        let count2 = generic_storage.count(StorageId::Individuals);
        let count3 = enum_storage.count(StorageId::Individuals);
        
        assert!(count1.is_ok() && count2.is_ok() && count3.is_ok(), "All counts should be Ok");
        if let (StorageResult::Ok(c1), StorageResult::Ok(c2), StorageResult::Ok(c3)) = (count1, count2, count3) {
            assert_eq!(c1, c2);
            assert_eq!(c2, c3);
        }
    }
} 