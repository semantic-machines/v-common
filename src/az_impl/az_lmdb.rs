use lmdb_rs_m::core::{Database, EnvCreateNoLock, EnvCreateNoMetaSync, EnvCreateNoSync, EnvCreateReadOnly};
use lmdb_rs_m::{DbFlags, EnvBuilder, Environment, MdbError};
use std::thread;
use std::time;
use v_authorization::common::{Storage, Trace};
use v_authorization::*;
use crate::v_authorization::common::AuthorizationContext;

const DB_PATH: &str = "./data/acl-indexes/";

pub struct LmdbAzContext {
    env: Environment,
}

fn open() -> LmdbAzContext {
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

impl LmdbAzContext {
    pub fn new() -> LmdbAzContext {
        open()
    }
}

impl AuthorizationContext for LmdbAzContext {
    fn authorize(&mut self, uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool) -> Result<u8, i64> {
        let mut t = Trace {
            acl: &mut String::new(),
            is_acl: false,
            group: &mut String::new(),
            is_group: false,
            info: &mut String::new(),
            is_info: false,
            str_num: 0,
        };

        self.authorize_and_trace(uri, user_uri, request_access, _is_check_for_reload, &mut t)
    }

    fn authorize_and_trace(&mut self, uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool, trace: &mut Trace) -> Result<u8, i64> {
        match _f_authorize(&mut self.env, uri, user_uri, request_access, _is_check_for_reload, trace) {
            Ok(r) => {
                return Ok(r);
            }
            Err(e) => {
                if e < 0 {
                    info!("reopen");
                    let env_builder = EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync);

                    match env_builder.open(DB_PATH, 0o644) {
                        Ok(env1) => {
                            self.env = env1;
                        }
                        Err(e1) => {
                            error!("Authorize: Err opening environment: {:?}", e1);
                            return Err(e);
                        }
                    }
                }
            }
        }
        return _f_authorize(&mut self.env, uri, user_uri, request_access, _is_check_for_reload, trace);
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

fn _f_authorize(env: &mut Environment, uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool, trace: &mut Trace) -> Result<u8, i64> {
    let db_handle;

    match env.get_default_db(DbFlags::empty()) {
        Ok(db_handle_res) => {
            db_handle = db_handle_res;
        }
        Err(e) => {
            error!("Authorize: Err opening db handle: {:?}", e);
//            thread::sleep(time::Duration::from_secs(3));
//            error!("Retry");
            return Err(-1);
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
            return Err(-1);
        }
    }

    let db = txn.bind(&db_handle);
    let storage = AzLmdbStorage {
        db: &db,
    };

    authorize(uri, user_uri, request_access, &storage, trace)
}
