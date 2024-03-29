use crate::v_authorization::common::AuthorizationContext;
use io::Error;
use lmdb_rs_m::core::{Database, EnvCreateNoLock, EnvCreateNoMetaSync, EnvCreateNoSync, EnvCreateReadOnly};
use lmdb_rs_m::{DbFlags, EnvBuilder, Environment, MdbError};
use std::io::ErrorKind;
use std::time;
use std::{io, thread};
use v_authorization::common::{Storage, Trace};
use v_authorization::*;

const DB_PATH: &str = "./data/acl-indexes/";

pub struct LmdbAzContext {
    env: Environment,
    authorize_counter: u64,
    max_authorize_counter: u64,
}

fn open(max_read_counter: u64) -> LmdbAzContext {
    let env_builder = EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync);
    loop {
        match env_builder.open(DB_PATH, 0o644) {
            Ok(env) => {
                info!("LIB_AZ: Opened environment {}", DB_PATH);
                return LmdbAzContext {
                    env,
                    authorize_counter: 0,
                    max_authorize_counter: max_read_counter,
                };
            },
            Err(e) => {
                error!("Authorize: Err opening environment: {:?}", e);
                thread::sleep(time::Duration::from_secs(3));
                error!("Retry");
            },
        }
    }
}

impl LmdbAzContext {
    pub fn new(max_read_counter: u64) -> LmdbAzContext {
        open(max_read_counter)
    }
}

impl Default for LmdbAzContext {
    fn default() -> Self {
        Self::new(u64::MAX)
    }
}

impl AuthorizationContext for LmdbAzContext {
    fn authorize(&mut self, uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool) -> Result<u8, std::io::Error> {
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

    fn authorize_and_trace(&mut self, uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool, trace: &mut Trace) -> Result<u8, std::io::Error> {
        self.authorize_counter += 1;
        //info!("az counter={}", self.authorize_counter);
        if self.authorize_counter >= self.max_authorize_counter {
            info!("az reopen, counter > {}", self.max_authorize_counter);
            self.authorize_counter = 0;
            let env_builder = EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync);

            match env_builder.open(DB_PATH, 0o644) {
                Ok(env1) => {
                    self.env = env1;
                },
                Err(e1) => {
                    return Err(Error::new(ErrorKind::Other, format!("Authorize: Err opening environment: {:?}", e1)));
                },
            }
        }

        match _f_authorize(&mut self.env, uri, user_uri, request_access, _is_check_for_reload, trace) {
            Ok(r) => {
                return Ok(r);
            },
            Err(e) => {
                info!("reopen");
                let env_builder = EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync);

                match env_builder.open(DB_PATH, 0o644) {
                    Ok(env1) => {
                        self.env = env1;
                    },
                    Err(e1) => {
                        error!("Authorize: Err opening environment: {:?}", e1);
                        return Err(e);
                    },
                }
            },
        }
        _f_authorize(&mut self.env, uri, user_uri, request_access, _is_check_for_reload, trace)
    }
}

pub struct AzLmdbStorage<'a> {
    db: &'a Database<'a>,
}

impl<'a> Storage for AzLmdbStorage<'a> {
    fn get(&self, key: &str) -> io::Result<Option<String>> {
        match self.db.get::<String>(&key) {
            Ok(val) => Ok(Some(val)),
            Err(e) => match e {
                MdbError::NotFound => Ok(None),
                _ => Err(Error::new(ErrorKind::Other, format!("Authorize: db.get {:?}, {}", e, key))),
            },
        }
    }

    fn fiber_yield(&self) {}
}

fn _f_authorize(env: &mut Environment, uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool, trace: &mut Trace) -> Result<u8, std::io::Error> {
    let db_handle = match env.get_default_db(DbFlags::empty()) {
        Ok(db_handle_res) => db_handle_res,
        Err(e) => {
            return Err(Error::new(ErrorKind::Other, format!("Authorize: Err opening db handle: {:?}", e)));
        },
    };

    let txn = match env.get_reader() {
        Ok(txn1) => txn1,
        Err(e) => {
            return Err(Error::new(ErrorKind::Other, format!("Authorize:CREATING TRANSACTION {:?}", e)));
        },
    };

    let db = txn.bind(&db_handle);
    let storage = AzLmdbStorage {
        db: &db,
    };

    authorize(uri, user_uri, request_access, &storage, trace)
}
