use lmdb_rs_m::core::{Database, EnvCreateNoLock, EnvCreateNoMetaSync, EnvCreateNoSync, EnvCreateReadOnly};
use lmdb_rs_m::{DbFlags, EnvBuilder, Environment, MdbError};
use std::thread;
use std::time;
use v_authorization::common::{AuthorizationContext, Storage, Trace};
use v_authorization::*;

const DB_PATH: &str = "./data/acl-indexes/";

pub struct LmdbAzContext {
    env: Environment,
}

impl LmdbAzContext {
    pub fn new() -> LmdbAzContext {
        let env_builder = EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync);
        loop {
            match env_builder.open(DB_PATH, 0o644) {
                Ok(env) => {
                    info!("LIB_AZ: Opened environment {}", DB_PATH);
                    return LmdbAzContext {
                        env,
                    };
                }
                Err(e) => {
                    error!("Authorize: Err opening environment: {:?}", e);
                    thread::sleep(time::Duration::from_secs(3));
                    error!("Retry");
                }
            }
        }
    }
}

impl AuthorizationContext for LmdbAzContext {
    fn authorize(&mut self, uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool, trace: Option<&mut Trace>) -> Result<u8, i64> {
        _f_authorize(&mut self.env, uri, user_uri, request_access, _is_check_for_reload, trace)
    }
}

pub struct AzLmdbStorage<'a> {
    db: &'a Database<'a>,
}

impl<'a> Storage for AzLmdbStorage<'a> {
    fn get(&self, key: &str) -> Result<String, i64> {
        match self.db.get::<String>(&key) {
            Ok(val) => Ok(val),
            Err(e) => match e {
                MdbError::NotFound => Err(0),
                _ => {
                    error!("Authorize: db.get {:?}, {}", e, key);
                    Err(-1)
                }
            },
        }
    }

    fn fiber_yield(&self) {}
}

pub fn _f_authorize(env: &Environment, uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool, trace: Option<&mut Trace>) -> Result<u8, i64> {
    let db_handle;
    loop {
        match env.get_default_db(DbFlags::empty()) {
            Ok(db_handle_res) => {
                db_handle = db_handle_res;
                break;
            }
            Err(e) => {
                error!("Authorize: Err opening db handle: {:?}", e);
                thread::sleep(time::Duration::from_secs(3));
                error!("Retry");
            }
        }
    }

    let txn;
    match env.get_reader() {
        Ok(txn1) => {
            txn = txn1;
        }
        Err(e) => {
            error!("Authorize:CREATING TRANSACTION {:?}", e);
            error!("reopen db");

            let env_builder = EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync);

            match env_builder.open(DB_PATH, 0o644) {
                Ok(env1) => {
                    return _f_authorize(&env1, uri, user_uri, request_access, _is_check_for_reload, trace);
                }
                Err(e) => {
                    error!("Authorize: Err opening environment: {:?}", e);
                }
            }

            return Err(-1);
        }
    }

    let db = txn.bind(&db_handle);
    let storage = AzLmdbStorage {
        db: &db,
    };

    if let Some(t) = trace {
        authorize(uri, user_uri, request_access, &storage, t)
    } else {
        let mut t = Trace {
            acl: &mut String::new(),
            is_acl: false,
            group: &mut String::new(),
            is_group: false,
            info: &mut String::new(),
            is_info: false,
            str_num: 0,
        };

        authorize(uri, user_uri, request_access, &storage, &mut t)
    }
}
