use crate::onto::individual::Individual;
use crate::onto::resource::Value::{Bool, Datetime, Int, Num, Str, Uri};
use crate::search::sql_params::tr_statement;
use crate::v_api::obj::ResultCode;
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlparser::ast::Value;
use sqlparser::dialect::{AnsiDialect, ClickHouseDialect, MySqlDialect};
use sqlparser::parser::Parser;
use std::collections::HashMap;
use std::io;
use std::io::{Error, ErrorKind};

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
        s.push_str(&self.query);
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

fn indv_to_args_map(indv: &mut Individual) -> io::Result<HashMap<String, Value>> {
    let mut mm = HashMap::new();
    for (p, vals) in &indv.get_obj().resources {
        if p.starts_with("v-s:param") {
            let pb = format!("'{}'", p);
            if let Some(v) = vals.get(0) {
                match &v.value {
                    Uri(v) | Str(v, _) => {
                        mm.insert(pb, sqlparser::ast::Value::DoubleQuotedString(v.to_owned()));
                    },
                    Int(v) => {
                        mm.insert(pb, sqlparser::ast::Value::Number(v.to_string(), false));
                    },
                    Bool(v) => {
                        mm.insert(pb, sqlparser::ast::Value::Boolean(*v));
                    },
                    Num(_m, _d) => {
                        mm.insert(pb, sqlparser::ast::Value::Number(v.get_float().to_string(), false));
                    },
                    Datetime(v) => {
                        mm.insert(pb, sqlparser::ast::Value::SingleQuotedString(format!("{:?}", &Utc.timestamp(*v, 0))));
                    },
                    _ => {},
                }
            }
        }
    }

    Ok(mm)
}

pub fn prepare_sql_params(in_query: &str, params: &mut Individual, dialect: &str) -> Result<String, Error> {
    let mut query = in_query.to_owned();
    for p in &params.get_predicates() {
        if p.starts_with("v-s:param") {
            let pb = "{".to_owned() + p + "}";
            query = query.replace(&pb, &format!("'{}'", &p));
        }
    }

    let lex_tree = match dialect {
        "clickhouse" => Parser::parse_sql(&ClickHouseDialect {}, &query),
        "mysql" => Parser::parse_sql(&MySqlDialect {}, &query),
        _ => Parser::parse_sql(&AnsiDialect {}, &query),
    };

    match lex_tree {
        Ok(mut ast) => {
            for el in ast.iter_mut() {
                if let Ok(mm) = indv_to_args_map(params) {
                    //println!("PREV: {}", el);
                    tr_statement(el, &mm)?;
                    //println!("NEW: {}", el);
                    return Ok(el.to_string());
                }
            }
        },
        Err(e) => {
            error!("{:?}", e);
        },
    }
    return Err(Error::new(ErrorKind::Other, "?"));
}
