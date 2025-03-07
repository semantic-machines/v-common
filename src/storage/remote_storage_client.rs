use v_individual_model::onto::individual::Individual;
use v_individual_model::onto::parser::parse_raw;
use crate::storage::common::StorageId;
use crate::v_api::obj::ResultCode;
use nng::{Message, Protocol, Socket};
use std::str;

// Remote client

pub struct StorageROClient {
    pub soc: Socket,
    pub addr: String,
    pub is_ready: bool,
}

impl Default for StorageROClient {
    fn default() -> Self {
        StorageROClient {
            soc: Socket::new(Protocol::Req0).unwrap(),
            addr: "".to_owned(),
            is_ready: false,
        }
    }
}

impl StorageROClient {
    pub fn new(addr: &str) -> Self {
        StorageROClient {
            soc: Socket::new(Protocol::Req0).unwrap(),
            addr: addr.to_string(),
            is_ready: false,
        }
    }

    pub fn connect(&mut self) -> bool {
        if let Err(e) = self.soc.dial(&self.addr) {
            error!("fail connect to storage_manager ({}), err={:?}", self.addr, e);
            self.is_ready = false;
        } else {
            info!("success connect connect to storage_manager ({})", self.addr);
            self.is_ready = true;
        }
        self.is_ready
    }

    pub fn get_individual_from_db(&mut self, db_id: StorageId, id: &str, iraw: &mut Individual) -> ResultCode {
        if !self.is_ready && !self.connect() {
            error!("REMOTE STORAGE: fail send to storage_manager, not ready");
            return ResultCode::NotReady;
        }

        let req = if db_id == StorageId::Tickets {
            Message::from(("t,".to_string() + id).as_bytes())
        } else {
            Message::from(("i,".to_string() + id).as_bytes())
        };

        if let Err(e) = self.soc.send(req) {
            error!("REMOTE STORAGE: fail send to storage_manager, err={:?}", e);
            return ResultCode::NotReady;
        }

        // Wait for the response from the server.
        match self.soc.recv() {
            Err(e) => {
                error!("REMOTE STORAGE: fail recv from main module, err={:?}", e);
                ResultCode::NotReady
            },

            Ok(msg) => {
                let data = msg.as_slice();
                if data == b"[]" {
                    return ResultCode::NotFound;
                }

                iraw.set_raw(data);

                if parse_raw(iraw).is_ok() {
                    ResultCode::Ok
                } else {
                    error!("REMOTE STORAGE: fail parse binobj, len={}, uri=[{}]", iraw.get_raw_len(), id);
                    ResultCode::NotReady
                }
            },
        }
    }

    pub fn count(&mut self, _storage: StorageId) -> usize {
        todo!()
    }
}
