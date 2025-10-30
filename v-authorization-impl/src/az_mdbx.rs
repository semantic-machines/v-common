use v_authorization::record_formats::{decode_filter, decode_rec_to_rights, decode_rec_to_rightset};
use chrono::{DateTime, Utc};
use io::Error;
use libmdbx::{Database, NoWriteMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::LazyLock;
use std::time;
use std::{io, thread};
use v_authorization::common::{Storage, Trace};
use v_authorization::*;

const DB_PATH: &str = "./data/acl-mdbx-indexes/";
const CACHE_DB_PATH: &str = "./data/acl-cache-mdbx-indexes/";

use crate::stat_manager::{StatPub, StatMode, Stat, parse_stat_mode, format_stat_message};
use crate::common::AuthorizationHelper;
use crate::impl_authorization_context;

// Global shared databases for multi-threaded access
static GLOBAL_DB: LazyLock<Mutex<Option<Arc<Database<NoWriteMap>>>>> = LazyLock::new(|| Mutex::new(None));
static GLOBAL_CACHE_DB: LazyLock<Mutex<Option<Arc<Database<NoWriteMap>>>>> = LazyLock::new(|| Mutex::new(None));

// Reset global databases (useful for tests)
// This drops the Arc references and allows the database to be fully closed
pub fn reset_global_envs() {
    let mut db = GLOBAL_DB.lock().unwrap();
    *db = None;
    
    let mut cache_db = GLOBAL_CACHE_DB.lock().unwrap();
    *cache_db = None;
    
    info!("LIB_AZ_MDBX: Reset global databases");
}

// Helper function to force sync of database
// This ensures data is written to disk before reopening
pub fn sync_env() -> bool {
    let db_opt = GLOBAL_DB.lock().unwrap();
    if let Some(db) = db_opt.as_ref() {
        match db.sync(true) {
            Ok(_) => {
                info!("LIB_AZ_MDBX: Successfully synced database");
                true
            },
            Err(e) => {
                error!("LIB_AZ_MDBX: Failed to sync database: {:?}", e);
                false
            }
        }
    } else {
        true // No database to sync
    }
}

pub struct MdbxAzContext {
    db: Arc<Database<NoWriteMap>>,
    cache_db: Option<Arc<Database<NoWriteMap>>>,
    authorize_counter: u64,
    max_authorize_counter: u64,
    stat: Option<Stat>,
}

fn open(max_read_counter: u64, stat_collector_url: Option<String>, stat_mode: StatMode, use_cache: Option<bool>) -> MdbxAzContext {
    // Get or initialize global database (shared across all threads)
    let db = {
        let mut db_lock = GLOBAL_DB.lock().unwrap();
        
        if let Some(existing_db) = db_lock.as_ref() {
            existing_db.clone()
        } else {
            // Create new database
            let new_db = loop {
                let path: PathBuf = PathBuf::from(DB_PATH);

                if !path.exists() {
                    error!("LIB_AZ: Database directory does not exist at path: {}", path.display());
                    thread::sleep(time::Duration::from_secs(3));
                    error!("Retrying database connection...");
                    continue;
                }

                match Database::open(&path) {
                    Ok(database) => {
                        info!("LIB_AZ: Opened shared MDBX database at path: {}", DB_PATH);
                        break Arc::new(database);
                    },
                    Err(e) => {
                        error!("Authorize: Error opening MDBX database: {:?}. Retrying in 3 seconds...", e);
                        thread::sleep(time::Duration::from_secs(3));
                    },
                }
            };
            
            *db_lock = Some(new_db.clone());
            new_db
        }
    };

    let stat_ctx = stat_collector_url.clone().and_then(|s| StatPub::new(&s).ok()).map(|p| Stat {
        point: p,
        mode: stat_mode.clone(),
    });

    if let Some(_stat) = &stat_ctx {
        info!("LIB_AZ: Stat collector URL: {:?}", stat_collector_url);
        info!("LIB_AZ: Stat mode: {:?}", &stat_mode);
    }

    let cache_db = if use_cache.unwrap_or(false) {
        let mut cache_lock = GLOBAL_CACHE_DB.lock().unwrap();
        
        if let Some(existing_cache) = cache_lock.as_ref() {
            Some(existing_cache.clone())
        } else {
            // Try to initialize cache database
            let path: PathBuf = PathBuf::from(CACHE_DB_PATH);
            match Database::open(&path) {
                Ok(database) => {
                    info!("LIB_AZ: Opened shared MDBX cache database at path: {}", CACHE_DB_PATH);
                    let arc_db = Arc::new(database);
                    *cache_lock = Some(arc_db.clone());
                    Some(arc_db)
                },
                Err(e) => {
                    warn!("LIB_AZ: Error opening MDBX cache database: {:?}. Cache will not be used.", e);
                    None
                },
            }
        }
    } else {
        None
    };

    MdbxAzContext {
        db,
        cache_db,
        authorize_counter: 0,
        max_authorize_counter: max_read_counter,
        stat: stat_ctx,
    }
}

impl MdbxAzContext {
    pub fn new_with_config(max_read_counter: u64, stat_collector_url: Option<String>, stat_mode_str: Option<String>, use_cache: Option<bool>) -> MdbxAzContext {
        let mode = parse_stat_mode(stat_mode_str);
        open(max_read_counter, stat_collector_url, mode, use_cache)
    }

    pub fn new(max_read_counter: u64) -> MdbxAzContext {
        open(max_read_counter, None, StatMode::None, None)
    }
}

impl Default for MdbxAzContext {
    fn default() -> Self {
        Self::new(u64::MAX)
    }
}

impl AuthorizationHelper for MdbxAzContext {
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
        let mut storage = AzMdbxStorage {
            db: &self.db,
            cache_db: self.cache_db.as_deref(),
            stat: &mut self.stat,
        };

        authorize(uri, user_uri, request_access, &mut storage, trace)
    }
}

impl_authorization_context!(MdbxAzContext);

pub struct AzMdbxStorage<'a> {
    db: &'a Database<NoWriteMap>,
    cache_db: Option<&'a Database<NoWriteMap>>,
    stat: &'a mut Option<Stat>,
}

impl<'a> AzMdbxStorage<'a> {
    // Helper function to read from database with less nesting
    fn read_from_db(&mut self, db: &Database<NoWriteMap>, key: &str, from_cache: bool) -> io::Result<Option<String>> {
        let txn = db.begin_ro_txn()
            .map_err(|e| Error::other(format!("Authorize: failed to begin transaction {:?}", e)))?;
        
        let table = txn.open_table(None)
            .map_err(|e| Error::other(format!("Authorize: failed to open table {:?}", e)))?;
        
        match txn.get::<Vec<u8>>(&table, key.as_bytes()) {
            Ok(Some(val)) => {
                let val_str = std::str::from_utf8(&val)
                    .map_err(|_| Error::other("Failed to decode UTF-8"))?;
                
                if let Some(stat) = self.stat {
                    if stat.mode == StatMode::Full {
                        stat.point.collect(format_stat_message(key, self.cache_db.is_some(), from_cache));
                    }
                }
                
                if from_cache {
                    debug!("@cache val={}", val_str);
                } else {
                    debug!("@db val={}", val_str);
                }
                
                Ok(Some(val_str.to_string()))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(Error::other(format!("Authorize: db.get {:?}, {}", e, key))),
        }
    }
}

impl<'a> Storage for AzMdbxStorage<'a> {
    fn get(&mut self, key: &str) -> io::Result<Option<String>> {
        // Try to read from cache first
        if let Some(cache_db) = self.cache_db {
            if let Ok(Some(value)) = self.read_from_db(cache_db, key, true) {
                return Ok(Some(value));
            }
            // If cache read failed or returned None, continue to main database
        }

        // Read from main database
        self.read_from_db(self.db, key, false)
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


