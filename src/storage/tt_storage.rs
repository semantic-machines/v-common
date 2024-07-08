use crate::onto::individual::Individual;
use crate::onto::parser::parse_raw;
use crate::storage::common::{Storage, StorageId};
use crate::v_api::obj::ResultCode;
use rusty_tarantool::tarantool::{Client, ClientConfig, IteratorType};
use std::str;
use tokio::runtime::Runtime;

pub struct TTStorage {
    rt: Runtime,
    client: Client,
}

const INDIVIDUALS_SPACE_ID: i32 = 512;
const TICKETS_SPACE_ID: i32 = 513;
const AZ_SPACE_ID: i32 = 514;

impl TTStorage {
    pub fn new(tt_uri: String, login: &str, pass: &str) -> TTStorage {
        TTStorage {
            rt: Runtime::new().unwrap(),
            client: ClientConfig::new(tt_uri, login, pass).set_timeout_time_ms(1000).set_reconnect_time_ms(10000).build(),
        }
    }
    fn get_space_id(&self, storage: &StorageId) -> i32 {
        match storage {
            StorageId::Tickets => TICKETS_SPACE_ID,
            StorageId::Az => AZ_SPACE_ID,
            _ => INDIVIDUALS_SPACE_ID,
        }
    }
}

impl Storage for TTStorage {
    fn get_individual_from_db(&mut self, storage: StorageId, uri: &str, iraw: &mut Individual) -> Result<(), ResultCode> {
        let space = self.get_space_id(&storage);

        let key = (uri,);

        match self.rt.block_on(self.client.select(space, 0, &key, 0, 100, IteratorType::EQ)) {
            Ok(v) => {
                iraw.set_raw(&v.data[5..]);

                return if parse_raw(iraw).is_ok() {
                    if !iraw.get_id().is_empty() {
                        Ok(())
                    } else {
                        Err(ResultCode::NotFound)
                    }
                } else {
                    Err(ResultCode::UnprocessableEntity)
                };
            },
            Err(e) => {
                error!("TTStorage: fail get [{}] from tarantool, err={:?}", uri, e);
            },
        }

        Err(ResultCode::NotReady)
    }

    fn remove(&mut self, storage: StorageId, key: &str) -> Result<(), ResultCode> {
        let space = self.get_space_id(&storage);
        let tuple = (key,);

        if let Err(e) = self.rt.block_on(self.client.delete(space, &tuple)) {
            error!("tarantool: fail remove, db [{:?}], err = {:?}", storage, e);
            Err(ResultCode::FailStore)
        } else {
            Ok(())
        }
    }

    fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> Result<(), ResultCode> {
        let space = self.get_space_id(&storage);
        let tuple = (key, val);

        if let Err(e) = self.rt.block_on(self.client.replace(space, &tuple)) {
            error!("tarantool: fail replace, db [{:?}], err = {:?}", storage, e);
            Err(ResultCode::FailStore)
        } else {
            Ok(())
        }
    }

    fn put_kv_raw(&mut self, storage: StorageId, _key: &str, val: Vec<u8>) -> Result<(), ResultCode> {
        let space = self.get_space_id(&storage);

        if let Err(e) = self.rt.block_on(self.client.replace_raw(space, val)) {
            error!("tarantool: fail replace, db [{:?}], err = {:?}", storage, e);
            Err(ResultCode::FailStore)
        } else {
            Ok(())
        }
    }

    fn get_v(&mut self, storage: StorageId, key: &str) -> Result<Option<String>, ResultCode> {
        let space = self.get_space_id(&storage);
        let key = (key,);

        match self.rt.block_on(self.client.select(space, 0, &key, 0, 100, IteratorType::EQ)) {
            Ok(v) => {
                if v.data.len() > 5 {
                    match std::str::from_utf8(&v.data[5..]) {
                        Ok(s) => Ok(Some(s.to_string())),
                        Err(_) => Err(ResultCode::UnprocessableEntity),
                    }
                } else {
                    Ok(None)
                }
            },
            Err(e) => {
                error!("TTStorage: fail get [{}] from tarantool, err={:?}", key.0, e);
                Err(ResultCode::DatabaseQueryError)
            },
        }
    }

    fn get_raw(&mut self, storage: StorageId, key: &str) -> Result<Option<Vec<u8>>, ResultCode> {
        let space = self.get_space_id(&storage);
        let key = (key,);

        match self.rt.block_on(self.client.select(space, 0, &key, 0, 100, IteratorType::EQ)) {
            Ok(v) => {
                if v.data.len() > 5 {
                    Ok(Some(v.data[5..].to_vec()))
                } else {
                    Ok(None)
                }
            },
            Err(e) => {
                error!("TTStorage: fail get_raw [{}] from tarantool, err={:?}", key.0, e);
                Err(ResultCode::DatabaseQueryError)
            },
        }
    }

    fn count(&mut self, storage: StorageId) -> Result<usize, ResultCode> {
        let space_name = match storage {
            StorageId::Tickets => "TICKETS",
            StorageId::Az => "AZ",
            _ => "INDIVIDUALS",
        };

        let query = format!("return box.space.{}:len()", space_name);

        match self.rt.block_on(self.client.eval(query, &(0,))) {
            Ok(response) => match response.decode::<(u64,)>() {
                Ok(res) => Ok(res.0 as usize),
                Err(e) => {
                    error!("Failed to decode count result for db [{}]: {:?}", space_name, e);
                    Err(ResultCode::UnprocessableEntity)
                },
            },
            Err(e) => {
                error!("Failed to count the number of records in db [{}]: {:?}", space_name, e);
                Err(ResultCode::DatabaseQueryError)
            },
        }
    }
}
