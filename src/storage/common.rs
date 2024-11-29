use crate::onto::individual::Individual;
use crate::storage::lmdb_storage::LMDBStorage;
use crate::storage::memory_storage::MemoryStorage;
use crate::storage::remote_storage_client::StorageROClient;
use crate::storage::tt_storage::TTStorage;
use crate::v_api::obj::ResultCode;

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum StorageMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum StorageId {
    Individuals,
    Tickets,
    Az,
}

pub trait Storage {
    fn get_individual_from_db(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> ResultCode;
    fn get_v(&mut self, storage: StorageId, key: &str) -> Option<String>;
    fn get_raw(&mut self, storage: StorageId, key: &str) -> Vec<u8>;
    fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> bool;
    fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> bool;
    fn remove(&mut self, storage: StorageId, key: &str) -> bool;
    fn count(&mut self, storage: StorageId) -> usize;
}

pub(crate) enum EStorage {
    Lmdb(LMDBStorage),
    Tt(TTStorage),
    Remote(StorageROClient),
    Memory(MemoryStorage),
    None,
}

pub struct VStorage {
    storage: EStorage,
}

impl VStorage {
    pub fn is_empty(&self) -> bool {
        matches!(self.storage, EStorage::None)
    }

    pub fn none() -> VStorage {
        VStorage {
            storage: EStorage::None,
        }
    }

    pub fn new_remote(addr: &str) -> VStorage {
        info!("Trying to connect to [remote], addr: {}", addr);
        VStorage {
            storage: EStorage::Remote(StorageROClient::new(addr)),
        }
    }

    pub fn new_tt(tt_uri: String, login: &str, pass: &str) -> VStorage {
        info!("Trying to connect to [Tarantool], addr: {}", tt_uri);
        VStorage {
            storage: EStorage::Tt(TTStorage::new(tt_uri, login, pass)),
        }
    }

    pub fn new_lmdb(db_path: &str, mode: StorageMode, max_read_counter_reopen: Option<u64>) -> VStorage {
        info!("Trying to connect to [LMDB], path: {}", db_path);
        VStorage {
            storage: EStorage::Lmdb(LMDBStorage::new(db_path, mode, max_read_counter_reopen)),
        }
    }

    pub fn new_memory() -> VStorage {
        info!("Creating in-memory storage");
        VStorage {
            storage: EStorage::Memory(MemoryStorage::new()),
        }
    }

    pub fn get_individual(&mut self, id: &str, iraw: &mut Individual) -> ResultCode {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_individual_from_db(StorageId::Individuals, id, iraw),
            EStorage::Lmdb(s) => s.get_individual_from_db(StorageId::Individuals, id, iraw),
            EStorage::Remote(s) => s.get_individual_from_db(StorageId::Individuals, id, iraw),
            EStorage::Memory(s) => s.get_individual_from_db(StorageId::Individuals, id, iraw),
            _ => ResultCode::NotReady,
        }
    }

    pub fn get_individual_from_db(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> ResultCode {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_individual_from_db(storage, id, iraw),
            EStorage::Lmdb(s) => s.get_individual_from_db(storage, id, iraw),
            EStorage::Remote(s) => s.get_individual_from_db(storage, id, iraw),
            EStorage::Memory(s) => s.get_individual_from_db(storage, id, iraw),
            _ => ResultCode::NotReady,
        }
    }

    pub fn get_value(&mut self, storage: StorageId, id: &str) -> Option<String> {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_v(storage, id),
            EStorage::Lmdb(s) => s.get_v(storage, id),
            EStorage::Remote(_s) => None,
            EStorage::Memory(s) => s.get_v(storage, id),
            _ => None,
        }
    }

    pub fn get_raw_value(&mut self, storage: StorageId, id: &str) -> Vec<u8> {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_raw(storage, id),
            EStorage::Lmdb(s) => s.get_raw(storage, id),
            EStorage::Remote(_s) => Default::default(),
            EStorage::Memory(s) => s.get_raw(storage, id),
            _ => Default::default(),
        }
    }

    pub fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> bool {
        match &mut self.storage {
            EStorage::Tt(s) => s.put_kv(storage, key, val),
            EStorage::Lmdb(s) => s.put_kv(storage, key, val),
            EStorage::Remote(_s) => false,
            EStorage::Memory(s) => s.put_kv(storage, key, val),
            _ => false,
        }
    }

    pub fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> bool {
        match &mut self.storage {
            EStorage::Tt(s) => s.put_kv_raw(storage, key, val),
            EStorage::Lmdb(s) => s.put_kv_raw(storage, key, val),
            EStorage::Remote(_s) => false,
            EStorage::Memory(s) => s.put_kv_raw(storage, key, val),
            _ => false,
        }
    }

    pub fn remove(&mut self, storage: StorageId, key: &str) -> bool {
        match &mut self.storage {
            EStorage::Tt(s) => s.remove(storage, key),
            EStorage::Lmdb(s) => s.remove(storage, key),
            EStorage::Remote(_s) => false,
            EStorage::Memory(s) => s.remove(storage, key),
            _ => false,
        }
    }

    pub fn count(&mut self, storage: StorageId) -> usize {
        match &mut self.storage {
            EStorage::Tt(s) => s.count(storage),
            EStorage::Lmdb(s) => s.count(storage),
            EStorage::Remote(s) => s.count(storage),
            EStorage::Memory(s) => s.count(storage),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage() {
        let mut storage = VStorage::new_memory();

        // Basic operations
        assert!(storage.put_kv(StorageId::Individuals, "test_key", "test_value"));
        assert_eq!(storage.get_value(StorageId::Individuals, "test_key"), Some("test_value".to_string()));

        // Check count
        assert_eq!(storage.count(StorageId::Individuals), 1);

        // Remove operation
        assert!(storage.remove(StorageId::Individuals, "test_key"));
        assert_eq!(storage.get_value(StorageId::Individuals, "test_key"), None);

        // Raw operations
        let raw_data = vec![1, 2, 3, 4];
        assert!(storage.put_kv_raw(StorageId::Individuals, "raw_key", raw_data.clone()));
        assert_eq!(storage.get_raw_value(StorageId::Individuals, "raw_key"), raw_data);
    }

    #[test]
    fn test_empty_storage() {
        let storage = VStorage::none();
        assert!(storage.is_empty());
    }

    #[test]
    fn test_individual_operations() {
        let mut storage = VStorage::new_memory();
        let mut individual = Individual::default();

        // Test with non-existent individual
        assert_eq!(storage.get_individual("non-existent", &mut individual), ResultCode::NotFound);
    }
}
