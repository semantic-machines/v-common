use crate::onto::individual::Individual;
use crate::storage::lmdb_storage::LMDBStorage;
use crate::storage::remote_storage_client::*;
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

pub(crate) enum EStorage {
    Lmdb(LMDBStorage),
    Tt(TTStorage),
    Remote(StorageROClient),
    None,
}

pub trait Storage {
    fn get_individual_from_db(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> Result<(), ResultCode>;
    fn get_v(&mut self, storage: StorageId, key: &str) -> Result<Option<String>, ResultCode>;
    fn get_raw(&mut self, storage: StorageId, key: &str) -> Result<Option<Vec<u8>>, ResultCode>;
    fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> Result<(), ResultCode>;
    fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> Result<(), ResultCode>;
    fn remove(&mut self, storage: StorageId, key: &str) -> Result<(), ResultCode>;
    fn count(&mut self, storage: StorageId) -> Result<usize, ResultCode>;
}

pub struct VStorage {
    storage: EStorage,
}

impl VStorage {
    pub fn is_empty(&self) -> bool {
        match &self.storage {
            EStorage::None => true,
            _ => false,
        }
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

    pub fn get_individual(&mut self, id: &str, iraw: &mut Individual) -> Result<(), ResultCode> {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_individual_from_db(StorageId::Individuals, id, iraw),
            EStorage::Lmdb(s) => s.get_individual_from_db(StorageId::Individuals, id, iraw),
            EStorage::Remote(s) => s.get_individual_from_db(StorageId::Individuals, id, iraw),
            _ => Err(ResultCode::NotImplemented),
        }
    }

    pub fn get_individual_from_db(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> Result<(), ResultCode> {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_individual_from_db(storage, id, iraw),
            EStorage::Lmdb(s) => s.get_individual_from_db(storage, id, iraw),
            EStorage::Remote(s) => s.get_individual_from_db(storage, id, iraw),
            _ => Err(ResultCode::NotImplemented),
        }
    }

    pub fn get_value(&mut self, storage: StorageId, id: &str) -> Result<Option<String>, ResultCode> {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_v(storage, id),
            EStorage::Lmdb(s) => s.get_v(storage, id),
            EStorage::Remote(_s) => Err(ResultCode::NotImplemented),
            _ => Err(ResultCode::NotImplemented),
        }
    }

    pub fn get_raw_value(&mut self, storage: StorageId, id: &str) -> Result<Option<Vec<u8>>, ResultCode> {
        match &mut self.storage {
            EStorage::Tt(s) => s.get_raw(storage, id),
            EStorage::Lmdb(s) => s.get_raw(storage, id),
            EStorage::Remote(_s) => Err(ResultCode::NotImplemented),
            _ => Err(ResultCode::NotImplemented),
        }
    }

    pub fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> Result<(), ResultCode> {
        match &mut self.storage {
            EStorage::Tt(s) => s.put_kv(storage, key, val),
            EStorage::Lmdb(s) => s.put_kv(storage, key, val),
            EStorage::Remote(_s) => Err(ResultCode::NotImplemented),
            _ => Err(ResultCode::NotImplemented),
        }
    }

    pub fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> Result<(), ResultCode> {
        match &mut self.storage {
            EStorage::Tt(s) => s.put_kv_raw(storage, key, val),
            EStorage::Lmdb(s) => s.put_kv_raw(storage, key, val),
            EStorage::Remote(_s) => Err(ResultCode::NotImplemented),
            _ => Err(ResultCode::NotImplemented),
        }
    }

    pub fn remove(&mut self, storage: StorageId, key: &str) -> Result<(), ResultCode> {
        match &mut self.storage {
            EStorage::Tt(s) => s.remove(storage, key),
            EStorage::Lmdb(s) => s.remove(storage, key),
            EStorage::Remote(_s) => Err(ResultCode::NotImplemented),
            _ => Err(ResultCode::NotImplemented),
        }
    }

    pub fn count(&mut self, storage: StorageId) -> Result<usize, ResultCode> {
        match &mut self.storage {
            EStorage::Tt(s) => s.count(storage),
            EStorage::Lmdb(s) => s.count(storage),
            EStorage::Remote(_) => Err(ResultCode::NotImplemented),
            _ => Err(ResultCode::NotImplemented),
        }
    }
}
