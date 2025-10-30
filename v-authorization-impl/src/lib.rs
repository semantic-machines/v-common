#[macro_use]
extern crate log;

mod az_lmdb;
mod az_mdbx;
mod stat_manager;
mod common;

use az_lmdb::LmdbAzContext;
use az_mdbx::MdbxAzContext;

pub use v_authorization;

// Re-export functions for test usage
pub use az_lmdb::{reset_global_envs as reset_lmdb_global_envs, sync_env as sync_lmdb_env};
pub use az_mdbx::{reset_global_envs as reset_mdbx_global_envs, sync_env as sync_mdbx_env};

use v_authorization::common::AuthorizationContext;

// Database backend type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AzDbType {
    Lmdb,
    Mdbx,
}

// Unified authorization context that wraps either implementation
pub enum AzContext {
    Lmdb(LmdbAzContext),
    Mdbx(MdbxAzContext),
}

impl AzContext {
    pub fn new(db_type: AzDbType, max_read_counter: u64) -> Self {
        match db_type {
            AzDbType::Lmdb => AzContext::Lmdb(LmdbAzContext::new(max_read_counter)),
            AzDbType::Mdbx => AzContext::Mdbx(MdbxAzContext::new(max_read_counter)),
        }
    }

    pub fn new_with_config(
        db_type: AzDbType,
        max_read_counter: u64,
        stat_collector_url: Option<String>,
        stat_mode_str: Option<String>,
        use_cache: Option<bool>,
    ) -> Self {
        match db_type {
            AzDbType::Lmdb => AzContext::Lmdb(LmdbAzContext::new_with_config(
                max_read_counter,
                stat_collector_url,
                stat_mode_str,
                use_cache,
            )),
            AzDbType::Mdbx => AzContext::Mdbx(MdbxAzContext::new_with_config(
                max_read_counter,
                stat_collector_url,
                stat_mode_str,
                use_cache,
            )),
        }
    }
}

impl AuthorizationContext for AzContext {
    fn authorize(
        &mut self,
        uri: &str,
        user_uri: &str,
        request_access: u8,
        is_check_for_reload: bool,
    ) -> Result<u8, std::io::Error> {
        match self {
            AzContext::Lmdb(ctx) => ctx.authorize(uri, user_uri, request_access, is_check_for_reload),
            AzContext::Mdbx(ctx) => ctx.authorize(uri, user_uri, request_access, is_check_for_reload),
        }
    }

    fn authorize_and_trace(
        &mut self,
        uri: &str,
        user_uri: &str,
        request_access: u8,
        is_check_for_reload: bool,
        trace: &mut v_authorization::common::Trace,
    ) -> Result<u8, std::io::Error> {
        match self {
            AzContext::Lmdb(ctx) => ctx.authorize_and_trace(uri, user_uri, request_access, is_check_for_reload, trace),
            AzContext::Mdbx(ctx) => ctx.authorize_and_trace(uri, user_uri, request_access, is_check_for_reload, trace),
        }
    }
}

