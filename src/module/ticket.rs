use crate::onto::individual::Individual;
use crate::v_api::obj::ResultCode;
use chrono::{NaiveDateTime, Utc};
use evmap::ShallowCopy;
use serde_json::Value;
use std::hash::{Hash, Hasher};
use std::mem::ManuallyDrop;

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
            && self.user_login == other.user_uri
    }

    fn ne(&self, other: &Self) -> bool {
        self.result != other.result
            || self.user_uri != other.user_uri
            || self.id != other.id
            || self.end_time != other.end_time
            || self.start_time != other.start_time
            || self.user_login != other.user_uri
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

        t
    }
}

impl Ticket {
    pub fn update_from_individual(&mut self, src: &mut Individual) {
        let when = src.get_first_literal("ticket:when");
        let duration = src.get_first_literal("ticket:duration").unwrap_or_default().parse::<i32>().unwrap_or_default();

        self.id = src.get_id().to_owned();
        self.user_uri = src.get_first_literal("ticket:accessor").unwrap_or_default();
        self.user_login = src.get_first_literal("ticket:login").unwrap_or_default();

        if self.user_uri.is_empty() {
            error!("found a session ticket is not complete, the user can not be found.");
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
            self.start_time = t.timestamp();
            self.end_time = self.start_time + duration as i64;
        } else {
            error!("fail parse field [ticket:when] = {}", when);
            self.user_uri = String::default();
        }
    }

    pub fn is_ticket_valid(&self) -> ResultCode {
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
