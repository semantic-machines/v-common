#[cfg(feature = "az_tt_2")]
use rusty_tarantool_2::tarantool::{Client, ClientConfig};
#[cfg(feature = "az_tt_3")]
use rusty_tarantool_3::tarantool::{Client, ClientConfig};

use std::io::{self, Error, ErrorKind};
use std::time::SystemTime;
use v_authorization::common::{AuthorizationContext, Trace};

use crate::runtime_wrapper::RuntimeWrapper;
use crate::stat_manager::StatPub;

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

pub struct TarantoolAzContext {
    rt: RuntimeWrapper,
    client: Client,
    stat: Option<Stat>,
}

impl TarantoolAzContext {
    pub fn new(uri: &str, login: &str, password: &str) -> Self {
        let client = ClientConfig::new(uri, login, password)
            .set_timeout_time_ms(5000)
            .set_reconnect_time_ms(3000)
            .build();
        
        TarantoolAzContext {
            rt: RuntimeWrapper::new(),
            client,
            stat: None,
        }
    }
    
    pub fn new_with_stat(
        uri: &str, 
        login: &str, 
        password: &str,
        stat_url: Option<String>,
        stat_mode_str: Option<String>,
    ) -> Self {
        let client = ClientConfig::new(uri, login, password)
            .set_timeout_time_ms(5000)
            .set_reconnect_time_ms(3000)
            .build();
        
        let mode = if let Some(v) = stat_mode_str {
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
        
        let stat = stat_url.and_then(|url| StatPub::new(&url).ok()).map(|p| Stat {
            point: p,
            mode,
        });
        
        TarantoolAzContext {
            rt: RuntimeWrapper::new(),
            client,
            stat,
        }
    }
}

impl AuthorizationContext for TarantoolAzContext {
    fn authorize(
        &mut self,
        uri: &str,
        user_uri: &str,
        request_access: u8,
        _is_check_for_reload: bool,
    ) -> Result<u8, io::Error> {
        let start = SystemTime::now();
        
        let result = self.rt.block_on(
            self.client.call_fn(
                "v_az_tarantool.authorize_check",
                &(uri, user_uri, request_access),
            )
        );
        
        // Collect stats
        if let Some(stat) = &mut self.stat {
            if stat.mode == StatMode::Full || stat.mode == StatMode::Minimal {
                let elapsed = start.elapsed().unwrap_or_default();
                stat.point.set_duration(elapsed);
                if let Err(e) = stat.point.flush() {
                    warn!("fail flush stat, err={:?}", e);
                }
            }
        }
        
        match result {
            Ok(response) => {
                match response.decode::<(u8,)>() {
                    Ok((rights,)) => Ok(rights),
                    Err(e) => Err(Error::new(ErrorKind::InvalidData, e.to_string())),
                }
            }
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        }
    }

    fn authorize_and_trace(
        &mut self,
        uri: &str,
        user_uri: &str,
        request_access: u8,
        _is_check_for_reload: bool,
        trace: &mut Trace,
    ) -> Result<u8, io::Error> {
        if !trace.is_info {
            return self.authorize(uri, user_uri, request_access, _is_check_for_reload);
        }
        
        let start = SystemTime::now();
        
        let result = self.rt.block_on(
            self.client.call_fn(
                "v_az_tarantool.authorize_trace",
                &(uri, user_uri, request_access),
            )
        );
        
        // Collect stats
        if let Some(stat) = &mut self.stat {
            if stat.mode == StatMode::Full || stat.mode == StatMode::Minimal {
                let elapsed = start.elapsed().unwrap_or_default();
                stat.point.set_duration(elapsed);
                if let Err(e) = stat.point.flush() {
                    warn!("fail flush stat, err={:?}", e);
                }
            }
        }
        
        match result {
            Ok(response) => {
                match response.decode::<(u8, String)>() {
                    Ok((rights, info)) => {
                        trace.info.push_str(&info);
                        Ok(rights)
                    }
                    Err(e) => Err(Error::new(ErrorKind::InvalidData, e.to_string())),
                }
            }
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        }
    }
}
