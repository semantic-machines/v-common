use crate::az_impl::az_lmdb::LmdbAzContext;
use crate::module::module_impl::Module;
use crate::onto::*;
use crate::search::common::{get_short_prefix, split_full_prefix, AuthorizationLevel, PrefixesCache, QueryResult, ResultFormat};
use crate::v_api::obj::ResultCode;
use awc::Client;
use futures::lock::Mutex;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::{json, Value};
use std::io::{Error, ErrorKind};
use std::time::Instant;
use stopwatch::Stopwatch;
use v_authorization::common::{Access, AuthorizationContext};

#[derive(Serialize, Deserialize)]
pub(crate) struct Head {
    pub vars: Vec<String>,
}
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Bindings {
    pub bindings: Vec<Value>,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct SparqlResponse {
    pub head: Head,
    pub results: Bindings,
}
pub struct SparqlClient {
    pub(crate) point: String,
    pub(crate) client: Client,
    pub(crate) az: LmdbAzContext,
}

impl Default for SparqlClient {
    fn default() -> Self {
        SparqlClient {
            point: format!("{}/{}?{}", Module::get_property("sparql_db").unwrap_or_default(), "query", "default"),
            client: Client::default(),
            az: LmdbAzContext::new(1000),
        }
    }
}

impl SparqlClient {
    pub async fn query_select_ids(&mut self, user_uri: &str, query: String, prefix_cache: &PrefixesCache) -> QueryResult {
        let total_time = Instant::now();

        let res_req =
            self.client.post(&self.point).header("Content-Type", "application/sparql-query").header("Accept", "application/sparql-results+json").send_body(query).await;

        let mut qres = QueryResult::default();

        if let Ok(mut response) = res_req {
            match response.json::<SparqlResponse>().await {
                Ok(v) => {
                    if v.head.vars.len() > 1 {
                        qres.result_code = ResultCode::BadRequest;
                        return qres;
                    }
                    let var = &v.head.vars[0];
                    debug!("vars:{var:?}");

                    qres.count = v.results.bindings.len() as i64;

                    qres.result_code = ResultCode::Ok;

                    let mut auth_sw = Stopwatch::new();
                    for el in v.results.bindings {
                        let r = &el[var];
                        if r["type"] == "uri" {
                            if let Some(v) = r["value"].as_str() {
                                let iri = split_full_prefix(v);

                                let fullprefix = iri.0;
                                let prefix = get_short_prefix(fullprefix, &prefix_cache);
                                let short_iri = format!("{prefix}:{}", iri.1);

                                auth_sw.start();
                                if self.az.authorize(&short_iri, user_uri, Access::CanRead as u8, true).unwrap_or(0) == Access::CanRead as u8 {
                                    qres.result.push(short_iri);
                                }
                                auth_sw.stop();
                            }
                        }
                    }
                    qres.processed = qres.result.len() as i64;
                    qres.authorize_time = auth_sw.elapsed_ms();
                },
                Err(e) => {
                    error!("{:?}", e);
                },
            }
        }

        qres.total_time = total_time.elapsed().as_millis() as i64;
        qres.query_time = qres.total_time - qres.authorize_time;

        qres
    }

    pub async fn query_select(
        &mut self,
        user_uri: &str,
        query: String,
        format: ResultFormat,
        authorization_level: AuthorizationLevel,
        az: &Mutex<LmdbAzContext>,
        prefix_cache: &PrefixesCache,
    ) -> Result<Value, Error> {
        let res_req =
            self.client.post(&self.point).header("Content-Type", "application/sparql-query").header("Accept", "application/sparql-results+json").send_body(query).await;

        let mut jres = Value::default();

        match res_req {
            Ok(mut response) => match response.json::<SparqlResponse>().await {
                Ok(v) => {
                    // Обработка заголовков
                    let v_cols: Vec<Value> = v.head.vars.iter().map(|el| json!(el)).collect();
                    // Подготовка данных для разных форматов
                    let mut jrows = Vec::new();
                    let mut col_data: Map<String, Value> = Map::new();

                    for el in v.results.bindings {
                        let mut skip_row = false;
                        let mut jrow = Map::new();
                        let mut row_vec = Vec::new();

                        for var in &v.head.vars {
                            let r = &el[var];
                            debug!("var={}", var);
                            let processed_value = if let (Some(r_type), Some(r_value), r_datatype) = (r.get("type"), r.get("value"), r.get("datatype")) {
                                match (r_type.as_str(), r_value.as_str()) {
                                    (Some("uri"), Some(data)) => {
                                        let iri = split_full_prefix(data);
                                        let prefix = get_short_prefix(iri.0, &prefix_cache);
                                        let short_iri = format!("{prefix}:{}", iri.1);
                                        if authorization_level == AuthorizationLevel::Cell || authorization_level == AuthorizationLevel::RowColumn {
                                            if az.lock().await.authorize(&short_iri, user_uri, Access::CanRead as u8, false).unwrap_or(0) == Access::CanRead as u8 {
                                                json!(short_iri)
                                            } else {
                                                if authorization_level == AuthorizationLevel::Cell {
                                                    json!("d:NotAuthorized")
                                                } else if authorization_level == AuthorizationLevel::RowColumn {
                                                    skip_row = true;
                                                    Value::Null
                                                } else {
                                                    Value::Null
                                                }
                                            }
                                        } else {
                                            json!(short_iri)
                                        }
                                    },
                                    (Some("literal"), Some(data)) => {
                                        if let Some(dt) = r_datatype.and_then(|dt| dt.as_str()) {
                                            match dt {
                                                XSD_INTEGER | XSD_INT | XSD_LONG => json!(data.parse::<i64>().unwrap_or_default()),
                                                XSD_STRING | XSD_NORMALIZED_STRING => json!(data),
                                                XSD_BOOLEAN => json!(data.parse::<bool>().unwrap_or_default()),
                                                XSD_DATE_TIME => json!(data),
                                                XSD_FLOAT | XSD_DOUBLE | XSD_DECIMAL => json!(data.parse::<f64>().unwrap_or_default()),
                                                _ => json!(data), // Для неопознанных типов данных просто возвращаем строку
                                            }
                                        } else {
                                            json!(data) // Если тип данных не указан, возвращаем как строку
                                        }
                                    },
                                    _ => Value::Null, // Для неизвестных или необработанных типов
                                }
                            } else {
                                Value::Null
                            };
                            debug!("processed_value={}", processed_value);
                            if !skip_row {
                                match format {
                                    ResultFormat::Full => {
                                        jrow.insert(var.clone(), processed_value);
                                    },
                                    ResultFormat::Rows => {
                                        row_vec.push(processed_value);
                                    },
                                    ResultFormat::Cols => {
                                        col_data.entry(var.clone()).or_insert_with(|| Value::Array(Vec::new())).as_array_mut().unwrap().push(processed_value);
                                    },
                                }
                            }
                        }

                        if !skip_row {
                            match format {
                                ResultFormat::Full => jrows.push(Value::Object(jrow)),
                                ResultFormat::Rows => jrows.push(Value::Array(row_vec)),
                                _ => (), // Для C

                                         //ols финальная обработка ниже
                            }
                        }
                    }

                    match format {
                        ResultFormat::Full | ResultFormat::Rows => {
                            jres["cols"] = json!(v_cols);
                            jres["rows"] = json!(jrows);
                        },
                        ResultFormat::Cols => {
                            let cols = col_data.into_iter().map(|(k, v)| (k, json!(v))).collect::<Map<_, _>>();
                            jres = json!(cols);
                        },
                    }
                },
                Err(e) => return Err(Error::new(ErrorKind::Other, format!("{:?}", e))),
            },
            Err(e) => return Err(Error::new(ErrorKind::Other, format!("{:?}", e))),
        }

        Ok(jres)
    }
}