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
#[derive(Eq, PartialEq, Debug, Clone)]
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

#[derive(Clone)]
pub struct NngClient {
    name: String,
    soc: Socket,
    addr: String,
    is_ready: bool,
}

impl NngClient {
    pub fn new(name: &str, addr: String) -> NngClient {
        NngClient {
            name: name.to_owned(),
            soc: Socket::new(Protocol::Req0).unwrap(),
            addr,
            is_ready: false,
        }
    }

    pub fn connect(&mut self) -> bool {
        if self.addr.is_empty() {
            error!("nng {} : invalid addr: [{}]", self.name, self.addr);
            return self.is_ready;
        }

        if let Err(e) = self.soc.dial(self.addr.as_str()) {
            error!("nng {}: fail dial to [{}], err={}", self.name, self.addr, e);
        } else {
            info!("nng {}: success connect to [{}]", self.name, self.addr);
            self.is_ready = true;

            if let Err(e) = self.soc.set_opt::<RecvTimeout>(Some(Duration::from_secs(30))) {
                error!("nng {}: fail set recv timeout, err={}", self.name, e);
            }
            if let Err(e) = self.soc.set_opt::<SendTimeout>(Some(Duration::from_secs(30))) {
                error!("nng {}: fail set send timeout, err={}", self.name, e);
            }
        }
        self.is_ready
    }

    pub(crate) fn req_recv(&mut self, query: Value) -> Result<Value, ApiError> {
        if !self.is_ready {
            self.connect();
        }
        if !self.is_ready {
            return Err(ApiError::new(ResultCode::NotReady, &format!("nng {}: fail connect", self.name)));
        }

        let req = Message::from(query.to_string().as_bytes());

        if let Err(e) = self.soc.send(req) {
            return Err(ApiError::new(ResultCode::NotReady, &format!("nng {}: fail send, err={:?}", self.name, e)));
        }

        // Wait for the response from the server.
        let wmsg = self.soc.recv();

        if let Err(e) = wmsg {
            return Err(ApiError::new(ResultCode::NotReady, &format!("nng {}: fail recv, err={:?}", self.name, e)));
        }

        let msg = wmsg.unwrap();

        debug!("nng-client: recv msg = {}", &String::from_utf8_lossy(&msg));

        let reply = serde_json::from_str(&String::from_utf8_lossy(&msg));

        if let Err(e) = reply {
            return Err(ApiError::new(ResultCode::BadRequest, &format!("nng {}: fail parse result operation [put], err={:?}", self.name, e)));
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
            client: NngClient::new("auth client", addr),
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
            },
            Err(e) => Err(e),
        }
    }

    pub fn authenticate(&mut self, login: &str, password: &Option<String>, addr: Option<IpAddr>, secret: &Option<String>) -> Result<Value, ApiError> {
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

    pub fn logout(&mut self, ticket: &Option<String>, addr: Option<IpAddr>) -> Result<Value, ApiError> {
        let query = json!({
            "function": "logout",
            "addr" : addr.unwrap().to_string(),
            "ticket": ticket
        });
        self.req_recv(query)
    }
}

#[derive(Clone)]
pub struct MStorageClient {
    client: NngClient,
    pub check_ticket_ip: bool,
}

#[derive(Default, Clone)]
pub struct UpdateOptions<'a> {
    pub event_id: Option<&'a str>,
    pub src: Option<&'a str>,
    pub assigned_subsystems: Option<i64>,
    pub n_count: Option<i64>,
    pub addr: Option<IpAddr>,
}

impl MStorageClient {
    pub fn new(addr: String) -> MStorageClient {
        MStorageClient {
            client: NngClient::new("mstorage client", addr),
            check_ticket_ip: true,
        }
    }

    pub fn connect(&mut self) -> bool {
        self.client.connect()
    }

    pub fn update(&mut self, indv: &Individual, ticket: &str, cmd: IndvOp, options: UpdateOptions) -> Result<OpResult, ApiError> {
        self.update_internal(indv, ticket, cmd, options)
    }

    pub fn updates(&mut self, indvs: &[Individual], ticket: &str, cmd: IndvOp, options: UpdateOptions) -> Result<OpResult, ApiError> {
        self.updates_internal(indvs, ticket, cmd, options)
    }

    fn update_internal(&mut self, indv: &Individual, ticket: &str, cmd: IndvOp, options: UpdateOptions) -> Result<OpResult, ApiError> {
        let mut query = json!({
            "function": cmd.as_string(),
            "ticket": ticket,
            "individuals": [indv.get_obj().as_json()],
        });

        self.add_options_to_query(&mut query, &options);
        self.update_form_json(query)
    }

    fn updates_internal(&mut self, indvs: &[Individual], ticket: &str, cmd: IndvOp, options: UpdateOptions) -> Result<OpResult, ApiError> {
        let mut query = json!({
            "function": cmd.as_string(),
            "ticket": ticket,
            "individuals": indvs.iter().map(|indv| indv.get_obj().as_json()).collect::<Vec<_>>(),
        });

        self.add_options_to_query(&mut query, &options);
        self.update_form_json(query)
    }

    fn add_options_to_query(&self, query: &mut serde_json::Value, options: &UpdateOptions) {
        if let Some(event_id) = options.event_id {
            query["event_id"] = json!(event_id);
        }

        if let Some(src) = options.src {
            query["src"] = json!(src);
        }

        if let Some(assigned_subsystems) = options.assigned_subsystems {
            query["assigned_subsystems"] = json!(assigned_subsystems);
        } else {
            query["assigned_subsystems"] = json!(ALL_MODULES);
        }

        if let Some(n_count) = options.n_count {
            query["n_count"] = json!(n_count);
        }

        if let Some(addr) = options.addr {
            query["addr"] = json!(addr.to_string());
        }
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
