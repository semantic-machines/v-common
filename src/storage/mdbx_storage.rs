use crate::onto::individual::Individual;
use crate::onto::parser::parse_raw;
use crate::storage::storage::{Storage, StorageId, StorageMode};
use heed::types::{ByteSlice, Str, UnalignedSlice};
use heed::{Database, Env, EnvOpenOptions, MdbError};
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::fs;
use std::str::from_utf8_unchecked;
use heed::flags::Flags;

pub(crate) struct MDBXStorage {
    db_path: String,

    individuals_db: Result<Database<Str, ByteSlice>, MdbError>,
    individuals_env: Result<Env, MdbError>,

    tickets_db: Result<Database<Str, ByteSlice>, MdbError>,
    tickets_env: Result<Env, MdbError>,

    az_db: Result<Database<Str, ByteSlice>, MdbError>,
    az_env: Result<Env, MdbError>,

    mode: StorageMode,
}

impl MDBXStorage {
    pub fn new(db_path: &str, mode: StorageMode) -> MDBXStorage {
        MDBXStorage {
            db_path: db_path.to_owned(),
            individuals_db: Err(MdbError::Panic),
            individuals_env: Err(MdbError::Panic),
            tickets_db: Err(MdbError::Panic),
            tickets_env: Err(MdbError::Panic),
            az_db: Err(MdbError::Panic),
            az_env: Err(MdbError::Panic),
            mode,
        }
    }

    fn open(&mut self, storage: StorageId, mode: StorageMode) -> Result<(), Box<dyn Error>> {
        let db_path = if storage == StorageId::Individuals {
            self.db_path.to_string() + "/lmdb-individuals/"
        } else if storage == StorageId::Tickets {
            self.db_path.to_string() + "/lmdb-tickets/"
        } else if storage == StorageId::Az {
            self.db_path.to_string() + "/acl-indexes/"
        } else {
            String::default()
        };

        fs::create_dir_all(&db_path)?;

        let mut env_builder = EnvOpenOptions::new();
        unsafe {
             env_builder.flag(Flags::MdbNoMetaSync);
         }
        env_builder.map_size(10 * 1024 * 1024);
        env_builder.max_dbs(3);

        let env = env_builder.open(db_path)?;

        if let Ok(db) = env.create_database::<Str, ByteSlice>(None) {
            if storage == StorageId::Individuals {
                self.individuals_db = Ok(db);
                self.individuals_env = Ok(env);
            } else if storage == StorageId::Tickets {
                self.tickets_db = Ok(db);
                self.tickets_env = Ok(env);
            } else if storage == StorageId::Az {
                self.az_db = Ok(db);
                self.az_env = Ok(env);
            }
        }
        Ok(())
    }
}

impl Storage for MDBXStorage {
    fn get_individual_from_db(&mut self, storage_id: StorageId, uri: &str, iraw: &mut Individual) -> bool {
        for _ in 0..2 {
            let (env, db) = get_db_of_id(storage_id.clone(), self);

            match env {
                Ok(env) => {
                    if let Ok(mut txn) = env.read_txn() {
                        match db {
                            Ok(db) => {
                                if let Ok(v) = db.get(&mut txn, uri) {
                                    if let Some(val) = v {
                                        iraw.set_raw(val);

                                        if parse_raw(iraw).is_ok() {
                                            return true;
                                        } else {
                                            error!("MDBX:fail parse binobj, len={}, uri=[{}]", iraw.get_raw_len(), uri);
                                            return false;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("MDBX:db, err={}, uri=[{}]", e, uri);
                            }
                        }
                    }
                }
                Err(e) => match e {
                    MdbError::Panic => {
                        warn!("MDBX: env {} reopen {:?}", self.db_path, &storage_id);
                        self.open(storage_id.clone(), self.mode.clone());
                    }
                    _ => {
                        error!("MDBX: environment, err={}, uri=[{}]", e, uri);
                        return false;
                    }
                },
            }
        }

        false
    }

    fn get_v(&mut self, storage_id: StorageId, key: &str) -> Option<String> {
        for _ in 0..2 {
            let (env, db) = get_db_of_id(storage_id.clone(), self);

            match env {
                Ok(env) => {
                    if let Ok(mut txn) = env.read_txn() {
                        match db {
                            Ok(db) => {
                                if let Ok(v) = db.get(&mut txn, key) {
                                    if let Some(v) = v {
                                        return Some(unsafe { from_utf8_unchecked(v).to_string() });
                                    }
                                }
                            }
                            Err(e) => {
                                error!("MDBX:db, err={}, uri=[{}]", e, key);
                            }
                        }
                    }
                }
                Err(e) => match e {
                    MdbError::Panic => {
                        warn!("MDBX: env {} reopen {:?}", self.db_path, &storage_id);
                        self.open(storage_id.clone(), self.mode.clone());
                    }
                    _ => {
                        error!("MDBX: environment, err={}, uri=[{}]", e, key);
                        return None;
                    }
                },
            }
        }

        None
    }

    fn get_raw(&mut self, storage_id: StorageId, key: &str) -> Vec<u8> {
        for _ in 0..2 {
            let (env, db) = get_db_of_id(storage_id.clone(), self);

            match env {
                Ok(env) => {
                    if let Ok(mut txn) = env.read_txn() {
                        match db {
                            Ok(db) => {
                                if let Ok(v) = db.get(&mut txn, key) {
                                    if let Some(v) = v {
                                        return v.to_vec();
                                    }
                                }
                            }
                            Err(e) => {
                                error!("MDBX:db, err={}, uri=[{}]", e, key);
                            }
                        }
                    }
                }
                Err(e) => match e {
                    MdbError::Panic => {
                        warn!("MDBX: env {} reopen {:?}", self.db_path, &storage_id);
                        self.open(storage_id.clone(), self.mode.clone());
                    }
                    _ => {
                        error!("MDBX: environment, err={}, uri=[{}]", e, key);
                        return vec![];
                    }
                },
            }
        }

        vec![]
    }

    fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> bool {
        let (env, db) = get_db_of_id(storage, self);
        return put_kv_mdbx(env, db, key, val.as_bytes()).is_ok();
    }

    fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> bool {
        let (env, db) = get_db_of_id(storage, self);
        return put_kv_mdbx(env, db, key, val.as_slice()).is_ok();
    }

    fn remove(&mut self, storage: StorageId, key: &str) -> bool {
        let (env, db) = get_db_of_id(storage, self);
        return remove_from_mdbx(env, db, key).is_ok();
    }

    fn count(&mut self, storage: StorageId) -> usize {
        0
    }
}

fn remove_from_mdbx(w_env: &Result<Env, MdbError>, w_db: &Result<Database<Str, ByteSlice>, MdbError>, key: &str) -> Result<(), Box<dyn Error>> {
    match w_env {
        Ok(env) => {
            let mut txn = env.write_txn()?;

            match w_db {
                Ok(db) => {
                    db.delete(&mut txn, key)?;
                    txn.commit()?;
                }
                Err(e) => {
                    return Err(Box::new(e.clone()));
                }
            }
        }
        Err(e) => {
            return Err(Box::new(e.clone()));
        }
    }

    Ok(())
}

fn put_kv_mdbx(w_env: &Result<Env, MdbError>, w_db: &Result<Database<Str, ByteSlice>, MdbError>, key: &str, val: &[u8]) -> Result<(), Box<dyn Error>> {
    match w_env {
        Ok(env) => {
            let mut txn = env.write_txn()?;

            match w_db {
                Ok(db) => {
                    db.put(&mut txn, key, val)?;
                    txn.commit()?;
                }
                Err(e) => {
                    return Err(Box::new(e.clone()));
                }
            }
        }
        Err(e) => {
            return Err(Box::new(e.clone()));
        }
    }

    Ok(())
}

fn get_db_of_id(db_id: StorageId, s: &MDBXStorage) -> (&Result<Env, MdbError>, &Result<Database<Str, ByteSlice>, MdbError>) {
    if db_id == StorageId::Individuals {
        ((&s.individuals_env, &s.individuals_db))
    } else if db_id == StorageId::Tickets {
        (&s.tickets_env, &s.tickets_db)
    } else if db_id == StorageId::Az {
        (&s.az_env, &s.az_db)
    } else {
        (&Err(MdbError::Panic), &Err(MdbError::Panic))
    }
}
