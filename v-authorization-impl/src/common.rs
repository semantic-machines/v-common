use v_authorization::common::Trace;
use std::time::SystemTime;
use crate::stat_manager::{Stat, StatMode};

// Trait for types that have authorization and tracing capabilities
pub(crate) trait AuthorizationHelper {
    fn get_stat_mut(&mut self) -> &mut Option<Stat>;
    fn get_authorize_counter(&self) -> u64;
    fn get_max_authorize_counter(&self) -> u64;
    fn set_authorize_counter(&mut self, value: u64);
    
    // Implementation-specific method to perform actual authorization using database
    fn authorize_use_db(
        &mut self,
        uri: &str,
        user_uri: &str,
        request_access: u8,
        is_check_for_reload: bool,
        trace: &mut Trace,
    ) -> Result<u8, std::io::Error>;
    
    // Default implementation that handles counter logic and retry
    fn authorize_and_trace_impl(
        &mut self,
        uri: &str,
        user_uri: &str,
        request_access: u8,
        is_check_for_reload: bool,
        trace: &mut Trace,
    ) -> Result<u8, std::io::Error> {
        let counter = self.get_authorize_counter() + 1;
        self.set_authorize_counter(counter);
        
        if counter >= self.get_max_authorize_counter() {
            self.set_authorize_counter(0);
        }

        match self.authorize_use_db(uri, user_uri, request_access, is_check_for_reload, trace) {
            Ok(r) => {
                return Ok(r);
            },
            Err(_e) => {
                // Retry authorization on db error
                info!("retrying authorization after error");
            },
        }
        // retry authorization if db err
        self.authorize_use_db(uri, user_uri, request_access, is_check_for_reload, trace)
    }
}

// Generic implementation of authorize method that can be used by both LMDB and MDBX contexts
pub(crate) fn authorize_with_stat<T: AuthorizationHelper>(
    ctx: &mut T,
    uri: &str,
    user_uri: &str,
    request_access: u8,
    is_check_for_reload: bool,
) -> Result<u8, std::io::Error> {
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

    let r = ctx.authorize_and_trace_impl(uri, user_uri, request_access, is_check_for_reload, &mut t);

    if let Some(stat) = ctx.get_stat_mut() {
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

// Macro to implement AuthorizationContext trait for types that implement AuthorizationHelper
#[macro_export]
macro_rules! impl_authorization_context {
    ($type:ty) => {
        impl v_authorization::common::AuthorizationContext for $type {
            fn authorize(
                &mut self,
                uri: &str,
                user_uri: &str,
                request_access: u8,
                is_check_for_reload: bool,
            ) -> Result<u8, std::io::Error> {
                $crate::common::authorize_with_stat(self, uri, user_uri, request_access, is_check_for_reload)
            }

            fn authorize_and_trace(
                &mut self,
                uri: &str,
                user_uri: &str,
                request_access: u8,
                is_check_for_reload: bool,
                trace: &mut v_authorization::common::Trace,
            ) -> Result<u8, std::io::Error> {
                self.authorize_and_trace_impl(uri, user_uri, request_access, is_check_for_reload, trace)
            }
        }
    };
}

