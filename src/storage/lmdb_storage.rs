use crate::onto::individual::Individual;
use crate::onto::parser::parse_raw;
use crate::storage::common::{Storage, StorageId, StorageMode};
use crate::v_api::obj::ResultCode;
use lmdb_rs_m::core::{EnvCreateNoLock, EnvCreateNoMetaSync, EnvCreateNoSync, EnvCreateReadOnly};
use lmdb_rs_m::{DbFlags, DbHandle, EnvBuilder, Environment, MdbError};
use lmdb_rs_m::{FromMdbValue, ToMdbValue};
use std::iter::Iterator;

pub struct LMDBStorage {
    individuals_db: LmdbInstance,
    tickets_db: LmdbInstance,
    az_db: LmdbInstance,
}

pub struct LmdbInstance {
    max_read_counter: u64,
    path: String,
    mode: StorageMode,
    db_handle: Result<DbHandle, MdbError>,
    db_env: Result<Environment, MdbError>,
    read_counter: u64,
}

impl Default for LmdbInstance {
    fn default() -> Self {
        LmdbInstance {
            max_read_counter: 1000,
            path: String::default(),
            mode: StorageMode::ReadOnly,
            db_handle: Err(MdbError::Panic),
            db_env: Err(MdbError::Panic),
            read_counter: 0,
        }
    }
}

struct LmdbIterator {
    keys: Vec<Vec<u8>>,
    index: usize,
}

impl Iterator for LmdbIterator {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.keys.len() {
            None
        } else {
            let key = self.keys[self.index].clone();
            self.index += 1;
            Some(key)
        }
    }
}

impl LmdbInstance {
    pub fn new(path: &str, mode: StorageMode) -> Self {
        LmdbInstance {
            max_read_counter: 1000,
            path: path.to_string(),
            mode,
            db_handle: Err(MdbError::Panic),
            db_env: Err(MdbError::Panic),
            read_counter: 0,
        }
    }

    pub fn iter(&mut self) -> Box<dyn Iterator<Item = Vec<u8>>> {
        if self.db_env.is_err() {
            self.open();
        }

        match &self.db_env {
            Ok(env) => match &self.db_handle {
                Ok(handle) => match env.get_reader() {
                    Ok(txn) => {
                        let db = txn.bind(handle);
                        let cursor_result = db.new_cursor();
                        match cursor_result {
                            Ok(mut cursor) => {
                                let mut keys = Vec::new();
                                while let Ok(()) = cursor.to_next_item() {
                                    if let Ok(key) = cursor.get_key::<Vec<u8>>() {
                                        keys.push(key);
                                    }
                                }
                                Box::new(LmdbIterator {
                                    keys,
                                    index: 0,
                                })
                            },
                            Err(_) => Box::new(std::iter::empty()),
                        }
                    },
                    Err(_) => Box::new(std::iter::empty()),
                },
                Err(_) => Box::new(std::iter::empty()),
            },
            Err(_) => Box::new(std::iter::empty()),
        }
    }

    pub fn open(&mut self) {
        let env_builder = if self.mode == StorageMode::ReadOnly {
            EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync)
        } else {
            EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateNoMetaSync | EnvCreateNoSync)
        };

        let db_env = env_builder.open(&self.path, 0o644);

        let db_handle = match &db_env {
            Ok(env) => env.get_default_db(DbFlags::empty()),
            Err(e) => {
                error!("LMDB:fail opening read only environment, {}, err={:?}", self.path, e);
                Err(MdbError::Corrupted)
            },
        };

        self.db_handle = db_handle;
        self.db_env = db_env;
        self.read_counter = 0;
    }

    fn get_individual(&mut self, uri: &str, iraw: &mut Individual) -> Result<(), ResultCode> {
        match self.get::<Vec<u8>>(uri) {
            Ok(Some(val)) => {
                iraw.set_raw(&val);
                if parse_raw(iraw).is_ok() {
                    Ok(())
                } else {
                    error!("LMDB:fail parse binobj, {}, len={}, uri=[{}]", self.path, iraw.get_raw_len(), uri);
                    Err(ResultCode::UnprocessableEntity)
                }
            },
            Ok(None) => Err(ResultCode::NotFound),
            Err(e) => {
                error!("LMDB:fail get individual, {}, uri=[{}], err={:?}", self.path, uri, e);
                Err(e)
            },
        }
    }

    fn get_raw(&mut self, key: &str) -> Result<Option<Vec<u8>>, ResultCode> {
        self.get::<Vec<u8>>(key)
    }

    pub fn get<T: FromMdbValue>(&mut self, key: &str) -> Result<Option<T>, ResultCode> {
        if self.db_env.is_err() {
            self.open();
        }

        for _it in 0..2 {
            let mut is_need_reopen = false;

            self.read_counter += 1;
            if self.read_counter > self.max_read_counter {
                is_need_reopen = true;
            }

            match &self.db_env {
                Ok(env) => match &self.db_handle {
                    Ok(handle) => match env.get_reader() {
                        Ok(txn) => {
                            let db = txn.bind(handle);

                            match db.get::<T>(&key) {
                                Ok(val) => {
                                    return Ok(Some(val));
                                },
                                Err(e) => match e {
                                    MdbError::NotFound => {
                                        return Ok(None);
                                    },
                                    _ => {
                                        error!("LMDB:db.get {}, {:?}, key=[{}]", self.path, e, key);
                                        return Ok(None);
                                    },
                                },
                            }
                        },
                        Err(e) => match e {
                            MdbError::Other(c, _) => {
                                if c == -30785 {
                                    is_need_reopen = true;
                                } else {
                                    error!("LMDB:fail create transaction, {}, err={}", self.path, e);
                                    return Ok(None);
                                }
                            },
                            _ => {
                                error!("LMDB:fail create transaction, {}, err={}", self.path, e);
                            },
                        },
                    },
                    Err(e) => {
                        error!("LMDB:db handle, {}, err={}", self.path, e);
                        return Ok(None);
                    },
                },
                Err(e) => match e {
                    MdbError::Panic => {
                        is_need_reopen = true;
                    },
                    _ => {
                        error!("LMDB:db environment, {}, err={}", self.path, e);
                        return Ok(None);
                    },
                },
            }

            if is_need_reopen {
                warn!("db {} reopen", self.path);
                self.open();
            }
        }

        Ok(None)
    }

    fn get_v(&mut self, key: &str) -> Result<Option<String>, ResultCode> {
        self.get::<String>(key)
    }

    pub fn count(&mut self) -> Result<usize, ResultCode> {
        if self.db_env.is_err() {
            self.open();
        }

        for _it in 0..2 {
            let mut is_need_reopen = false;

            match &self.db_env {
                Ok(env) => match env.stat() {
                    Ok(stat) => {
                        return Ok(stat.ms_entries);
                    },
                    Err(e) => match e {
                        MdbError::Other(c, _) => {
                            if c == -30785 {
                                is_need_reopen = true;
                            } else {
                                error!("LMDB:fail read stat, {}, err={}", self.path, e);
                                return Err(ResultCode::DatabaseQueryError);
                            }
                        },
                        _ => {
                            error!("LMDB:fail create transaction, {}, err={}", self.path, e);
                        },
                    },
                },
                Err(e) => match e {
                    MdbError::Panic => {
                        is_need_reopen = true;
                    },
                    _ => {
                        error!("LMDB:db environment, {}, err={}", self.path, e);
                        return Err(ResultCode::FailOpenTransaction);
                    },
                },
            }

            if is_need_reopen {
                warn!("db {} reopen", self.path);
                self.open();
            }
        }

        // If we've reached this point, we've tried opening the database twice and still failed
        error!("LMDB:failed to open database after retries, {}", self.path);
        Err(ResultCode::FailOpenTransaction)
    }

    pub fn remove(&mut self, key: &str) -> Result<(), ResultCode> {
        if self.db_env.is_err() {
            self.open();
        }
        remove_from_lmdb(&self.db_env, &self.db_handle, key, &self.path)
    }

    pub fn put<T: ToMdbValue>(&mut self, key: &str, val: T) -> Result<(), ResultCode> {
        if self.db_env.is_err() {
            self.open();
        }
        put_kv_lmdb(&self.db_env, &self.db_handle, key, val, &self.path)
    }
}

impl LMDBStorage {
    pub fn new(db_path: &str, mode: StorageMode, max_read_counter_reopen: Option<u64>) -> LMDBStorage {
        LMDBStorage {
            individuals_db: LmdbInstance {
                max_read_counter: max_read_counter_reopen.unwrap_or(u32::MAX as u64),
                path: db_path.to_owned() + "/lmdb-individuals/",
                mode: mode.clone(),
                ..Default::default()
            },
            tickets_db: LmdbInstance {
                max_read_counter: max_read_counter_reopen.unwrap_or(u32::MAX as u64),
                path: db_path.to_owned() + "/lmdb-tickets/",
                mode: mode.clone(),
                ..Default::default()
            },
            az_db: LmdbInstance {
                max_read_counter: max_read_counter_reopen.unwrap_or(u32::MAX as u64),
                path: db_path.to_owned() + "/acl-indexes/",
                mode: mode.clone(),
                ..Default::default()
            },
        }
    }

    fn get_db_instance(&mut self, storage: &StorageId) -> &mut LmdbInstance {
        match storage {
            StorageId::Individuals => &mut self.individuals_db,
            StorageId::Tickets => &mut self.tickets_db,
            StorageId::Az => &mut self.az_db,
        }
    }

    pub fn open(&mut self, storage: StorageId) {
        let db_instance = self.get_db_instance(&storage);
        db_instance.open();

        info!("LMDBStorage: db {} open {:?}", db_instance.path, storage);
    }
}

impl Storage for LMDBStorage {
    fn get_individual_from_db(&mut self, storage: StorageId, uri: &str, iraw: &mut Individual) -> Result<(), ResultCode> {
        let db_instance = self.get_db_instance(&storage);
        db_instance.get_individual(uri, iraw)
    }

    fn get_v(&mut self, storage: StorageId, key: &str) -> Result<Option<String>, ResultCode> {
        let db_instance = self.get_db_instance(&storage);
        db_instance.get_v(key)
    }

    fn get_raw(&mut self, storage: StorageId, key: &str) -> Result<Option<Vec<u8>>, ResultCode> {
        let db_instance = self.get_db_instance(&storage);
        db_instance.get_raw(key)
    }

    fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> Result<(), ResultCode> {
        let db_instance = self.get_db_instance(&storage);

        put_kv_lmdb(&db_instance.db_env, &db_instance.db_handle, key, val.as_bytes(), &db_instance.path)
    }

    fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> Result<(), ResultCode> {
        let db_instance = self.get_db_instance(&storage);

        put_kv_lmdb(&db_instance.db_env, &db_instance.db_handle, key, val.as_slice(), &db_instance.path)
    }

    fn remove(&mut self, storage: StorageId, key: &str) -> Result<(), ResultCode> {
        let db_instance = self.get_db_instance(&storage);

        remove_from_lmdb(&db_instance.db_env, &db_instance.db_handle, key, &db_instance.path)
    }

    fn count(&mut self, storage: StorageId) -> Result<usize, ResultCode> {
        let db_instance = self.get_db_instance(&storage);
        db_instance.count()
    }
}

fn remove_from_lmdb(db_env: &Result<Environment, MdbError>, db_handle: &Result<DbHandle, MdbError>, key: &str, path: &str) -> Result<(), ResultCode> {
    match db_env {
        Ok(env) => match env.new_transaction() {
            Ok(txn) => match db_handle {
                Ok(handle) => {
                    let db = txn.bind(handle);
                    if let Err(e) = db.del(&key) {
                        error!("LMDB:failed put, {}, err={}", path, e);
                        return Err(ResultCode::FailStore);
                    }

                    if let Err(e) = txn.commit() {
                        if let MdbError::Other(c, _) = e {
                            if c == -30792 && grow_db(db_env, path) {
                                return remove_from_lmdb(db_env, db_handle, key, path);
                            }
                        }
                        error!("LMDB:failed to commit, {}, err={}", path, e);
                        return Err(ResultCode::FailStore);
                    }
                    Ok(())
                },
                Err(e) => {
                    error!("LMDB:db handle, {}, err={}", path, e);
                    return Err(ResultCode::FailStore);
                },
            },
            Err(e) => {
                error!("LMDB:db create transaction, {}, err={}", path, e);
                return Err(ResultCode::FailStore);
            },
        },
        Err(e) => {
            error!("LMDB:db environment, {}, err={}", path, e);
            return Err(ResultCode::FailStore);
        },
    }
}

fn put_kv_lmdb<T: ToMdbValue>(db_env: &Result<Environment, MdbError>, db_handle: &Result<DbHandle, MdbError>, key: &str, val: T, path: &str) -> Result<(), ResultCode> {
    match db_env {
        Ok(env) => match env.new_transaction() {
            Ok(txn) => match db_handle {
                Ok(handle) => {
                    let db = txn.bind(handle);
                    if let Err(e) = db.set(&key, &val) {
                        error!("LMDB:failed put, {}, err={}", path, e);
                        return Err(ResultCode::FailStore);
                    }

                    if let Err(e) = txn.commit() {
                        if let MdbError::Other(c, _) = e {
                            if c == -30792 && grow_db(db_env, path) {
                                return put_kv_lmdb(db_env, db_handle, key, val, path);
                            }
                        }
                        error!("LMDB:failed to commit, {}, err={}", path, e);
                        return Err(ResultCode::FailStore);
                    }
                    return Ok(());
                },
                Err(e) => {
                    error!("LMDB:db handle, {}, err={}", path, e);
                    return Err(ResultCode::FailStore);
                },
            },
            Err(e) => {
                error!("LMDB:db create transaction, {}, err={}", path, e);
                return Err(ResultCode::FailStore);
            },
        },
        Err(e) => {
            error!("LMDB:db environment, {}, err={}", path, e);
            return Err(ResultCode::FailStore);
        },
    }
}

fn grow_db(db_env: &Result<Environment, MdbError>, path: &str) -> bool {
    match db_env {
        Ok(env) => {
            if let Ok(stat) = env.info() {
                let new_size = stat.me_mapsize + 100 * 10_048_576;
                if env.set_mapsize(new_size).is_ok() {
                    info!("success grow db, new size = {}", new_size);
                    return true;
                }
            }
        },
        Err(e) => {
            error!("LMDB:db environment, {}, err={}", path, e);
        },
    }
    false
}
