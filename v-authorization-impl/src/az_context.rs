use std::io;
use v_authorization::common::{AuthorizationContext, Trace};

#[cfg(feature = "az_lmdb")]
use crate::az_lmdb::LmdbAzContext;
#[cfg(any(feature = "az_tt_2", feature = "az_tt_3"))]
use crate::az_tarantool::TarantoolAzContext;

/// Unified authorization context with backend selection at compile time
pub enum AzContext {
    #[cfg(feature = "az_lmdb")]
    Lmdb(LmdbAzContext),
    #[cfg(any(feature = "az_tt_2", feature = "az_tt_3"))]
    Tarantool(TarantoolAzContext),
}

impl AzContext {
    /// Create LMDB-backed context
    #[cfg(feature = "az_lmdb")]
    pub fn new(max_read_counter: u64) -> Self {
        AzContext::Lmdb(LmdbAzContext::new(max_read_counter))
    }
    
    /// Create LMDB-backed context with full configuration
    #[cfg(feature = "az_lmdb")]
    pub fn new_with_config(
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
    #[cfg(any(feature = "az_tt_2", feature = "az_tt_3"))]
    pub fn new(uri: &str, login: &str, password: &str) -> Self {
        AzContext::Tarantool(TarantoolAzContext::new(uri, login, password))
    }
    
    /// Create Tarantool-backed context with stats
    #[cfg(any(feature = "az_tt_2", feature = "az_tt_3"))]
    pub fn new_with_config(
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
            #[cfg(feature = "az_lmdb")]
            AzContext::Lmdb(ctx) => ctx.authorize(uri, user_uri, request_access, is_check_for_reload),
            #[cfg(any(feature = "az_tt_2", feature = "az_tt_3"))]
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
            #[cfg(feature = "az_lmdb")]
            AzContext::Lmdb(ctx) => ctx.authorize_and_trace(uri, user_uri, request_access, is_check_for_reload, trace),
            #[cfg(any(feature = "az_tt_2", feature = "az_tt_3"))]
            AzContext::Tarantool(ctx) => ctx.authorize_and_trace(uri, user_uri, request_access, is_check_for_reload, trace),
        }
    }
}

#[cfg(feature = "az_lmdb")]
impl Default for AzContext {
    fn default() -> Self {
        AzContext::new(u64::MAX)
    }
}
