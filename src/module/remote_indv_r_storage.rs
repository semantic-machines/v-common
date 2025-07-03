use crate::module::common::get_queue_status;
use crate::module::veda_backend::get_storage_use_prop;
use v_individual_model::onto::individual::{Individual, RawObj};
use v_individual_model::onto::individual2msgpack::to_msgpack;
use crate::storage::{StorageId, StorageMode, VStorage, StorageROClient};
use nng::{Message, Protocol, Socket};
use std::cell::RefCell;
use std::str;
use std::sync::Mutex;
use uuid::Uuid;
use lazy_static::lazy_static;
use log::error;

lazy_static! {
    pub static ref STORAGE: Mutex<RefCell<StorageROClient>> = Mutex::new(RefCell::new(StorageROClient::default()));
}

// inproc storage server

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
                return Message::from("[]".as_bytes());
            }

            return Message::from(binobj.as_slice());
        }

        let binobj = storage.get_raw_value(StorageId::Individuals, id);
        if binobj.is_empty() {
            return Message::from("[]".as_bytes());
        }
        return Message::from(binobj.as_slice());
    }

    Message::default()
}

pub fn get_individual(id: &str) -> Option<Individual> {
    if id.starts_with("srv:queue-state-") {
        return Some(get_queue_status(id));
    }

    let req = Message::from(id.to_string().as_bytes());

    let mut sh_client = STORAGE.lock().unwrap();
    let client = sh_client.get_mut();

    if !client.is_ready {
        client.connect();
    }

    if !client.is_ready {
        return None;
    }

    if let Err(e) = client.soc.send(req) {
        error!("fail send to storage_manager, err={:?}", e);
        return None;
    }

    // Wait for the response from the server.
    let wmsg = client.soc.recv();
    if let Err(e) = wmsg {
        error!("fail recv from main module, err={:?}", e);
        return None;
    }

    drop(sh_client);

    if let Ok(msg) = wmsg {
        let data = msg.as_slice();
        if data == b"[]" {
            return None;
        }
        return Some(Individual::new_raw(RawObj::new(data.to_vec())));
    }

    None
}
