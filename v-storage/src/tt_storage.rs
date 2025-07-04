use super::tt_wrapper::{Client, ClientConfig, IteratorType};
use v_individual_model::onto::individual::Individual;
use v_individual_model::onto::parser::parse_raw;
use crate::common::{Storage, StorageId, StorageResult};
use crate::RuntimeWrapper;
use std::str;

pub struct TTStorage {
    rt: RuntimeWrapper,
    client: Client,
}

const INDIVIDUALS_SPACE_ID: i32 = 512;
const TICKETS_SPACE_ID: i32 = 513;
const AZ_SPACE_ID: i32 = 514;

impl TTStorage {
    pub fn new(tt_uri: String, login: &str, pass: &str) -> TTStorage {
        TTStorage {
            rt: RuntimeWrapper::new(),
            client: ClientConfig::new(tt_uri, login, pass).set_timeout_time_ms(1000).set_reconnect_time_ms(10000).build(),
        }
    }
}

impl Storage for TTStorage {
    fn get_individual(&mut self, storage: StorageId, uri: &str, iraw: &mut Individual) -> StorageResult<()> {
        let space = if storage == StorageId::Tickets {
            TICKETS_SPACE_ID
        } else if storage == StorageId::Az {
            AZ_SPACE_ID
        } else {
            INDIVIDUALS_SPACE_ID
        };

        let key = (uri,);

        match self.rt.block_on(self.client.select(space, 0, &key, 0, 100, IteratorType::EQ)) {
            Ok(v) => {
                if !v.data.is_empty() {
                    iraw.set_raw(&v.data[5..]);
                    if parse_raw(iraw).is_ok() {
                        return StorageResult::Ok(());
                    } else {
                        return StorageResult::UnprocessableEntity;
                    }
                }
                StorageResult::NotFound
            },
            Err(_) => StorageResult::UnprocessableEntity,
        }
    }

    fn get_value(&mut self, storage: StorageId, key: &str) -> StorageResult<String> {
        let space = if storage == StorageId::Tickets {
            TICKETS_SPACE_ID
        } else if storage == StorageId::Az {
            AZ_SPACE_ID
        } else {
            INDIVIDUALS_SPACE_ID
        };

        let key_tuple = (key,);

        match self.rt.block_on(self.client.select(space, 0, &key_tuple, 0, 100, IteratorType::EQ)) {
            Ok(v) => {
                match std::str::from_utf8(&v.data[5..]) {
                    Ok(s) => StorageResult::Ok(s.to_string()),
                    Err(_) => StorageResult::Error("Invalid UTF-8 data".to_string()),
                }
            },
            Err(e) => {
                error!("TTStorage: fail get [{}] from tarantool, err={:?}", key, e);
                StorageResult::NotFound
            },
        }
    }

    fn get_raw_value(&mut self, storage: StorageId, key: &str) -> StorageResult<Vec<u8>> {
        let space = if storage == StorageId::Tickets {
            TICKETS_SPACE_ID
        } else if storage == StorageId::Az {
            AZ_SPACE_ID
        } else {
            INDIVIDUALS_SPACE_ID
        };

        let key_tuple = (key,);

        match self.rt.block_on(self.client.select(space, 0, &key_tuple, 0, 100, IteratorType::EQ)) {
            Ok(v) => StorageResult::Ok(v.data[5..].to_vec()),
            Err(e) => {
                error!("TTStorage: fail get raw [{}] from tarantool, err={:?}", key, e);
                StorageResult::NotFound
            },
        }
    }

    fn put_value(&mut self, storage: StorageId, key: &str, val: &str) -> StorageResult<()> {
        let space = if storage == StorageId::Tickets {
            TICKETS_SPACE_ID
        } else if storage == StorageId::Az {
            AZ_SPACE_ID
        } else {
            INDIVIDUALS_SPACE_ID
        };

        let tuple = (key, val);

        match self.rt.block_on(self.client.replace(space, &tuple)) {
            Ok(_) => StorageResult::Ok(()),
            Err(e) => {
                error!("tarantool: fail replace, db [{:?}], err = {:?}", storage, e);
                StorageResult::Error(format!("Failed to put value: {:?}", e))
            },
        }
    }

    fn put_raw_value(&mut self, storage: StorageId, _key: &str, val: Vec<u8>) -> StorageResult<()> {
        let space = if storage == StorageId::Tickets {
            TICKETS_SPACE_ID
        } else if storage == StorageId::Az {
            AZ_SPACE_ID
        } else {
            INDIVIDUALS_SPACE_ID
        };

        match self.rt.block_on(self.client.replace_raw(space, val)) {
            Ok(_) => StorageResult::Ok(()),
            Err(e) => {
                error!("tarantool: fail replace raw, db [{:?}], err = {:?}", storage, e);
                StorageResult::Error(format!("Failed to put raw value: {:?}", e))
            },
        }
    }

    fn remove_value(&mut self, storage: StorageId, key: &str) -> StorageResult<()> {
        let space = if storage == StorageId::Tickets {
            TICKETS_SPACE_ID
        } else if storage == StorageId::Az {
            AZ_SPACE_ID
        } else {
            INDIVIDUALS_SPACE_ID
        };

        let tuple = (key,);

        match self.rt.block_on(self.client.delete(space, &tuple)) {
            Ok(_) => StorageResult::Ok(()),
            Err(e) => {
                error!("tarantool: fail remove, db [{:?}], err = {:?}", storage, e);
                StorageResult::NotFound
            },
        }
    }

    fn count(&mut self, storage: StorageId) -> StorageResult<usize> {
        let space_name = if storage == StorageId::Tickets {
            "TICKETS"
        } else if storage == StorageId::Az {
            "AZ"
        } else {
            "INDIVIDUALS"
        };

        match self.rt.block_on(self.client.eval(format!("return box.space.{}:len()", space_name), &(0,))) {
            Ok(response) => {
                match response.decode::<(u64,)>() {
                    Ok(res) => StorageResult::Ok(res.0 as usize),
                    Err(e) => {
                        error!("failed to decode count response: db [{}], err = {:?}", space_name, e);
                        StorageResult::Error("Failed to decode count response".to_string())
                    },
                }
            },
            Err(e) => {
                error!("failed to count the number of records: db [{}], err = {:?}", space_name, e);
                StorageResult::Error(format!("Failed to count records: {:?}", e))
            },
        }
    }
}
