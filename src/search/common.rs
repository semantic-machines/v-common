use crate::onto::onto_index::OntoIndex;
use crate::storage::async_storage::get_individual_from_db;
use crate::storage::async_storage::AStorage;
use crate::v_api::obj::ResultCode;
use futures::lock::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryResult {
    pub result: Vec<String>,
    pub count: i64,
    pub estimated: i64,
    pub processed: i64,
    pub cursor: i64,
    pub total_time: i64,
    pub query_time: i64,
    pub authorize_time: i64,
    pub result_code: ResultCode,
}

impl Default for QueryResult {
    fn default() -> Self {
        QueryResult {
            result: vec![],
            count: 0,
            estimated: 0,
            processed: 0,
            cursor: 0,
            total_time: 0,
            query_time: 0,
            authorize_time: 0,
            result_code: ResultCode::NotReady,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FTQuery {
    pub ticket: String,
    pub user: String,
    pub query: String,
    pub sort: String,
    pub databases: String,
    pub reopen: bool,
    pub top: i32,
    pub limit: i32,
    pub from: i32,
}

impl FTQuery {
    pub fn new_with_user(user: &str, query: &str) -> FTQuery {
        FTQuery {
            ticket: "".to_owned(),
            user: user.to_owned(),
            query: query.to_owned(),
            sort: "".to_owned(),
            databases: "".to_owned(),
            reopen: false,
            top: 10000,
            limit: 10000,
            from: 0,
        }
    }

    pub fn new_with_ticket(ticket: &str, query: &str) -> FTQuery {
        FTQuery {
            ticket: ticket.to_owned(),
            user: "".to_owned(),
            query: query.to_owned(),
            sort: "".to_owned(),
            databases: "".to_owned(),
            reopen: false,
            top: 10000,
            limit: 10000,
            from: 0,
        }
    }

    pub fn as_string(&self) -> String {
        let mut s = String::new();

        s.push_str("[\"");
        if self.ticket.is_empty() {
            if !self.user.is_empty() {
                s.push_str("\"UU=");
                s.push_str(&self.user);
            }
        } else {
            s.push_str(&self.ticket);
        }

        s.push_str("\",\"");
        s.push_str(&self.query.replace('"', "\\\""));
        s.push_str("\",\"");
        s.push_str(&self.sort);
        s.push_str("\",\"");
        s.push_str(&self.databases);
        s.push_str("\",");
        s.push_str(&self.reopen.to_string());
        s.push(',');
        s.push_str(&self.top.to_string());
        s.push(',');
        s.push_str(&self.limit.to_string());
        s.push(',');
        s.push_str(&self.from.to_string());
        s.push(']');

        s
    }
}

////////////////////////////////////////////////////////////////////////

pub struct PrefixesCache {
    pub full2short_r: evmap::ReadHandle<String, String>,
    pub full2short_w: Arc<Mutex<evmap::WriteHandle<String, String>>>,
    pub short2full_r: evmap::ReadHandle<String, String>,
    pub short2full_w: Arc<Mutex<evmap::WriteHandle<String, String>>>,
}

pub fn split_full_prefix(v: &str) -> (&str, &str) {
    let pos = if let Some(n) = v.rfind('/') {
        n
    } else {
        v.rfind('#').unwrap_or_default()
    };

    v.split_at(pos + 1)
}

pub fn split_short_prefix(v: &str) -> Option<(&str, &str)> {
    if let Some(pos) = v.rfind(':') {
        let lr = v.split_at(pos);
        if let Some(l) = lr.1.strip_prefix(':') {
            return Some((lr.0, l));
        }
    }
    None
}

pub fn get_short_prefix(full_prefix: &str, prefixes_cache: &PrefixesCache) -> String {
    if let Some(v) = prefixes_cache.full2short_r.get(full_prefix) {
        if let Some(t) = v.get_one() {
            return t.to_string();
        }
    }

    full_prefix.to_owned()
}

pub fn get_full_prefix(short_prefix: &str, prefixes_cache: &PrefixesCache) -> String {
    if let Some(v) = prefixes_cache.short2full_r.get(short_prefix) {
        if let Some(t) = v.get_one() {
            return t.to_string();
        }
    }

    short_prefix.to_owned()
}

pub async fn load_prefixes(storage: &AStorage, prefixes_cache: &PrefixesCache) {
    let onto_index = OntoIndex::load();

    let mut f2s = prefixes_cache.full2short_w.lock().await;
    let mut s2f = prefixes_cache.short2full_w.lock().await;

    for id in onto_index.data.keys() {
        if let Ok((mut rindv, _res)) = get_individual_from_db(id, "", storage, None).await {
            rindv.parse_all();

            if rindv.any_exists("rdf:type", &["owl:Ontology"]) {
                if let Some(full_url) = rindv.get_first_literal("v-s:fullUrl") {
                    debug!("prefix : {} -> {}", rindv.get_id(), full_url);
                    let short_prefix = rindv.get_id().trim_end_matches(':');

                    f2s.insert(full_url.to_owned(), short_prefix.to_owned());
                    s2f.insert(short_prefix.to_owned(), full_url.to_owned());
                }
            }
        } else {
            error!("failed to read individual {}", id);
        }
        f2s.refresh();
        s2f.refresh();
    }
}
