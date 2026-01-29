use std::io;
use v_authorization::common::{AuthorizationContext, Trace};

use crate::az_tarantool::TarantoolAzContext;
use v_authorization_impl_common::LmdbAzContext;

/// Unified authorization context supporting both LMDB and Tarantool backends
pub enum AzContext {
    Lmdb(LmdbAzContext),
    Tarantool(TarantoolAzContext),
}

impl AzContext {
    /// Create LMDB-backed context
    pub fn new_lmdb(max_read_counter: u64) -> Self {
        AzContext::Lmdb(LmdbAzContext::new(max_read_counter))
    }
    
    /// Create LMDB-backed context with full configuration
    pub fn new_lmdb_with_config(
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
    pub fn new_tarantool(uri: &str, login: &str, password: &str) -> Self {
        AzContext::Tarantool(TarantoolAzContext::new(uri, login, password))
    }
    
    /// Create Tarantool-backed context with stats
    pub fn new_tarantool_with_config(
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
            AzContext::Tarantool(ctx) => ctx.authorize_and_trace(uri, user_uri, request_access, is_check_for_reload, trace),
        }
    }
}

impl Default for AzContext {
    fn default() -> Self {
        AzContext::new_lmdb(u64::MAX)
    }
}
