use std::io;
use v_authorization::common::{AuthorizationContext, Trace};

use crate::az_lmdb::LmdbAzContext;
#[cfg(any(feature = "tt", feature = "tt_02", feature = "tt_1"))]
use crate::az_tarantool::TarantoolAzContext;

/// Unified authorization context with runtime backend selection
pub enum AzContext {
    Lmdb(LmdbAzContext),
    #[cfg(any(feature = "tt", feature = "tt_02", feature = "tt_1"))]
    Tarantool(TarantoolAzContext),
}

impl AzContext {
    /// Create LMDB-backed context
    pub fn lmdb(max_read_counter: u64) -> Self {
        AzContext::Lmdb(LmdbAzContext::new(max_read_counter))
    }
    
    /// Create LMDB-backed context with full configuration
    pub fn lmdb_with_config(
        max_read_counter: u64,
        stat_url: Option<String>,
        stat_mode: Option<String>,
        use_cache: Option<bool>,
    ) -> Self {
        AzContext::Lmdb(LmdbAzContext::new_with_config(
            max_read_counter, stat_url, stat_mode, use_cache
        ))
    }
    
    /// Create Tarantool-backed context
    #[cfg(any(feature = "tt", feature = "tt_02", feature = "tt_1"))]
    pub fn tarantool(uri: &str, login: &str, password: &str) -> Self {
        AzContext::Tarantool(TarantoolAzContext::new(uri, login, password))
    }
    
    /// Create Tarantool-backed context with stats
    #[cfg(any(feature = "tt", feature = "tt_02", feature = "tt_1"))]
    pub fn tarantool_with_stat(
        uri: &str, 
        login: &str, 
        password: &str,
        stat_url: Option<String>,
        stat_mode: Option<String>,
    ) -> Self {
        AzContext::Tarantool(TarantoolAzContext::new_with_stat(
            uri, login, password, stat_url, stat_mode
        ))
    }
}

impl AuthorizationContext for AzContext {
    fn authorize(
        &mut self,
        uri: &str,
        user_uri: &str,
        request_access: u8,
        is_check_for_reload: bool,
    ) -> Result<u8, io::Error> {
        match self {
            AzContext::Lmdb(ctx) => ctx.authorize(uri, user_uri, request_access, is_check_for_reload),
            #[cfg(any(feature = "tt", feature = "tt_02", feature = "tt_1"))]
            AzContext::Tarantool(ctx) => ctx.authorize(uri, user_uri, request_access, is_check_for_reload),
        }
    }

    fn authorize_and_trace(
        &mut self,
        uri: &str,
        user_uri: &str,
        request_access: u8,
        is_check_for_reload: bool,
        trace: &mut Trace,
    ) -> Result<u8, io::Error> {
        match self {
            AzContext::Lmdb(ctx) => ctx.authorize_and_trace(uri, user_uri, request_access, is_check_for_reload, trace),
            #[cfg(any(feature = "tt", feature = "tt_02", feature = "tt_1"))]
            AzContext::Tarantool(ctx) => ctx.authorize_and_trace(uri, user_uri, request_access, is_check_for_reload, trace),
        }
    }
}

impl Default for AzContext {
    fn default() -> Self {
        AzContext::lmdb(u64::MAX)
    }
}
