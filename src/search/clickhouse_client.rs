use crate::az_impl::az_lmdb::LmdbAzContext;
use crate::search::common::{FTQuery, QueryResult};
use crate::v_api::obj::{OptAuthorize, ResultCode};
use crate::v_authorization::common::AuthorizationContext;
use clickhouse_rs::errors::Error;
use clickhouse_rs::Pool;
use futures::executor::block_on;
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
}

async fn select_from_clickhouse(req: FTQuery, pool: &Pool, op_auth: OptAuthorize, out_res: &mut QueryResult, az: &mut LmdbAzContext) -> Result<(), Error> {
    let mut authorized_count = 0;
    let mut total_count = 0;

    if req
        .query
        .to_uppercase()
        .split([':', '-', ' ', '(', ')', '<', '<', '=', ','].as_ref())
        .any(|x| x == "INSERT" || x == "UPDATE" || x == "DROP" || x == "DELETE" || x == "ALTER" || x == "EXEC")
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
