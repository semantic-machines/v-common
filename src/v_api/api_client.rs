use crate::onto::individual::Individual;
use crate::v_api::obj::ResultCode;
use nng::options::{Options, RecvTimeout, SendTimeout};
use nng::{Message, Protocol, Socket};
use serde_json::json;
use serde_json::Value;
use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

pub const ALL_MODULES: i64 = 0;

#[derive(Debug)]
pub struct ApiError {
    pub result: ResultCode,
    info: String,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "There is an error: {} {:?}", self.info, self.result)
    }
}

//impl Error for ApiError {}

impl ApiError {
    fn new(result: ResultCode, info: &str) -> Self {
        ApiError {
            result,
            info: info.to_owned(),
        }
    }
}

impl Default for ApiError {
    fn default() -> Self {
        ApiError {
            result: ResultCode::Zero,
            info: Default::default(),
        }
    }
}
#[derive(PartialEq, Debug, Clone)]
#[repr(u16)]
pub enum IndvOp {
    /// Сохранить
    Put = 1,

    /// Установить в
    SetIn = 45,

    /// Добавить в
    AddTo = 47,

    /// Убрать из
    RemoveFrom = 48,

    /// Убрать
    Remove = 51,

    None = 52,
}

impl IndvOp {
    pub fn from_i64(value: i64) -> IndvOp {
        match value {
            1 => IndvOp::Put,
            51 => IndvOp::Remove,
            47 => IndvOp::AddTo,
            45 => IndvOp::SetIn,
            48 => IndvOp::RemoveFrom,
            // ...
            _ => IndvOp::None,
        }
    }

    pub fn to_i64(&self) -> i64 {
        match self {
            IndvOp::Put => 1,
            IndvOp::Remove => 51,
            IndvOp::AddTo => 47,
            IndvOp::SetIn => 45,
            IndvOp::RemoveFrom => 48,
            // ...
            IndvOp::None => 52,
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            IndvOp::Put => "put",
            IndvOp::Remove => "remove",
            IndvOp::AddTo => "add_to",
            IndvOp::SetIn => "set_in",
            IndvOp::RemoveFrom => "remove_from",
            // ...
            IndvOp::None => "none",
        }
        .to_string()
    }
}

#[derive(Debug)]
pub struct OpResult {
    pub result: ResultCode,
    pub op_id: i64,
}

impl OpResult {
    pub fn res(r: ResultCode) -> Self {
        OpResult {
            result: r,
            op_id: -1,
        }
    }
}

pub struct NngClient {
    soc: Socket,
    addr: String,
    is_ready: bool,
}

impl NngClient {
    pub fn new(addr: String) -> NngClient {
        NngClient {
            soc: Socket::new(Protocol::Req0).unwrap(),
            addr,
            is_ready: false,
        }
    }

    pub fn connect(&mut self) -> bool {
        if self.addr.is_empty() {
            error!("mstorage-client:invalid addr: [{}]", self.addr);
            return self.is_ready;
        }

        if let Err(e) = self.soc.dial(self.addr.as_str()) {
            error!("mstorage-client:fail dial to main module, [{}], err={}", self.addr, e);
        } else {
            info!("success connect to main module, [{}]", self.addr);
            self.is_ready = true;

            if let Err(e) = self.soc.set_opt::<RecvTimeout>(Some(Duration::from_secs(30))) {
                error!("fail set recv timeout, err={}", e);
            }
            if let Err(e) = self.soc.set_opt::<SendTimeout>(Some(Duration::from_secs(30))) {
                error!("fail set send timeout, err={}", e);
            }
        }
        self.is_ready
    }

    pub(crate) fn req_recv(&mut self, query: Value) -> Result<Value, ApiError> {
        if !self.is_ready {
            self.connect();
        }
        if !self.is_ready {
            return Err(ApiError::new(ResultCode::NotReady, "fail connect"));
        }

        debug!("SEND {}", query.to_string());
        let req = Message::from(query.to_string().as_bytes());

        if let Err(e) = self.soc.send(req) {
            return Err(ApiError::new(ResultCode::NotReady, &format!("fail send to main module, err={:?}", e)));
        }

        // Wait for the response from the server.
        let wmsg = self.soc.recv();

        if let Err(e) = wmsg {
            return Err(ApiError::new(ResultCode::NotReady, &format!("fail recv from main module, err={:?}", e)));
        }

        let msg = wmsg.unwrap();

        debug!("recv msg = {}", &String::from_utf8_lossy(&msg));

        let reply = serde_json::from_str(&String::from_utf8_lossy(&msg));

        if let Err(e) = reply {
            return Err(ApiError::new(ResultCode::BadRequest, &format!("fail parse result operation [put], err={:?}", e)));
        }
        Ok(reply.unwrap())
    }
}

pub struct AuthClient {
    client: NngClient,
}

impl AuthClient {
    pub fn new(addr: String) -> AuthClient {
        AuthClient {
            client: NngClient::new(addr),
        }
    }

    pub fn connect(&mut self) -> bool {
        self.client.connect()
    }

    fn req_recv(&mut self, query: Value) -> Result<Value, ApiError> {
        match self.client.req_recv(query) {
            Ok(v) => {
                if let Some(r) = v["result"].as_i64() {
                    let res = ResultCode::from_i64(r);
                    if res != ResultCode::Ok {
                        return Err(ApiError::new(res, "api:update - invalid \"data\" section"));
                    }
                    Ok(v)
                } else {
                    Err(ApiError::new(ResultCode::BadRequest, "api:update - invalid \"data\" section"))
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn authenticate(&mut self, login: &str, password: &str, addr: Option<IpAddr>, secret: &Option<String>) -> Result<Value, ApiError> {
        let query = json!({
            "function": "authenticate",
            "login": login,
            "password": password,
            "addr" : addr.unwrap().to_string(),
            "secret" : secret
        });
        self.req_recv(query)
    }

    pub fn get_ticket_trusted(&mut self, ticket: &str, login: Option<&String>, addr: Option<IpAddr>) -> Result<Value, ApiError> {
        let query = json!({
            "function": "get_ticket_trusted",
            "login": login,
            "addr" : addr.unwrap().to_string(),
            "ticket": ticket,
        });
        self.req_recv(query)
    }
}

pub struct MStorageClient {
    client: NngClient,
    pub check_ticket_ip: bool,
}

impl MStorageClient {
    pub fn new(addr: String) -> MStorageClient {
        MStorageClient {
            client: NngClient::new(addr),
            check_ticket_ip: true,
        }
    }

    pub fn connect(&mut self) -> bool {
        self.client.connect()
    }

    pub fn update(&mut self, ticket: &str, cmd: IndvOp, indv: &Individual, addr: Option<IpAddr>) -> OpResult {
        match self.update_use_param(ticket, "", "", ALL_MODULES, cmd, indv, addr) {
            Ok(r) => r,
            Err(e) => OpResult::res(e.result),
        }
    }

    pub fn update_or_err(&mut self, ticket: &str, event_id: &str, src: &str, cmd: IndvOp, indv: &Individual, addr: Option<IpAddr>) -> Result<OpResult, ApiError> {
        self.update_use_param(ticket, event_id, src, ALL_MODULES, cmd, indv, addr)
    }

    pub fn update_use_param(
        &mut self,
        ticket: &str,
        event_id: &str,
        src: &str,
        assigned_subsystems: i64,
        cmd: IndvOp,
        indv: &Individual,
        addr: Option<IpAddr>,
    ) -> Result<OpResult, ApiError> {
        let query = json!({
            "function": cmd.as_string(),
            "ticket": ticket,
            "individuals": [indv.get_obj().as_json()],
            "assigned_subsystems": assigned_subsystems,
            "event_id" : event_id,
            "src" : src,
            "addr" : addr
        });

        self.update_form_json(query)
    }

    pub fn updates_use_param(
        &mut self,
        ticket: &str,
        event_id: &str,
        src: &str,
        assigned_subsystems: i64,
        cmd: IndvOp,
        indvs: &[Individual],
        addr: Option<IpAddr>,
    ) -> Result<OpResult, ApiError> {
        let mut jindvs = vec![];
        for indv in indvs {
            jindvs.push(indv.get_obj().as_json());
        }
        let query = json!({
            "function": cmd.as_string(),
            "ticket": ticket,
            "individuals": jindvs,
            "assigned_subsystems": assigned_subsystems,
            "event_id" : event_id,
            "src" : src,
            "addr": addr
        });
        self.update_form_json(query)
    }

    pub fn update_form_json(&mut self, query: Value) -> Result<OpResult, ApiError> {
        let json: Value = self.client.req_recv(query)?;

        if let Some(t) = json["type"].as_str() {
            if t != "OpResult" {
                return Err(ApiError::new(ResultCode::BadRequest, &format!("api:update - expecten \"type\" = \"OpResult\", found {}", t)));
            }
        } else {
            return Err(ApiError::new(ResultCode::BadRequest, "api:update - not found \"type\""));
        }

        if let Some(arr) = json["data"].as_array() {
            if arr.len() != 1 {
                return Err(ApiError::new(ResultCode::BadRequest, "api:update - invalid \"data\" section"));
            }

            if let Some(res) = arr[0]["result"].as_i64() {
                if let Some(op_id) = arr[0]["op_id"].as_i64() {
                    return Ok(OpResult {
                        result: ResultCode::from_i64(res),
                        op_id,
                    });
                }
            } else {
                return Err(ApiError::new(ResultCode::BadRequest, "api:update - invalid \"data\" section"));
            }
        } else {
            return if let Some(res) = json["result"].as_i64() {
                Ok(OpResult {
                    result: ResultCode::from_i64(res),
                    op_id: 0,
                })
            } else {
                error!("api:update - not found \"data\"");
                return Err(ApiError::new(ResultCode::BadRequest, "api:update - not found \"data\""));
            };
        }

        Err(ApiError::new(ResultCode::BadRequest, "api:update - unknown"))
    }
}
