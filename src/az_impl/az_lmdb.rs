use crate::az_impl::formats::{decode_filter, decode_rec_to_rights, decode_rec_to_rightset};
use crate::v_authorization::common::AuthorizationContext;
use chrono::{DateTime, Utc};
use io::Error;
use lmdb_rs_m::core::{Database, EnvCreateNoLock, EnvCreateNoMetaSync, EnvCreateNoSync, EnvCreateReadOnly};
use lmdb_rs_m::{DbFlags, EnvBuilder, Environment, MdbError};
use std::cmp::PartialEq;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time;
use std::time::SystemTime;
use std::{io, thread};
use v_authorization::common::{Storage, Trace};
use v_authorization::*;

const DB_PATH: &str = "./data/acl-indexes/";
const CACHE_DB_PATH: &str = "./data/acl-cache-indexes/";

use crate::az_impl::stat_manager::StatPub;
use crate::module::module_impl::Module;

#[derive(Debug, Eq, PartialEq, Clone)]
enum StatMode {
    Full,
    Minimal,
    None,
}

struct Stat {
    point: StatPub,
    mode: StatMode,
}

pub struct LmdbAzContext {
    env: Environment,
    cache_env: Option<Environment>,
    authorize_counter: u64,
    max_authorize_counter: u64,
    stat: Option<Stat>,
}

fn open(max_read_counter: u64, stat_collector_url: Option<String>, stat_mode: StatMode, use_cache: Option<bool>) -> LmdbAzContext {
    let env_builder = EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync);

    loop {
        let path: PathBuf = PathBuf::from(format!("{}{}", DB_PATH, "data.mdb"));

        if !path.exists() {
            error!("LIB_AZ: Database does not exist at path: {}", path.display());
            thread::sleep(time::Duration::from_secs(3));
            error!("Retrying database connection...");
            continue;
        }

        match env_builder.open(DB_PATH, 0o644) {
            Ok(env) => {
                info!("LIB_AZ: Opened environment at path: {}", DB_PATH);

                let stat_ctx = stat_collector_url.clone().and_then(|s| StatPub::new(&s).ok()).map(|p| Stat {
                    point: p,
                    mode: stat_mode.clone(),
                });

                if let Some(_stat) = &stat_ctx {
                    info!("LIB_AZ: Stat collector URL: {:?}", stat_collector_url);
                    info!("LIB_AZ: Stat mode: {:?}", &stat_mode);
                }

                return if use_cache.unwrap_or(false) {
                    let cache_env_builder = EnvBuilder::new().flags(EnvCreateNoLock | EnvCreateReadOnly | EnvCreateNoMetaSync | EnvCreateNoSync);
                    let cache_env = match cache_env_builder.open(CACHE_DB_PATH, 0o644) {
                        Ok(env) => {
                            info!("LIB_AZ: Opened cache environment at path: {}", CACHE_DB_PATH);
                            Some(env)
                        },
                        Err(e) => {
                            warn!("LIB_AZ: Error opening cache environment: {:?}. Proceeding without cache.", e);
                            None
                        },
                    };

                    LmdbAzContext {
                        env,
                        cache_env,
                        authorize_counter: 0,
                        max_authorize_counter: max_read_counter,
                        stat: stat_ctx,
                    }
                } else {
                    LmdbAzContext {
                        env,
                        cache_env: None,
                        authorize_counter: 0,
                        max_authorize_counter: max_read_counter,
                        stat: stat_ctx,
                    }
                };
            },
            Err(e) => {
                error!("Authorize: Error opening environment: {:?}. Retrying in 3 seconds...", e);
                thread::sleep(time::Duration::from_secs(3));
            },
        }
    }
}

impl LmdbAzContext {
    pub fn new(max_read_counter: u64) -> LmdbAzContext {
        let mode = if let Some(v) = Module::get_property::<String>("stat_mode") {
            match v.to_lowercase().as_str() {
                "full" => StatMode::Full,
                "minimal" => StatMode::Minimal,
                "off" => StatMode::None,
                "none" => StatMode::None,
                _ => StatMode::Full,
            }
        } else {
            StatMode::Full
        };

        let stat_collector_url = Module::get_property("stat_collector_url");
        let use_authorization_cache = Module::get_property("use_authorization_cache");

        open(max_read_counter, stat_collector_url, mode, use_authorization_cache)
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

        let start_time = SystemTime::now();

        let r = self.authorize_and_trace(uri, user_uri, request_access, _is_check_for_reload, &mut t);

        if let Some(stat) = &mut self.stat {
            if stat.mode == StatMode::Full || stat.mode == StatMode::Minimal {
                let elapsed = start_time.elapsed().unwrap_or_default();
                stat.point.set_duration(elapsed);
                if let Err(e) = stat.point.flush() {
                    warn!("fail flush stat, err={:?}", e);
                }
            }
        }

        r
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
    stat: &'a mut Option<Stat>,
}

fn message(key: &str, use_cache: bool, from_cache: bool) -> String {
    match (use_cache, from_cache) {
        (true, true) => format!("{}/C", key),
        (true, false) => format!("{}/cB", key),
        (false, _) => format!("{}/B", key),
    }
}

impl<'a> Storage for AzLmdbStorage<'a> {
    fn get(&mut self, key: &str) -> io::Result<Option<String>> {
        if let Some(cache_db) = self.cache_db {
            match cache_db.get::<String>(&key) {
                Ok(val) => {
                    if let Some(stat) = self.stat {
                        if stat.mode == StatMode::Full {
                            stat.point.collect(message(key, true, true));
                        }
                    }
                    debug!("@cache val={}", val);
                    return Ok(Some(val));
                },
                Err(e) => match e {
                    MdbError::NotFound => {
                        // Данные не найдены в кеше, продолжаем чтение из основной базы
                    },
                    _ => {},
                },
            }
        }

        match self.db.get::<String>(&key) {
            Ok(val) => {
                if let Some(stat) = self.stat {
                    if stat.mode == StatMode::Full {
                        stat.point.collect(message(key, self.cache_db.is_some(), false));
                    }
                }
                debug!("@db val={}", val);
                Ok(Some(val))
            },
            Err(e) => match e {
                MdbError::NotFound => Ok(None),
                _ => Err(Error::new(ErrorKind::Other, format!("Authorize: db.get {:?}, {}", e, key))),
            },
        }
    }

    fn fiber_yield(&self) {}

    fn decode_rec_to_rights(&self, src: &str, result: &mut Vec<ACLRecord>) -> (bool, Option<DateTime<Utc>>) {
        decode_rec_to_rights(src, result)
    }

    fn decode_rec_to_rightset(&self, src: &str, new_rights: &mut ACLRecordSet) -> (bool, Option<DateTime<Utc>>) {
        decode_rec_to_rightset(src, new_rights)
    }

    fn decode_filter(&self, filter_value: String) -> (Option<ACLRecord>, Option<DateTime<Utc>>) {
        decode_filter(filter_value)
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
