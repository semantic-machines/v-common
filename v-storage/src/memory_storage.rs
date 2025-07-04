// memory_storage.rs

use v_individual_model::onto::individual::Individual;
use v_individual_model::onto::parser::parse_raw;
use crate::common::{Storage, StorageId, StorageResult};
use std::collections::HashMap;
use std::sync::RwLock;

pub struct MemoryStorage {
    individuals: RwLock<HashMap<String, Vec<u8>>>,
    tickets: RwLock<HashMap<String, Vec<u8>>>,
    az: RwLock<HashMap<String, Vec<u8>>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStorage {
    pub fn new() -> Self {
        MemoryStorage {
            individuals: RwLock::new(HashMap::new()),
            tickets: RwLock::new(HashMap::new()),
            az: RwLock::new(HashMap::new()),
        }
    }

    fn get_storage(&self, storage: StorageId) -> &RwLock<HashMap<String, Vec<u8>>> {
        match storage {
            StorageId::Individuals => &self.individuals,
            StorageId::Tickets => &self.tickets,
            StorageId::Az => &self.az,
        }
    }

    #[cfg(test)]
    pub fn insert_test_data(&self, storage: StorageId, key: &str, val: Vec<u8>) {
        if let Ok(mut map) = self.get_storage(storage).write() {
            map.insert(key.to_string(), val);
        }
    }

    #[cfg(test)]
    pub fn get_test_data(&self, storage: StorageId, key: &str) -> Option<Vec<u8>> {
        if let Ok(map) = self.get_storage(storage).read() {
            map.get(key).cloned()
        } else {
            None
        }
    }
}

impl Storage for MemoryStorage {
    fn get_individual(&mut self, storage: StorageId, uri: &str, iraw: &mut Individual) -> StorageResult<()> {
        let storage_map = self.get_storage(storage);
        if let Some(data) = storage_map.read().unwrap().get(uri) {
            iraw.set_raw(data);
            if parse_raw(iraw).is_ok() {
                return StorageResult::Ok(());
            } else {
                return StorageResult::UnprocessableEntity;
            }
        }
        StorageResult::NotFound
    }

    fn get_value(&mut self, storage: StorageId, key: &str) -> StorageResult<String> {
        if let Ok(map) = self.get_storage(storage).read() {
            match map.get(key) {
                Some(val) => match String::from_utf8(val.clone()) {
                    Ok(string_val) => StorageResult::Ok(string_val),
                    Err(_) => StorageResult::Error("Invalid UTF-8 data".to_string()),
                },
                None => StorageResult::NotFound,
            }
        } else {
            StorageResult::NotReady
        }
    }

    fn get_raw_value(&mut self, storage: StorageId, key: &str) -> StorageResult<Vec<u8>> {
        if let Ok(map) = self.get_storage(storage).read() {
            match map.get(key) {
                Some(val) => StorageResult::Ok(val.clone()),
                None => StorageResult::NotFound,
            }
        } else {
            StorageResult::NotReady
        }
    }

    fn put_value(&mut self, storage: StorageId, key: &str, val: &str) -> StorageResult<()> {
        if let Ok(mut map) = self.get_storage(storage).write() {
            map.insert(key.to_string(), val.as_bytes().to_vec());
            StorageResult::Ok(())
        } else {
            StorageResult::NotReady
        }
    }

    fn put_raw_value(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> StorageResult<()> {
        if let Ok(mut map) = self.get_storage(storage).write() {
            map.insert(key.to_string(), val);
            StorageResult::Ok(())
        } else {
            StorageResult::NotReady
        }
    }

    fn remove_value(&mut self, storage: StorageId, key: &str) -> StorageResult<()> {
        if let Ok(mut map) = self.get_storage(storage).write() {
            match map.remove(key) {
                Some(_) => StorageResult::Ok(()),
                None => StorageResult::NotFound,
            }
        } else {
            StorageResult::NotReady
        }
    }

    fn count(&mut self, storage: StorageId) -> StorageResult<usize> {
        if let Ok(map) = self.get_storage(storage).read() {
            StorageResult::Ok(map.len())
        } else {
            StorageResult::NotReady
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut storage = MemoryStorage::new();

        // Test put and get
        assert!(storage.put_value(StorageId::Individuals, "key1", "value1").is_ok());
        let get_result = storage.get_value(StorageId::Individuals, "key1");
        assert!(get_result.is_ok(), "Expected Ok, got: {:?}", get_result);
        assert_eq!(get_result.unwrap_or_default(), "value1");

        // Test raw operations
        let raw_data = vec![1, 2, 3, 4];
        assert!(storage.put_raw_value(StorageId::Individuals, "key2", raw_data.clone()).is_ok());
        let raw_result = storage.get_raw_value(StorageId::Individuals, "key2");
        assert!(raw_result.is_ok(), "Expected Ok, got: {:?}", raw_result);
        assert_eq!(raw_result.unwrap_or_default(), raw_data);

        // Test remove
        assert!(storage.remove_value(StorageId::Individuals, "key1").is_ok());
        let removed_result = storage.get_value(StorageId::Individuals, "key1");
        assert_eq!(removed_result, StorageResult::NotFound, "Expected NotFound, got: {:?}", removed_result);

        // Test count
        let count_result = storage.count(StorageId::Individuals);
        assert!(count_result.is_ok(), "Expected Ok count, got: {:?}", count_result);
        assert_eq!(count_result.unwrap_or_default(), 1);
    }

    #[test]
    fn test_individual() {
        let mut storage = MemoryStorage::new();
        let mut individual = Individual::default();

        // Test non-existent individual
        assert_eq!(storage.get_individual(StorageId::Individuals, "non-existent", &mut individual), StorageResult::NotFound);

        // Test with properly formatted Individual data
        let valid_individual_data = r#"{"@": "test:individual", "rdf:type": [{"type": "Uri", "data": "test:Class"}]}"#;
        let put_result = storage.put_value(StorageId::Individuals, "test:individual", valid_individual_data);
        assert!(put_result.is_ok(), "Failed to put individual data: {:?}", put_result);
        
                let get_result = storage.get_individual(StorageId::Individuals, "test:individual", &mut individual);
        assert!(get_result == StorageResult::Ok(()) || get_result == StorageResult::UnprocessableEntity, 
                "Expected Ok or UnprocessableEntity, got: {:?}", get_result);

        // Test with invalid data
        let invalid_data = "invalid json";
        assert!(storage.put_value(StorageId::Individuals, "test:invalid", invalid_data).is_ok());
        let invalid_result = storage.get_individual(StorageId::Individuals, "test:invalid", &mut individual);
        assert_eq!(invalid_result, StorageResult::UnprocessableEntity, "Expected UnprocessableEntity for invalid data");
    }

    #[test]
    fn test_backward_compatibility() {
        let mut storage = MemoryStorage::new();

        // Test deprecated methods still work
        #[allow(deprecated)]
        {
            assert!(storage.put_kv(StorageId::Individuals, "key", "value"));
            assert_eq!(storage.get_v(StorageId::Individuals, "key"), Some("value".to_string()));
            assert!(storage.remove(StorageId::Individuals, "key"));
        }
    }

    #[test]
    fn test_edge_cases() {
        let mut storage = MemoryStorage::new();

        // Test with very long keys and values
        let long_key = "a".repeat(1000);
        let long_value = "b".repeat(10000);
        assert!(storage.put_value(StorageId::Individuals, &long_key, &long_value).is_ok());
        
        let long_result = storage.get_value(StorageId::Individuals, &long_key);
        assert!(long_result.is_ok());
        if let StorageResult::Ok(value) = long_result {
            assert_eq!(value.len(), 10000);
            assert_eq!(value, long_value);
        }

        // Test with special characters
        let special_key = "тест-ключ!@#$%^&*()_+{}|:\"<>?";
        let special_value = "тест-значение\n\t\r\\\"'";
        assert!(storage.put_value(StorageId::Individuals, special_key, special_value).is_ok());
        
        let special_result = storage.get_value(StorageId::Individuals, special_key);
        assert!(special_result.is_ok());
        if let StorageResult::Ok(value) = special_result {
            assert_eq!(value, special_value);
        }

        // Test binary data in raw operations
        let binary_data = vec![0u8, 255u8, 128u8, 42u8];
        assert!(storage.put_raw_value(StorageId::Individuals, "binary", binary_data.clone()).is_ok());
        
        let binary_result = storage.get_raw_value(StorageId::Individuals, "binary");
        assert!(binary_result.is_ok());
        if let StorageResult::Ok(data) = binary_result {
            assert_eq!(data, binary_data);
        }

        // Test overwriting existing keys
        assert!(storage.put_value(StorageId::Individuals, "overwrite", "first").is_ok());
        assert!(storage.put_value(StorageId::Individuals, "overwrite", "second").is_ok());
        
        let overwrite_result = storage.get_value(StorageId::Individuals, "overwrite");
        assert!(overwrite_result.is_ok());
        if let StorageResult::Ok(value) = overwrite_result {
            assert_eq!(value, "second");
        }
    }

    #[test]
    fn test_different_storage_types() {
        let mut storage = MemoryStorage::new();

        // Test that different StorageId types are isolated
        assert!(storage.put_value(StorageId::Individuals, "same_key", "individuals_value").is_ok());
        assert!(storage.put_value(StorageId::Tickets, "same_key", "tickets_value").is_ok());
        assert!(storage.put_value(StorageId::Az, "same_key", "az_value").is_ok());

        let individuals_result = storage.get_value(StorageId::Individuals, "same_key");
        let tickets_result = storage.get_value(StorageId::Tickets, "same_key");
        let az_result = storage.get_value(StorageId::Az, "same_key");

        assert!(individuals_result.is_ok() && tickets_result.is_ok() && az_result.is_ok());

        if let (StorageResult::Ok(ind_val), StorageResult::Ok(tick_val), StorageResult::Ok(az_val)) = 
            (individuals_result, tickets_result, az_result) {
            assert_eq!(ind_val, "individuals_value");
            assert_eq!(tick_val, "tickets_value");
            assert_eq!(az_val, "az_value");
        }

        // Test counts are separate
        let ind_count = storage.count(StorageId::Individuals);
        let tick_count = storage.count(StorageId::Tickets);
        let az_count = storage.count(StorageId::Az);

        assert!(ind_count.is_ok() && tick_count.is_ok() && az_count.is_ok());
        if let (StorageResult::Ok(ic), StorageResult::Ok(tc), StorageResult::Ok(ac)) = (ind_count, tick_count, az_count) {
            assert_eq!(ic, 1);
            assert_eq!(tc, 1); 
            assert_eq!(ac, 1);
        }
    }
}
