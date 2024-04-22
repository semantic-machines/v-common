use crate::az_impl::formats::{decode_rec_to_rights, decode_rec_to_rightset};
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
const CACHE_DB_PATH: &str = "./data/acl-cache-indexes/";

use crate::az_impl::stat_manager::StatPub;
use crate::module::module_impl::Module;

pub struct LmdbAzContext {
    env: Environment,
    cache_env: Option<Environment>,
    authorize_counter: u64,
    max_authorize_counter: u64,
    stat: Option<StatPub>,
}

fn open(max_read_counter: u64, stat_collector_url: Option<String>) -> LmdbAzContext {
    let env_builder = EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync);
    let stat = if let Some(s) = stat_collector_url {
        StatPub::new(&s).ok()
    } else {
        None
    };
    loop {
        match env_builder.open(DB_PATH, 0o644) {
            Ok(env) => {
                info!("LIB_AZ: Opened environment {}", DB_PATH);
                let cache_env_builder = EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync);
                let cache_env = match cache_env_builder.open(CACHE_DB_PATH, 0o644) {
                    Ok(env) => {
                        info!("LIB_AZ: Opened cache environment {}", CACHE_DB_PATH);
                        Some(env)
                    },
                    Err(e) => {
                        warn!("Authorize: Err opening cache environment: {:?}. Proceeding without cache.", e);
                        None
                    },
                };

                return LmdbAzContext {
                    env,
                    cache_env,
                    authorize_counter: 0,
                    max_authorize_counter: max_read_counter,
                    stat,
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
        open(max_read_counter, Module::get_property("stat_collector_url"))
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

        let r = self.authorize_and_trace(uri, user_uri, request_access, _is_check_for_reload, &mut t);

        if let Some(s) = &mut self.stat {
            if let Err(e) = s.flush() {
                warn!("fail flush stat, err={:?}", e);
            }
        }

        return r;
    }

    fn authorize_and_trace(&mut self, uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool, trace: &mut Trace) -> Result<u8, std::io::Error> {
        self.authorize_counter += 1;
        //info!("az counter={}", self.authorize_counter);
        if self.authorize_counter >= self.max_authorize_counter {
            //info!("az reopen, counter > {}", self.max_authorize_counter);
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

        match self.authorize_use_db(uri, user_uri, request_access, _is_check_for_reload, trace) {
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
        // retry authorization if db err
        self.authorize_use_db(uri, user_uri, request_access, _is_check_for_reload, trace)
    }
}

pub struct AzLmdbStorage<'a> {
    db: &'a Database<'a>,
    cache_db: Option<&'a Database<'a>>,
    stat: &'a mut Option<StatPub>,
}

impl<'a> Storage for AzLmdbStorage<'a> {
    fn get(&mut self, key: &str) -> io::Result<Option<String>> {
        if let Some(cache_db) = self.cache_db {
            match cache_db.get::<String>(&key) {
                Ok(val) => {
                    if let Some(p) = self.stat {
                        p.collect(key);
                    }
                    return Ok(Some(val));
                },
                Err(e) => match e {
                    MdbError::NotFound => {
                        // Данные не найдены в кеше, продолжаем чтение из основной базы
                    },
                    _ => return Err(Error::new(ErrorKind::Other, format!("Authorize: cache_db.get {:?}, {}", e, key))),
                },
            }
        }

        match self.db.get::<String>(&key) {
            Ok(val) => {
                if let Some(p) = self.stat {
                    p.collect(key);
                }
                Ok(Some(val))
            },
            Err(e) => match e {
                MdbError::NotFound => Ok(None),
                _ => Err(Error::new(ErrorKind::Other, format!("Authorize: db.get {:?}, {}", e, key))),
            },
        }
    }

    fn fiber_yield(&self) {}

    fn decode_rec_to_rights(&self, src: &str, result: &mut Vec<Right>) -> bool {
        decode_rec_to_rights(src, result)
    }

    fn decode_rec_to_rightset(&self, src: &str, new_rights: &mut RightSet) -> bool {
        decode_rec_to_rightset(src, new_rights)
    }
}

impl LmdbAzContext {
    fn authorize_use_db(&mut self, uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool, trace: &mut Trace) -> Result<u8, std::io::Error> {
        let db_handle = match self.env.get_default_db(DbFlags::empty()) {
            Ok(db_handle_res) => db_handle_res,
            Err(e) => {
                return Err(Error::new(ErrorKind::Other, format!("Authorize: Err opening db handle: {:?}", e)));
            },
        };
        let txn = match self.env.get_reader() {
            Ok(txn1) => txn1,
            Err(e) => {
                return Err(Error::new(ErrorKind::Other, format!("Authorize:CREATING TRANSACTION {:?}", e)));
            },
        };
        let db = txn.bind(&db_handle);

        let txn_cache;
        let cache_db = if let Some(env) = &self.cache_env {
            let db_handle = match env.get_default_db(DbFlags::empty()) {
                Ok(db_handle_res) => db_handle_res,
                Err(e) => {
                    return Err(Error::new(ErrorKind::Other, format!("Authorize: Err opening db handle: {:?}", e)));
                },
            };
            txn_cache = match env.get_reader() {
                Ok(txn1) => txn1,
                Err(e) => {
                    return Err(Error::new(ErrorKind::Other, format!("Authorize:CREATING TRANSACTION {:?}", e)));
                },
            };
            let cache_db = txn_cache.bind(&db_handle);
            Some(cache_db)
        } else {
            None
        };

        let mut storage = AzLmdbStorage {
            db: &db,
            cache_db: cache_db.as_ref(),
            stat: &mut self.stat,
        };

        authorize(uri, user_uri, request_access, &mut storage, trace)
    }
}
