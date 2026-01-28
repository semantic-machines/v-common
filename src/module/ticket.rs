use v_individual_model::onto::datatype::Lang;
use v_individual_model::onto::individual::Individual;
use crate::v_api::common_type::ResultCode;
use chrono::{DateTime, NaiveDateTime, Utc};
use evmap::ShallowCopy;
use serde_json::Value;
use std::hash::{Hash, Hasher};
use std::mem::ManuallyDrop;
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct Ticket {
    pub id: String,
    /// Uri пользователя
    pub user_uri: String,
    /// login пользователя
    pub user_login: String,
    /// Код результата, если тикет не валидный != ResultCode.Ok
    pub result: ResultCode,
    /// Дата начала действия тикета
    pub start_time: i64,
    /// Дата окончания действия тикета
    pub end_time: i64,
    pub user_addr: String,
    /// Authentication method used (e.g., "sms", "password", "oauth")
    pub auth_method: String,
    /// Domain or service name (e.g., "Ideas", "Main")
    pub domain: String,
    /// Ticket creation initiator (e.g., "@https://optiflow.nn.local/ida/#/")
    pub initiator: String,
    /// Authentication origin type
    pub auth_origin: String,
}

impl Hash for Ticket {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Ticket {}

impl PartialEq for Ticket {
    fn eq(&self, other: &Self) -> bool {
        self.result == other.result
            && self.user_uri == other.user_uri
            && self.id == other.id
            && self.end_time == other.end_time
            && self.start_time == other.start_time
            && self.user_login == other.user_login
            && self.user_addr == other.user_addr
            && self.auth_method == other.auth_method
            && self.domain == other.domain
            && self.initiator == other.initiator
            && self.auth_origin == other.auth_origin
    }
}

impl ShallowCopy for Ticket {
    unsafe fn shallow_copy(&self) -> ManuallyDrop<Self> {
        ManuallyDrop::new(Ticket {
            id: self.id.clone(),
            user_uri: self.user_uri.clone(),
            user_login: self.user_login.clone(),
            result: self.result,
            start_time: self.start_time,
            end_time: self.end_time,
            user_addr: self.user_addr.clone(),
            auth_method: self.auth_method.clone(),
            domain: self.domain.clone(),
            initiator: self.initiator.clone(),
            auth_origin: self.auth_origin.clone(),
        })
    }
}

impl Default for Ticket {
    fn default() -> Self {
        Ticket {
            id: String::default(),
            user_uri: String::default(),
            user_login: String::default(),
            result: ResultCode::AuthenticationFailed,
            start_time: 0,
            end_time: 0,
            user_addr: "".to_string(),
            auth_method: "".to_string(),
            domain: "".to_string(),
            initiator: "".to_string(),
            auth_origin: "".to_string(),
        }
    }
}

impl From<serde_json::Value> for Ticket {
    fn from(val: Value) -> Self {
        let mut t = Ticket::default();
        if let Some(v) = val["id"].as_str() {
            t.id = v.to_owned();
        }
        if let Some(v) = val["user_uri"].as_str() {
            t.user_uri = v.to_owned();
        }
        if let Some(v) = val["user_login"].as_str() {
            t.user_login = v.to_owned();
        }
        if let Some(v) = val["result"].as_i64() {
            t.result = ResultCode::from_i64(v);
        }
        if let Some(v) = val["end_time"].as_i64() {
            t.end_time = v;
        }
        if let Some(v) = val["user_addr"].as_str() {
            t.user_addr = v.to_owned();
        }
        if let Some(v) = val["auth_method"].as_str() {
            t.auth_method = v.to_owned();
        }
        if let Some(v) = val["domain"].as_str() {
            t.domain = v.to_owned();
        }
        if let Some(v) = val["initiator"].as_str() {
            t.initiator = v.to_owned();
        }
        if let Some(v) = val["auth_origin"].as_str() {
            t.auth_origin = v.to_owned();
        }

        t
    }
}

impl Ticket {
    pub fn to_individual(&self) -> Individual {
        let mut ticket_indv = Individual::default();

        ticket_indv.add_string("rdf:type", "ticket:ticket", Lang::none());
        ticket_indv.set_id(&self.id);

        ticket_indv.add_string("ticket:login", &self.user_login, Lang::none());
        ticket_indv.add_string("ticket:accessor", &self.user_uri, Lang::none());
        ticket_indv.add_string("ticket:addr", &self.user_addr, Lang::none());

        let start_time_str = DateTime::<Utc>::from_timestamp(self.start_time, 0).unwrap_or_default().format("%Y-%m-%dT%H:%M:%S%.f").to_string();

        if start_time_str.len() > 28 {
            ticket_indv.add_string("ticket:when", &start_time_str[0..28], Lang::none());
        } else {
            ticket_indv.add_string("ticket:when", &start_time_str, Lang::none());
        }

        ticket_indv.add_string("ticket:duration", &(self.end_time - self.start_time).to_string(), Lang::none());
        
        // Add new fields
        ticket_indv.add_string("ticket:authMethod", &self.auth_method, Lang::none());
        ticket_indv.add_string("ticket:domain", &self.domain, Lang::none());
        ticket_indv.add_string("ticket:initiator", &self.initiator, Lang::none());
        ticket_indv.add_string("ticket:authOrigin", &self.auth_origin, Lang::none());
        
        ticket_indv
    }

    pub fn update_from_individual(&mut self, src: &mut Individual) {
        let when = src.get_first_literal("ticket:when");
        let duration = src.get_first_literal("ticket:duration").unwrap_or_default().parse::<i32>().unwrap_or_default();

        self.id = src.get_id().to_owned();
        self.user_uri = src.get_first_literal("ticket:accessor").unwrap_or_default();
        self.user_login = src.get_first_literal("ticket:login").unwrap_or_default();
        self.user_addr = src.get_first_literal("ticket:addr").unwrap_or_default();
        
        // Load new fields from RDF
        self.auth_method = src.get_first_literal("ticket:authMethod").unwrap_or_default();
        self.domain = src.get_first_literal("ticket:domain").unwrap_or_default();
        self.initiator = src.get_first_literal("ticket:initiator").unwrap_or_default();
        self.auth_origin = src.get_first_literal("ticket:authOrigin").unwrap_or_default();

        if self.user_uri.is_empty() {
            error!("found a session ticket is not complete, the user can not be found. ticket_id={}", self.id);
            self.user_uri = String::default();
            return;
        }

        if !self.user_uri.is_empty() && (when.is_none() || duration < 10) {
            error!("found a session ticket is not complete, we believe that the user has not been found.");
            self.user_uri = String::default();
            return;
        }
        let when = when.unwrap();

        if let Ok(t) = NaiveDateTime::parse_from_str(&when, "%Y-%m-%dT%H:%M:%S%.f") {
            self.start_time = t.and_utc().timestamp();
            self.end_time = self.start_time + duration as i64;
        } else {
            error!("fail parse field [ticket:when] = {}", when);
            self.user_uri = String::default();
        }
    }

    pub fn is_ticket_valid(&self, addr: &Option<IpAddr>, is_check_addr: bool) -> ResultCode {
        if is_check_addr {
            if let Some(a) = addr {
                if self.user_addr != a.to_string() {
                    error!("decline: ticket {}/{} request from {}", self.id, self.user_addr, a.to_string());
                    return ResultCode::TicketExpired;
                }
            } else {
                return ResultCode::TicketExpired;
            }
        }

        if self.result != ResultCode::Ok {
            return self.result;
        }

        if Utc::now().timestamp() > self.end_time {
            return ResultCode::TicketExpired;
        }

        if self.user_uri.is_empty() {
            return ResultCode::NotReady;
        }

        ResultCode::Ok
    }
}
