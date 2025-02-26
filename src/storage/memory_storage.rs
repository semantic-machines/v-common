// memory_storage.rs

use v_individual_model::onto::individual::Individual;
use v_individual_model::onto::parser::parse_raw;
use crate::storage::common::{Storage, StorageId};
use crate::v_api::obj::ResultCode;
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
    fn get_individual_from_db(&mut self, storage: StorageId, uri: &str, iraw: &mut Individual) -> ResultCode {
        if let Ok(map) = self.get_storage(storage).read() {
            if let Some(val) = map.get(uri) {
                iraw.set_raw(val);
                if parse_raw(iraw).is_ok() {
                    return ResultCode::Ok;
                } else {
                    return ResultCode::UnprocessableEntity;
                }
            }
            ResultCode::NotFound
        } else {
            ResultCode::NotReady
        }
    }

    fn get_v(&mut self, storage: StorageId, key: &str) -> Option<String> {
        if let Ok(map) = self.get_storage(storage).read() {
            map.get(key).and_then(|v| String::from_utf8(v.clone()).ok())
        } else {
            None
        }
    }

    fn get_raw(&mut self, storage: StorageId, key: &str) -> Vec<u8> {
        if let Ok(map) = self.get_storage(storage).read() {
            map.get(key).cloned().unwrap_or_default()
        } else {
            Vec::default()
        }
    }

    fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> bool {
        if let Ok(mut map) = self.get_storage(storage).write() {
            map.insert(key.to_string(), val.as_bytes().to_vec());
            true
        } else {
            false
        }
    }

    fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> bool {
        if let Ok(mut map) = self.get_storage(storage).write() {
            map.insert(key.to_string(), val);
            true
        } else {
            false
        }
    }

    fn remove(&mut self, storage: StorageId, key: &str) -> bool {
        if let Ok(mut map) = self.get_storage(storage).write() {
            map.remove(key).is_some()
        } else {
            false
        }
    }

    fn count(&mut self, storage: StorageId) -> usize {
        if let Ok(map) = self.get_storage(storage).read() {
            map.len()
        } else {
            0
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
        assert!(storage.put_kv(StorageId::Individuals, "key1", "value1"));
        assert_eq!(storage.get_v(StorageId::Individuals, "key1"), Some("value1".to_string()));

        // Test raw operations
        let raw_data = vec![1, 2, 3, 4];
        assert!(storage.put_kv_raw(StorageId::Individuals, "key2", raw_data.clone()));
        assert_eq!(storage.get_raw(StorageId::Individuals, "key2"), raw_data);

        // Test remove
        assert!(storage.remove(StorageId::Individuals, "key1"));
        assert_eq!(storage.get_v(StorageId::Individuals, "key1"), None);

        // Test count
        assert_eq!(storage.count(StorageId::Individuals), 1);
    }

    #[test]
    fn test_individual() {
        let mut storage = MemoryStorage::new();
        let mut individual = Individual::default();

        // Test non-existent individual
        assert_eq!(storage.get_individual_from_db(StorageId::Individuals, "non-existent", &mut individual), ResultCode::NotFound);

        // Here you would add more tests with properly formatted Individual data
    }
}
