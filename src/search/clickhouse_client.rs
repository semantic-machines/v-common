use crate::az_impl::az_lmdb::LmdbAzContext;
use crate::search::common::{is_identifier, AuthorizationLevel, FTQuery, QueryResult, ResultFormat};
use crate::v_api::obj::{OptAuthorize, ResultCode};
use crate::v_authorization::common::AuthorizationContext;
use chrono::prelude::*;
use chrono_tz::Tz;
use clickhouse_rs::errors::Error;
use clickhouse_rs::types::{Column, SqlType};
use clickhouse_rs::types::{FromSql, Row};
use clickhouse_rs::Pool;
use futures::executor::block_on;
use futures::lock::Mutex;
use serde_json::json;
use serde_json::Value;
use std::collections::HashSet;
use std::time::*;
use url::Url;
use v_authorization::common::Access;

pub struct CHClient {
    client: Option<Pool>,
    addr: String,
    is_ready: bool,
    az: LmdbAzContext,
}

impl CHClient {
    pub fn new(client_addr: String) -> CHClient {
        CHClient {
            client: None,
            addr: client_addr,
            is_ready: false,
            az: LmdbAzContext::new(1000),
        }
    }

    pub fn connect(&mut self) -> bool {
        info!("Configuration to connect to Clickhouse: {}", self.addr);
        match Url::parse(self.addr.as_ref()) {
            Ok(url) => {
                let host = url.host_str().unwrap_or("127.0.0.1");
                let port = url.port().unwrap_or(9000);
                let user = url.username();
                let pass = url.password().unwrap_or("123");
                let url = format!("tcp://{}:{}@{}:{}/", user, pass, host, port);
                info!("Trying to connect to Clickhouse, host: {}, port: {}, user: {}, password: {}", host, port, user, pass);
                info!("Connection url: {}", url);
                let pool = Pool::new(url);
                self.client = Some(pool);
                self.is_ready = true;
            },
            Err(e) => {
                error!("Invalid connection url, err={:?}", e);
                self.is_ready = false;
            },
        }
        self.is_ready
    }

    pub fn select(&mut self, req: FTQuery, op_auth: OptAuthorize) -> QueryResult {
        if !self.is_ready {
            self.connect();
        }

        let start = Instant::now();
        let mut res = QueryResult::default();

        if let Some(c) = &self.client {
            if let Err(e) = block_on(select_from_clickhouse(req, c, op_auth, &mut res, &mut self.az)) {
                error!("fail read from clickhouse: {:?}", e);
                res.result_code = ResultCode::InternalServerError
            }
        }

        res.total_time = start.elapsed().as_millis() as i64;
        res.query_time = res.total_time - res.authorize_time;
        debug!("result={:?}", res);

        res
    }

    pub async fn select_async(&mut self, req: FTQuery, op_auth: OptAuthorize) -> Result<QueryResult, Error> {
        let start = Instant::now();
        let mut res = QueryResult::default();

        if let Some(c) = &self.client {
            select_from_clickhouse(req, c, op_auth, &mut res, &mut self.az).await?;
        }
        res.total_time = start.elapsed().as_millis() as i64;
        res.query_time = res.total_time - res.authorize_time;
        debug!("result={:?}", res);

        Ok(res)
    }

    pub async fn query_select_async(
        &mut self,
        user_uri: &str,
        query: &str,
        res_format: ResultFormat,
        authorization_level: AuthorizationLevel,
        az: &Mutex<LmdbAzContext>,
    ) -> Result<Value, Error> {
        let mut jres = Value::default();
        if let Some(pool) = &self.client {
            let mut client = pool.get_handle().await?;
            let block = client.query(query).fetch_all().await?;

            let mut excluded_rows = HashSet::new();

            if res_format == ResultFormat::Cols {
                for col in block.columns() {
                    let mut jrow = Value::Array(vec![]);
                    let mut row_count = 0;
                    for row in block.rows() {
                        if !col_to_json(&row, col, &mut jrow, user_uri, &res_format, &authorization_level, az).await? {
                            if authorization_level == AuthorizationLevel::RowColumn {
                                excluded_rows.insert(row_count);
                            }
                        }
                        row_count += 1;
                    }
                    jres[col.name().to_owned()] = jrow;
                }
            } else {
                let mut v_cols = vec![];
                for col in block.columns() {
                    v_cols.push(Value::String(col.name().to_owned()));
                }
                jres["cols"] = Value::Array(v_cols);
                let mut jrows = vec![];
                for row in block.rows() {
                    let mut skip_row = false;
                    let mut jrow = if res_format == ResultFormat::Full {
                        Value::from(serde_json::Map::new())
                    } else {
                        Value::Array(vec![])
                    };
                    for col in block.columns() {
                        //println!("{} {}", col.name(), col.sql_type());
                        if !col_to_json(&row, col, &mut jrow, user_uri, &res_format, &authorization_level, az).await? {
                            skip_row = true;
                            break;
                        }
                    }
                    if !skip_row {
                        jrows.push(jrow);
                    }
                }

                jres["rows"] = Value::Array(jrows);
            }

            if res_format == ResultFormat::Cols && authorization_level == AuthorizationLevel::RowColumn {
                for (_col_name, col_values) in jres.as_object_mut().unwrap().iter_mut() {
                    if let Value::Array(ref mut rows) = col_values {
                        let mut i = 0;
                        rows.retain(|_| {
                            let retain = !excluded_rows.contains(&i);
                            i += 1;
                            retain
                        });
                    }
                }
            }
        }

        //println!("{}", res);
        Ok(jres)
    }
}

async fn cltjs<'a, K: clickhouse_rs::types::ColumnType, T: FromSql<'a> + serde::Serialize>(
    row: &'a Row<'_, K>,
    col: &'a Column<K>,
    jrow: &mut Value,
    user_uri: &str,
    res_format: &ResultFormat,
    authorization_level: &AuthorizationLevel,
    az: &Mutex<LmdbAzContext>,
) -> Result<bool, Error> {
    let v: T = row.get(col.name())?;
    let jv = json!(v);

    async fn check_authorization(
        jv: &Value,
        jrow: &mut Value,
        col_name: &str,
        user_uri: &str,
        res_format: &ResultFormat,
        authorization_level: &AuthorizationLevel,
        az: &Mutex<LmdbAzContext>,
    ) -> Result<bool, Error> {
        match jv {
            Value::String(vc) => {
                let authorized = process_authorization(vc, user_uri, authorization_level, az).await?;
                if authorized {
                    insert_value(jrow, col_name, jv.clone());
                } else {
                    match authorization_level {
                        AuthorizationLevel::Cell => insert_value(jrow, col_name, json!("d:NotAuthorized")),
                        _ => {
                            if res_format == &ResultFormat::Cols {
                                insert_value(jrow, col_name, json!("d:NotAuthorized"))
                            }
                            return Ok(false);
                        },
                    }
                }
                Ok(true)
            },
            Value::Array(array) => {
                let mut new_array = Vec::new();
                for item in array {
                    match item {
                        Value::String(vc) => {
                            let authorized = process_authorization(vc, user_uri, authorization_level, az).await?;
                            if authorized {
                                new_array.push(json!(vc));
                            } else {
                                match authorization_level {
                                    AuthorizationLevel::Cell => new_array.push(json!("v-s:NotAuthorized")),
                                    _ => {
                                        if res_format == &ResultFormat::Cols {
                                            new_array.push(json!("v-s:NotAuthorized"))
                                        }
                                        return Ok(false);
                                    },
                                }
                            }
                        },
                        _ => new_array.push(item.clone()), // Для не строковых элементов вставка без изменений
                    }
                }
                insert_value(jrow, col_name, Value::Array(new_array));
                Ok(true)
            },
            _ => {
                insert_value(jrow, col_name, jv.clone());
                Ok(true)
            },
        }
    }

    async fn process_authorization(vc: &str, user_uri: &str, authorization_level: &AuthorizationLevel, az: &Mutex<LmdbAzContext>) -> Result<bool, Error> {
        if (authorization_level == &AuthorizationLevel::Cell || authorization_level == &AuthorizationLevel::RowColumn) && is_identifier(vc) {
            let mut az_lock = az.lock().await;
            let authorized = az_lock.authorize(vc, user_uri, Access::CanRead as u8, false)?;
            Ok(authorized == Access::CanRead as u8)
        } else {
            // Если значение не является идентификатором, считаем, что авторизация не требуется
            Ok(true)
        }
    }

    fn insert_value(jrow: &mut Value, col_name: &str, value: Value) {
        if let Some(o) = jrow.as_object_mut() {
            o.insert(col_name.to_owned(), value);
        } else if let Some(o) = jrow.as_array_mut() {
            o.push(value);
        }
    }

    check_authorization(&jv, jrow, col.name(), user_uri, res_format, authorization_level, az).await
}

async fn col_to_json<K: clickhouse_rs::types::ColumnType>(
    row: &Row<'_, K>,
    col: &Column<K>,
    jrow: &mut Value,
    user_uri: &str,
    res_format: &ResultFormat,
    authorization_level: &AuthorizationLevel,
    az: &Mutex<LmdbAzContext>,
) -> Result<bool, Error> {
    let mut res = true;
    let sql_type = col.sql_type();
    match sql_type {
        SqlType::UInt8 => {
            res = cltjs::<K, u8>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::UInt16 => {
            res = cltjs::<K, u16>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::UInt32 => {
            res = cltjs::<K, u32>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::UInt64 => {
            res = cltjs::<K, u64>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::Int8 => {
            res = cltjs::<K, i8>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::Int16 => {
            res = cltjs::<K, i16>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::Int32 => {
            res = cltjs::<K, i32>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::Int64 => {
            res = cltjs::<K, i64>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::String => {
            res = cltjs::<K, String>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::FixedString(_) => {
            res = cltjs::<K, String>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::Float32 => {
            res = cltjs::<K, f32>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::Float64 => {
            res = cltjs::<K, f64>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
        },
        SqlType::Date => {
            let v: Date<Tz> = row.get(col.name())?;
            if let Some(o) = jrow.as_object_mut() {
                o.insert(col.name().to_owned(), json!(v.to_string()));
            } else if let Some(o) = jrow.as_array_mut() {
                o.push(json!(v.to_string()));
            }
        },
        SqlType::DateTime(_) => {
            let v: DateTime<Tz> = row.get(col.name())?;
            if let Some(o) = jrow.as_object_mut() {
                o.insert(col.name().to_owned(), json!(v.to_rfc3339_opts(SecondsFormat::Millis, false)));
            } else if let Some(o) = jrow.as_array_mut() {
                o.push(json!(v.to_rfc3339_opts(SecondsFormat::Millis, false)));
            }
        },
        SqlType::Decimal(_, _) => {
            let v: f64 = row.get(col.name())?;
            if let Some(o) = jrow.as_object_mut() {
                o.insert(col.name().to_owned(), json!(v));
            } else if let Some(o) = jrow.as_array_mut() {
                o.push(json!(v));
            }
        },
        SqlType::Array(stype) => match stype {
            SqlType::UInt8 => {
                res = cltjs::<K, Vec<u8>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::UInt16 => {
                res = cltjs::<K, Vec<u16>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::UInt32 => {
                res = cltjs::<K, Vec<u32>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::UInt64 => {
                res = cltjs::<K, Vec<u64>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::Int8 => {
                res = cltjs::<K, Vec<i8>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::Int16 => {
                res = cltjs::<K, Vec<i16>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::Int32 => {
                res = cltjs::<K, Vec<i32>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::Int64 => {
                res = cltjs::<K, Vec<i64>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::String => {
                res = cltjs::<K, Vec<String>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::FixedString(_) => {
                res = cltjs::<K, Vec<String>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::Float32 => {
                res = cltjs::<K, Vec<f32>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::Float64 => {
                res = cltjs::<K, Vec<f64>>(row, col, jrow, user_uri, res_format, authorization_level, az).await?;
            },
            SqlType::Date => {
                let v: Vec<Date<Tz>> = row.get(col.name())?;
                let mut a = vec![];
                for ev in v {
                    a.push(json!(ev.to_string()));
                }
                if let Some(o) = jrow.as_object_mut() {
                    o.insert(col.name().to_owned(), json!(a));
                } else if let Some(o) = jrow.as_array_mut() {
                    o.push(json!(a));
                }
            },
            SqlType::DateTime(_) => {
                let v: Vec<DateTime<Tz>> = row.get(col.name())?;
                let mut a = vec![];
                for ev in v {
                    a.push(json!(ev.to_rfc3339_opts(SecondsFormat::Millis, false)));
                }
                if let Some(o) = jrow.as_object_mut() {
                    o.insert(col.name().to_owned(), json!(a));
                } else if let Some(o) = jrow.as_array_mut() {
                    o.push(json!(a));
                }
            },
            SqlType::Decimal(_, _) => {
                let v: Vec<f64> = row.get(col.name())?;
                let mut a = vec![];
                for ev in v {
                    a.push(json!(ev));
                }
                if let Some(o) = jrow.as_object_mut() {
                    o.insert(col.name().to_owned(), json!(a));
                } else if let Some(o) = jrow.as_array_mut() {
                    o.push(json!(a));
                }
            },
            _ => {
                println!("unknown type {:?}", stype);
            },
        },
        _ => {
            println!("unknown type {:?}", col.sql_type());
        },
    }
    Ok(res)
}

async fn select_from_clickhouse(req: FTQuery, pool: &Pool, op_auth: OptAuthorize, out_res: &mut QueryResult, az: &mut LmdbAzContext) -> Result<(), Error> {
    let mut authorized_count = 0;
    let mut total_count = 0;

    if req
        .query
        .to_uppercase()
        .split([':', '-', ' ', '(', ')', '<', '<', '=', ','].as_ref())
        .any(|x| x.trim() == "INSERT" || x.trim() == "UPDATE" || x.trim() == "DROP" || x.trim() == "DELETE" || x.trim() == "ALTER" || x.trim() == "EXEC")
    {
        out_res.result_code = ResultCode::BadRequest;
        return Ok(());
    }

    let fq = if req.limit > 0 {
        format!("{} LIMIT {} OFFSET {}", req.query, req.limit, req.from)
    } else {
        format!("{} OFFSET {}", req.query, req.from)
    };

    debug!("query={}", fq);

    let mut client = pool.get_handle().await?;
    let block = client.query(fq).fetch_all().await?;
    for row in block.rows() {
        total_count += 1;

        let id: String = row.get(row.name(0)?)?;

        if op_auth == OptAuthorize::YES {
            let start = Instant::now();

            match az.authorize(&id, &req.user, Access::CanRead as u8, false) {
                Ok(res) => {
                    if res == Access::CanRead as u8 {
                        out_res.result.push(id);
                        authorized_count += 1;

                        if authorized_count >= req.top {
                            break;
                        }
                    }
                },
                Err(e) => error!("fail authorization {}, err={}", req.user, e),
            }
            out_res.authorize_time += start.elapsed().as_micros() as i64;
        } else {
            out_res.result.push(id);
        }

        if req.limit > 0 && total_count >= req.limit {
            break;
        }
    }

    out_res.result_code = ResultCode::Ok;
    out_res.estimated = (req.from + block.row_count() as i32) as i64;
    out_res.count = authorized_count as i64;
    out_res.processed = total_count as i64;
    out_res.cursor = (req.from + total_count) as i64;
    out_res.authorize_time /= 1000;

    Ok(())
}
