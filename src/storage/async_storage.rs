use crate::az_impl::az_lmdb::LmdbAzContext;
use crate::onto::individual::Individual;
use crate::onto::parser::parse_raw;
use crate::storage::common::{Storage, StorageId};
use crate::storage::lmdb_storage::LMDBStorage;
use crate::v_api::obj::ResultCode;
use crate::v_authorization::common::{Access, AuthorizationContext, Trace};
use futures::lock::Mutex;
use rusty_tarantool::tarantool::{Client, IteratorType};
use std::io;
use std::io::{Error, ErrorKind};

pub const INDIVIDUALS_SPACE_ID: i32 = 512;
pub const TICKETS_SPACE_ID: i32 = 513;

pub struct AStorage {
    pub tt: Option<Client>,
    pub lmdb: Option<Mutex<LMDBStorage>>,
}

pub async fn check_indv_access_read(mut indv: Individual, uri: &str, user_uri: &str, az: Option<&Mutex<LmdbAzContext>>) -> io::Result<(Individual, ResultCode)> {
    if let Some(a) = az {
        if a.lock().await.authorize(uri, user_uri, Access::CanRead as u8, false).unwrap_or(0) != Access::CanRead as u8 {
            return Ok((indv, ResultCode::NotAuthorized));
        }
    }

    if indv.get_id().is_empty() {
        return Ok((indv, ResultCode::NotFound));
    }
    indv.parse_all();
    Ok((indv, ResultCode::Ok))
}

pub async fn check_user_in_group(user_id: &str, group_id: &str, az: Option<&Mutex<LmdbAzContext>>) -> io::Result<bool> {
    if let Some(a) = az {
        let mut tr = Trace {
            acl: &mut "".to_string(),
            is_acl: false,
            group: &mut String::new(),
            is_group: true,
            info: &mut "".to_string(),
            is_info: false,
            str_num: 0,
        };
        if a.lock().await.authorize_and_trace(user_id, user_id, 0xF, false, &mut tr).is_ok() {
            for gr in tr.group.split('\n') {
                if gr == group_id {
                    return Ok(true);
                }
            }
        } else {
            return Err(Error::new(ErrorKind::Other, "fail authorize_and_trace"));
        }
    }

    Ok(false)
}

pub async fn get_individual_from_db(uri: &str, user_uri: &str, db: &AStorage, az: Option<&Mutex<LmdbAzContext>>) -> io::Result<(Individual, ResultCode)> {
    get_individual_use_space(INDIVIDUALS_SPACE_ID, uri, user_uri, db, az).await
}

pub async fn get_individual_use_space(space_id: i32, uri: &str, user_uri: &str, db: &AStorage, az: Option<&Mutex<LmdbAzContext>>) -> io::Result<(Individual, ResultCode)> {
    if let Some(tt) = &db.tt {
        let response = tt.select(space_id, 0, &(uri,), 0, 100, IteratorType::EQ).await?;

        let mut iraw = Individual::default();
        iraw.set_raw(&response.data[5..]);
        if parse_raw(&mut iraw).is_ok() {
            return check_indv_access_read(iraw, uri, user_uri, az).await;
        }
        return Ok((iraw, ResultCode::UnprocessableEntity));
    }
    if let Some(lmdb) = &db.lmdb {
        let mut iraw = Individual::default();
        if lmdb.lock().await.get_individual_from_db(StorageId::Individuals, uri, &mut iraw) {
            return check_indv_access_read(iraw, uri, user_uri, az).await;
        } else {
            return Ok((Individual::default(), ResultCode::NotFound));
        }
    }

    Ok((Individual::default(), ResultCode::UnprocessableEntity))
}
