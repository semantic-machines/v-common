use crate::module::common::get_queue_status;
use crate::module::veda_backend::get_storage_use_prop;
use crate::onto::individual::{Individual, RawObj};
use crate::onto::individual2msgpack::to_msgpack;
use crate::storage::common::{StorageId, StorageMode, VStorage};
use crate::storage::remote_storage_client::StorageROClient;
use crate::v_api::obj::ResultCode;
use nng::{Message, Protocol, Socket};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::str;
use std::sync::Mutex;
use uuid::Uuid;

lazy_static! {
    pub static ref STORAGE: Mutex<RefCell<StorageROClient>> = Mutex::new(RefCell::new(StorageROClient::default()));
}

#[derive(Serialize, Deserialize)]
struct ResponseData {
    code: i32,
    data: Vec<u8>,
}

impl From<i32> for ResultCode {
    fn from(code: i32) -> Self {
        match code {
            200 => ResultCode::Ok,
            404 => ResultCode::NotFound,
            400 => ResultCode::BadRequest,
            500 => ResultCode::InternalServerError,
            // Добавьте другие соответствия по мере необходимости
            _ => ResultCode::InternalServerError, // Значение по умолчанию для неизвестных кодов
        }
    }
}

pub fn inproc_storage_manager() -> std::io::Result<()> {
    let ro_storage_url = "inproc://nng/".to_owned() + &Uuid::new_v4().to_hyphenated().to_string();
    STORAGE.lock().unwrap().get_mut().addr = ro_storage_url.to_owned();

    let mut storage = get_storage_use_prop(StorageMode::ReadOnly);

    let server = Socket::new(Protocol::Rep0)?;
    if let Err(e) = server.listen(&ro_storage_url) {
        error!("fail listen, {:?}", e);
        return Ok(());
    }

    loop {
        if let Ok(recv_msg) = server.recv() {
            let res = req_prepare(&recv_msg, &mut storage);
            if let Err(e) = server.send(res) {
                error!("fail send {:?}", e);
            }
        }
    }
}

fn req_prepare(request: &Message, storage: &mut VStorage) -> Message {
    if let Ok(id) = str::from_utf8(request.as_slice()) {
        if id.starts_with("srv:queue-state-") {
            let indv = get_queue_status(id);

            let mut binobj: Vec<u8> = Vec::new();
            if let Err(e) = to_msgpack(&indv, &mut binobj) {
                error!("failed to serialize, err = {:?}", e);
                return create_response(ResultCode::InternalServerError, vec![]);
            }

            return create_response(ResultCode::Ok, binobj);
        }

        match storage.get_raw_value(StorageId::Individuals, id) {
            Ok(Some(binobj)) => create_response(ResultCode::Ok, binobj),
            Ok(None) => {
                warn!("Data not found for id: {}", id);
                create_response(ResultCode::NotFound, vec![])
            },
            Err(e) => {
                error!("Error getting raw value for id: {}, error: {:?}", id, e);
                create_response(e, vec![])
            },
        }
    } else {
        create_response(ResultCode::BadRequest, vec![])
    }
}

fn create_response(code: ResultCode, data: Vec<u8>) -> Message {
    let response = ResponseData {
        code: code as i32,
        data,
    };
    let serialized = bincode::serialize(&response).unwrap_or_default();
    Message::from(serialized.as_slice())
}

pub fn get_individual(id: &str) -> Result<Option<Individual>, ResultCode> {
    if id.starts_with("srv:queue-state-") {
        return Ok(Some(get_queue_status(id)));
    }

    let req = Message::from(id.to_string().as_bytes());

    let mut sh_client = STORAGE.lock().unwrap();
    let client = sh_client.get_mut();

    if !client.is_ready {
        client.connect();
    }

    if !client.is_ready {
        error!("client not ready");
        return Err(ResultCode::NotReady);
    }

    if let Err(e) = client.soc.send(req) {
        error!("fail send to storage_manager, err={:?}", e);
        return Err(ResultCode::ConnectError);
    }

    // Wait for the response from the server.
    let wmsg = client.soc.recv();
    if let Err(e) = wmsg {
        error!("fail recv from main module, err={:?}", e);
        return Err(ResultCode::ConnectError);
    }

    drop(sh_client);

    if let Ok(msg) = wmsg {
        let response: ResponseData = match bincode::deserialize(msg.as_slice()) {
            Ok(resp) => resp,
            Err(_) => return Err(ResultCode::InternalServerError),
        };

        match ResultCode::from(response.code) {
            ResultCode::Ok => Ok(Some(Individual::new_raw(RawObj::new(response.data)))),
            ResultCode::NotFound => Ok(None),
            error_code => Err(error_code),
        }
    } else {
        Err(ResultCode::InternalServerError)
    }
}
