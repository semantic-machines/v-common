use v_individual_model::onto::individual::Individual;
use v_individual_model::onto::parser::parse_raw;
use crate::common::{Storage, StorageId, StorageResult};
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

    pub fn get_individual_from_db(&mut self, db_id: StorageId, id: &str, iraw: &mut Individual) -> StorageResult<()> {
        if !self.is_ready && !self.connect() {
            error!("REMOTE STORAGE: fail send to storage_manager, not ready");
            return StorageResult::NotReady;
        }

        let req = if db_id == StorageId::Tickets {
            Message::from(("t,".to_string() + id).as_bytes())
        } else {
            Message::from(("i,".to_string() + id).as_bytes())
        };

        if let Err(e) = self.soc.send(req) {
            error!("REMOTE STORAGE: fail send to storage_manager, err={:?}", e);
            return StorageResult::NotReady;
        }

        // Wait for the response from the server.
        match self.soc.recv() {
            Err(e) => {
                error!("REMOTE STORAGE: fail recv from main module, err={:?}", e);
                StorageResult::NotReady
            },

            Ok(msg) => {
                let data = msg.as_slice();
                if data == b"[]" {
                    return StorageResult::NotFound;
                }

                iraw.set_raw(data);

                if parse_raw(iraw).is_ok() {
                    StorageResult::Ok(())
                } else {
                    error!("REMOTE STORAGE: fail parse binobj, len={}, uri=[{}]", iraw.get_raw_len(), id);
                    StorageResult::UnprocessableEntity
                }
            },
        }
    }

    pub fn count(&mut self, _storage: StorageId) -> StorageResult<usize> {
        StorageResult::Error("Remote storage does not support count".to_string())
    }
}

impl Storage for StorageROClient {
    fn get_individual(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> StorageResult<()> {
        self.get_individual_from_db(storage, id, iraw)
    }

    fn get_value(&mut self, _storage: StorageId, _key: &str) -> StorageResult<String> {
        // Remote storage пока не поддерживает get_value
        StorageResult::Error("Remote storage does not support get_value".to_string())
    }

    fn get_raw_value(&mut self, _storage: StorageId, _key: &str) -> StorageResult<Vec<u8>> {
        // Remote storage пока не поддерживает get_raw_value
        StorageResult::Error("Remote storage does not support get_raw_value".to_string())
    }

    fn put_value(&mut self, _storage: StorageId, _key: &str, _val: &str) -> StorageResult<()> {
        // Remote storage пока не поддерживает put_value (read-only client)
        StorageResult::Error("Remote storage is read-only".to_string())
    }

    fn put_raw_value(&mut self, _storage: StorageId, _key: &str, _val: Vec<u8>) -> StorageResult<()> {
        // Remote storage пока не поддерживает put_raw_value (read-only client)
        StorageResult::Error("Remote storage is read-only".to_string())
    }

    fn remove_value(&mut self, _storage: StorageId, _key: &str) -> StorageResult<()> {
        // Remote storage пока не поддерживает remove_value (read-only client)
        StorageResult::Error("Remote storage is read-only".to_string())
    }

    fn count(&mut self, _storage: StorageId) -> StorageResult<usize> {
        // Remote storage пока не поддерживает count
        StorageResult::Error("Remote storage does not support count".to_string())
    }
}
