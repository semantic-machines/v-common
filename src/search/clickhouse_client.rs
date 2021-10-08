use crate::az_impl::az_lmdb::LmdbAzContext;
use crate::search::common::QueryResult;
use crate::v_api::obj::{OptAuthorize, ResultCode};
use crate::v_authorization::common::AuthorizationContext;
use clickhouse_rs::errors::Error;
use clickhouse_rs::Pool;
use futures::executor::block_on;
use std::str;
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
            az: LmdbAzContext::new(),
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
            }
            Err(e) => {
                error!("Invalid connection url, err={:?}", e);
                self.is_ready = false;
            }
        }
        self.is_ready
    }

    pub fn select(&mut self, user_uri: &str, query: &str, top: i64, limit: i64, from: i64, op_auth: OptAuthorize) -> QueryResult {
        if !self.is_ready {
            self.connect();
        }

        let start = Instant::now();
        let mut res = QueryResult::default();

        if let Some(c) = &self.client {
            if let Err(e) = block_on(select_from_clickhouse(c, user_uri, query, top, limit, from, op_auth, &mut res, &mut self.az)) {
                error!("fail read from clickhouse: {:?}", e);
                res.result_code = ResultCode::InternalServerError
            }
        }

        res.total_time = start.elapsed().as_millis() as i64;
        res.query_time = res.total_time - res.authorize_time;
        debug!("result={:?}", res);

        res
    }

    pub async fn select_async(&mut self, user_uri: &str, query: &str, top: i64, limit: i64, from: i64, op_auth: OptAuthorize) -> Result<QueryResult, Error> {
        let start = Instant::now();
        let mut res = QueryResult::default();

        if let Some(c) = &self.client {
            select_from_clickhouse(c, user_uri, query, top, limit, from, op_auth, &mut res, &mut self.az).await?;
        }
        res.total_time = start.elapsed().as_millis() as i64;
        res.query_time = res.total_time - res.authorize_time;
        debug!("result={:?}", res);

        Ok(res)
    }
}

async fn select_from_clickhouse(
    pool: &Pool,
    user_uri: &str,
    query: &str,
    top: i64,
    limit: i64,
    from: i64,
    op_auth: OptAuthorize,
    out_res: &mut QueryResult,
    az: &mut LmdbAzContext,
) -> Result<(), Error> {
    let mut authorized_count = 0;
    let mut total_count = 0;

    if query.to_uppercase().split_whitespace().any(|x| x == "INSERT" || x == "UPDATE" || x == "DROP" || x == "DELETE") {
        out_res.result_code = ResultCode::BadRequest;
        return Ok(());
    }

    let fq = if limit > 0 {
        format!("{} LIMIT {} OFFSET {}", query, limit, from)
    } else {
        format!("{} OFFSET {}", query, from)
    };

    //info!("query={}", fq);

    let mut client = pool.get_handle().await?;
    let block = client.query(fq).fetch_all().await?;
    for row in block.rows() {
        total_count += 1;

        let id: String = row.get(row.name(0)?)?;

        if op_auth == OptAuthorize::YES {
            let start = Instant::now();

            match az.authorize(&id, user_uri, Access::CanRead as u8, false) {
                Ok(res) => {
                    if res == Access::CanRead as u8 {
                        out_res.result.push(id);
                        authorized_count += 1;

                        if authorized_count >= top {
                            break;
                        }
                    }
                }
                Err(e) => error!("fail authorization {}, err={}", user_uri, e),
            }
            out_res.authorize_time += start.elapsed().as_micros() as i64;
        } else {
            out_res.result.push(id);
        }

        if limit > 0 && total_count >= limit {
            break;
        }
    }

    out_res.result_code = ResultCode::Ok;
    out_res.estimated = from + (block.row_count() as i64);
    out_res.count = authorized_count;
    out_res.processed = total_count;
    out_res.cursor = from + total_count;
    out_res.authorize_time /= 1000;

    Ok(())
}
