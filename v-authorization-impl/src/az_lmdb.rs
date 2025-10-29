use v_authorization::record_formats::{decode_filter, decode_rec_to_rights, decode_rec_to_rightset};
use chrono::{DateTime, Utc};
use io::Error;
use heed::{Env, EnvOpenOptions, Database, RoTxn};
use heed::types::Str;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time;
use std::{io, thread};
use v_authorization::common::{Storage, Trace};
use v_authorization::*;

const DB_PATH: &str = "./data/acl-indexes/";
const CACHE_DB_PATH: &str = "./data/acl-cache-indexes/";

use crate::stat_manager::{StatPub, StatMode, Stat, parse_stat_mode, format_stat_message};
use crate::common::AuthorizationHelper;
use crate::impl_authorization_context;

// Global shared environments for multi-threaded access
static GLOBAL_ENV: OnceLock<Arc<Env>> = OnceLock::new();
static GLOBAL_CACHE_ENV: OnceLock<Arc<Env>> = OnceLock::new();

pub struct LmdbAzContext {
    env: Arc<Env>,
    cache_env: Option<Arc<Env>>,
    authorize_counter: u64,
    max_authorize_counter: u64,
    stat: Option<Stat>,
}

fn open(max_read_counter: u64, stat_collector_url: Option<String>, stat_mode: StatMode, use_cache: Option<bool>) -> LmdbAzContext {
    // Get or initialize global environment (shared across all threads)
    let env = GLOBAL_ENV.get_or_init(|| {
        loop {
            let path: PathBuf = PathBuf::from(format!("{}{}", DB_PATH, "data.mdb"));

            if !path.exists() {
                error!("LIB_AZ: Database does not exist at path: {}", path.display());
                thread::sleep(time::Duration::from_secs(3));
                error!("Retrying database connection...");
                continue;
            }

            match unsafe { EnvOpenOptions::new().max_dbs(1).open(DB_PATH) } {
                Ok(env) => {
                    info!("LIB_AZ: Opened shared environment at path: {}", DB_PATH);
                    return Arc::new(env);
                },
                Err(e) => {
                    error!("Authorize: Error opening environment: {:?}. Retrying in 3 seconds...", e);
                    thread::sleep(time::Duration::from_secs(3));
                },
            }
        }
    }).clone();

    let stat_ctx = stat_collector_url.clone().and_then(|s| StatPub::new(&s).ok()).map(|p| Stat {
        point: p,
        mode: stat_mode.clone(),
    });

    if let Some(_stat) = &stat_ctx {
        info!("LIB_AZ: Stat collector URL: {:?}", stat_collector_url);
        info!("LIB_AZ: Stat mode: {:?}", &stat_mode);
    }

    let cache_env = if use_cache.unwrap_or(false) {
        // Get or try to initialize cache environment
        let cache_result = GLOBAL_CACHE_ENV.get_or_init(|| {
            match unsafe { EnvOpenOptions::new().max_dbs(1).open(CACHE_DB_PATH) } {
                Ok(env) => {
                    info!("LIB_AZ: Opened shared cache environment at path: {}", CACHE_DB_PATH);
                    Arc::new(env)
                },
                Err(e) => {
                    warn!("LIB_AZ: Error opening cache environment: {:?}. Cache will not be used.", e);
                    // Use a closed/empty environment as marker
                    Arc::new(unsafe { EnvOpenOptions::new().max_dbs(1).open(CACHE_DB_PATH).unwrap() })
                },
            }
        });
        Some(cache_result.clone())
    } else {
        None
    };

    LmdbAzContext {
        env,
        cache_env,
        authorize_counter: 0,
        max_authorize_counter: max_read_counter,
        stat: stat_ctx,
    }
}

impl LmdbAzContext {
    pub fn new_with_config(max_read_counter: u64, stat_collector_url: Option<String>, stat_mode_str: Option<String>, use_cache: Option<bool>) -> LmdbAzContext {
        let mode = parse_stat_mode(stat_mode_str);
        open(max_read_counter, stat_collector_url, mode, use_cache)
    }

    pub fn new(max_read_counter: u64) -> LmdbAzContext {
        open(max_read_counter, None, StatMode::None, None)
    }
}

impl Default for LmdbAzContext {
    fn default() -> Self {
        Self::new(u64::MAX)
    }
}

impl AuthorizationHelper for LmdbAzContext {
    fn get_stat_mut(&mut self) -> &mut Option<Stat> {
        &mut self.stat
    }

    fn get_authorize_counter(&self) -> u64 {
        self.authorize_counter
    }

    fn get_max_authorize_counter(&self) -> u64 {
        self.max_authorize_counter
    }

    fn set_authorize_counter(&mut self, value: u64) {
        self.authorize_counter = value;
    }

    fn authorize_use_db(
        &mut self,
        uri: &str,
        user_uri: &str,
        request_access: u8,
        _is_check_for_reload: bool,
        trace: &mut Trace,
    ) -> Result<u8, std::io::Error> {
        let txn = match self.env.read_txn() {
            Ok(txn1) => txn1,
            Err(e) => {
                return Err(Error::other(format!("Authorize:CREATING TRANSACTION {:?}", e)));
            },
        };

        let db: Database<Str, Str> = match self.env.open_database(&txn, None) {
            Ok(Some(db_res)) => db_res,
            Ok(None) => {
                return Err(Error::other("Authorize: database not found"));
            },
            Err(e) => {
                return Err(Error::other(format!("Authorize: Err opening database: {:?}", e)));
            },
        };

        let (cache_txn_owned, cache_db, cache_txn_ref) = if let Some(env) = &self.cache_env {
            let txn_cache = match env.read_txn() {
                Ok(txn1) => txn1,
                Err(e) => {
                    return Err(Error::other(format!("Authorize:CREATING CACHE TRANSACTION {:?}", e)));
                },
            };
            
            let db = match env.open_database(&txn_cache, None) {
                Ok(Some(db_res)) => Some(db_res),
                Ok(None) => {
                    warn!("Authorize: cache database not found");
                    None
                },
                Err(e) => {
                    warn!("Authorize: Err opening cache database: {:?}", e);
                    None
                },
            };
            
            (Some(txn_cache), db, true)
        } else {
            (None, None, false)
        };

        let cache_txn_ptr = if cache_txn_ref { 
            cache_txn_owned.as_deref()
        } else { 
            None 
        };
        
        let mut storage = AzLmdbStorage {
            txn: &txn,
            db,
            cache_txn: cache_txn_ptr,
            cache_db,
            stat: &mut self.stat,
        };

        authorize(uri, user_uri, request_access, &mut storage, trace)
    }
}

impl_authorization_context!(LmdbAzContext);

pub struct AzLmdbStorage<'a> {
    txn: &'a RoTxn<'a>,
    db: Database<Str, Str>,
    cache_txn: Option<&'a RoTxn<'a>>,
    cache_db: Option<Database<Str, Str>>,
    stat: &'a mut Option<Stat>,
}

impl<'a> Storage for AzLmdbStorage<'a> {
    fn get(&mut self, key: &str) -> io::Result<Option<String>> {
        if let Some(cache_db) = self.cache_db {
            if let Some(cache_txn) = self.cache_txn {
                match cache_db.get(cache_txn, key) {
                    Ok(Some(val)) => {
                        if let Some(stat) = self.stat {
                            if stat.mode == StatMode::Full {
                                stat.point.collect(format_stat_message(key, true, true));
                            }
                        }
                        debug!("@cache val={}", val);
                        return Ok(Some(val.to_string()));
                    },
                    Ok(None) => {
                        // Data not found in cache, continue reading from main database
                    },
                    Err(_e) => {
                        // Error reading cache, continue reading from main database
                    },
                }
            }
        }

        match self.db.get(self.txn, key) {
            Ok(Some(val)) => {
                if let Some(stat) = self.stat {
                    if stat.mode == StatMode::Full {
                        stat.point.collect(format_stat_message(key, self.cache_db.is_some(), false));
                    }
                }
                debug!("@db val={}", val);
                Ok(Some(val.to_string()))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(Error::other(format!("Authorize: db.get {:?}, {}", e, key))),
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

