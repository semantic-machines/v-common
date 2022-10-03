use crate::onto::individual::Individual;
use crate::storage::lmdb_storage::LMDBStorage;
use crate::storage::remote_storage_client::*;
use crate::storage::tt_storage::TTStorage;

pub enum StorageError {
    None,
    NotReady,
}

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

pub(crate) enum EStorage {
    Lmdb(LMDBStorage),
    Tt(TTStorage),
    Remote(StorageROClient),
    None,
}

pub trait Storage {
    fn get_individual_from_db(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> bool;
    fn get_v(&mut self, storage: StorageId, key: &str) -> Option<String>;
    fn get_raw(&mut self, storage: StorageId, key: &str) -> Vec<u8>;
    fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> bool;
    fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> bool;
    fn remove(&mut self, storage: StorageId, key: &str) -> bool;
    fn count(&mut self, storage: StorageId) -> usize;
}

pub struct VStorage {
    storage: EStorage,
}

impl VStorage {
    pub fn none() -> VStorage {
        VStorage {
            storage: EStorage::None,
        }
    }

    pub fn new_remote(addr: &str) -> VStorage {
        VStorage {
            storage: EStorage::Remote(StorageROClient::new(addr)),
        }
    }

    pub fn new_tt(tt_id: String, login: &str, pass: &str) -> VStorage {
        VStorage {
            storage: EStorage::Tt(TTStorage::new(tt_id, login, pass)),
        }
    }

    pub fn new_lmdb(db_path: &str, mode: StorageMode, max_read_counter_reopen: Option<u64>) -> VStorage {
        VStorage {
            storage: EStorage::Lmdb(LMDBStorage::new(db_path, mode, max_read_counter_reopen)),
        }
    }

    pub fn get_individual(&mut self, id: &str, iraw: &mut Individual) -> bool {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_individual_from_db(StorageId::Individuals, id, iraw),
            EStorage::Lmdb(s) => s.get_individual_from_db(StorageId::Individuals, id, iraw),
            EStorage::Remote(s) => s.get_individual_from_db(StorageId::Individuals, id, iraw),
            _ => false,
        }
    }

    pub fn get_individual_from_db(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> bool {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_individual_from_db(storage, id, iraw),
            EStorage::Lmdb(s) => s.get_individual_from_db(storage, id, iraw),
            EStorage::Remote(s) => s.get_individual_from_db(storage, id, iraw),
            _ => false,
        }
    }

    pub fn get_value(&mut self, storage: StorageId, id: &str) -> Option<String> {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_v(storage, id),
            EStorage::Lmdb(s) => s.get_v(storage, id),
            EStorage::Remote(_s) => None,
            _ => None,
        }
    }

    pub fn get_raw_value(&mut self, storage: StorageId, id: &str) -> Vec<u8> {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_raw(storage, id),
            EStorage::Lmdb(s) => s.get_raw(storage, id),
            EStorage::Remote(_s) => Default::default(),
            _ => Default::default(),
        }
    }

    pub fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> bool {
        match &mut self.storage {
            EStorage::Tt(s) => s.put_kv(storage, key, val),
            EStorage::Lmdb(s) => s.put_kv(storage, key, val),
            EStorage::Remote(_s) => false,
            _ => false,
        }
    }

    pub fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> bool {
        match &mut self.storage {
            EStorage::Tt(s) => s.put_kv_raw(storage, key, val),
            EStorage::Lmdb(s) => s.put_kv_raw(storage, key, val),
            EStorage::Remote(_s) => false,
            _ => false,
        }
    }

    pub fn remove(&mut self, storage: StorageId, key: &str) -> bool {
        match &mut self.storage {
            EStorage::Tt(s) => s.remove(storage, key),
            EStorage::Lmdb(s) => s.remove(storage, key),
            EStorage::Remote(_s) => false,
            _ => false,
        }
    }

    pub fn count(&mut self, storage: StorageId) -> usize {
        match &mut self.storage {
            EStorage::Tt(s) => s.count(storage),
            EStorage::Lmdb(s) => s.count(storage),
            EStorage::Remote(s) => s.count(storage),
            _ => 0,
        }
    }
}
