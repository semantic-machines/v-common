use v_authorization_impl::AzContext;
use crate::module::module_impl::Module;
use crate::search::common::{get_short_prefix, split_full_prefix, AuthorizationLevel, PrefixesCache, QueryResult, ResultFormat};
use crate::v_api::common_type::ResultCode;
use futures::lock::Mutex;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::time::Instant;
use v_authorization::common::{Access, AuthorizationContext};

use super::awc_wrapper::{Client, HeaderValue, ACCEPT, CONTENT_TYPE};

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
    pub(crate) az: AzContext,
}

impl Default for SparqlClient {
    fn default() -> Self {
        let client = Client::builder().max_http_version(http::Version::HTTP_11).finish();

        SparqlClient {
            point: format!("{}/{}?{}", Module::get_property::<String>("sparql_db").unwrap_or_default(), "query", "default"),
            client,
            az: AzContext::default(),
        }
    }
}

impl SparqlClient {
    pub async fn query_select_ids(&mut self, user_uri: &str, query: String, prefix_cache: &PrefixesCache) -> QueryResult {
        let total_time = Instant::now();

        #[cfg(feature = "awc_2")]
        let res_req = self
            .client
            .post(&self.point)
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "application/sparql-results+json")
            .send_body(query)
            .await;

        #[cfg(feature = "awc_3")]
        let res_req = self
            .client
            .post(&self.point)
            .insert_header((CONTENT_TYPE, HeaderValue::from_static("application/sparql-query")))
            .insert_header((ACCEPT, HeaderValue::from_static("application/sparql-results+json")))
            .send_body(query)
            .await;

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

                    let mut auth_time_ms = 0u64;
                    for el in v.results.bindings {
                        let r = &el[var];
                        if r["type"] == "uri" {
                            if let Some(v) = r["value"].as_str() {
                                let iri = split_full_prefix(v);

                                let fullprefix = iri.0;
                                let prefix = get_short_prefix(fullprefix, &prefix_cache);
                                let short_iri = format!("{prefix}:{}", iri.1);

                                let auth_start = Instant::now();
                                if self.az.authorize(&short_iri, user_uri, Access::CanRead as u8, true).unwrap_or(0) == Access::CanRead as u8 {
                                    qres.result.push(short_iri);
                                }
                                auth_time_ms += auth_start.elapsed().as_millis() as u64;
                            }
                        }
                    }
                    qres.processed = qres.result.len() as i64;
                    qres.authorize_time = auth_time_ms as i64;
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
        res_format: ResultFormat,
        authorization_level: AuthorizationLevel,
        az: &Mutex<AzContext>,
        prefix_cache: &PrefixesCache,
    ) -> Result<Value, Error> {
        #[cfg(feature = "awc_2")]
        let mut response = self
            .client
            .post(&self.point)
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "application/sparql-results+json")
            .send_body(query)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?;

        #[cfg(feature = "awc_3")]
        let mut response = self
            .client
            .post(&self.point)
            .insert_header((CONTENT_TYPE, HeaderValue::from_static("application/sparql-query")))
            .insert_header((ACCEPT, HeaderValue::from_static("application/sparql-results+json")))
            .send_body(query)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?;

        let mut jres = Value::default();

        let body = response.body().limit(usize::MAX).await.map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?;

        let v: SparqlResponse = serde_json::from_slice(&body).map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?;

        // Обработка заголовков
        let v_cols: Vec<Value> = v.head.vars.iter().map(|el| json!(el)).collect();
        // Подготовка данных для разных форматов
        let mut jrows = Vec::new();
        let mut col_data: Map<String, Value> = Map::new();

        let mut excluded_rows = HashSet::new();
        let mut row_count = 0;

        for el in v.results.bindings {
            let mut skip_row = false;
            let mut jrow = if res_format == ResultFormat::Full {
                Value::Object(Map::new())
            } else {
                Value::Array(Vec::new())
            };

            for var in v.head.vars.iter() {
                let mut is_authorized = true;
                let r = &el[var];

                let processed_value = if let (Some(r_type), Some(r_value)) = (r.get("type"), r.get("value")) {
                    if r_type == "uri" {
                        if authorization_level == AuthorizationLevel::Cell || authorization_level == AuthorizationLevel::RowColumn {
                            if r_type == "uri" {
                                if let Some(val) = r_value.as_str() {
                                    let iri = split_full_prefix(val);
                                    let prefix = get_short_prefix(iri.0, prefix_cache);
                                    let short_iri = format!("{prefix}:{}", iri.1);

                                    if az.lock().await.authorize(&short_iri, user_uri, Access::CanRead as u8, false).unwrap_or(0) != Access::CanRead as u8 {
                                        is_authorized = false;
                                        if authorization_level == AuthorizationLevel::Cell {
                                            json!("v-s:NotAuthorized")
                                        } else {
                                            excluded_rows.insert(row_count);
                                            if res_format == ResultFormat::Rows {
                                                skip_row = true;
                                            }
                                            Value::Null
                                        }
                                    } else {
                                        json!(short_iri)
                                    }
                                } else {
                                    Value::Null
                                }
                            } else {
                                json!(r_value)
                            }
                        } else {
                            if let Some(val) = r_value.as_str() {
                                let iri = split_full_prefix(val);
                                let prefix = get_short_prefix(iri.0, prefix_cache);
                                let short_iri = format!("{prefix}:{}", iri.1);
                                json!(short_iri)
                            } else {
                                Value::Null
                            }
                        }
                    } else {
                        json!(r_value)
                    }
                } else {
                    Value::Null
                };

                if is_authorized && !skip_row {
                    match res_format {
                        ResultFormat::Full => {
                            if let Some(obj) = jrow.as_object_mut() {
                                obj.insert(var.clone(), processed_value);
                            }
                        },
                        ResultFormat::Rows => {
                            if let Some(arr) = jrow.as_array_mut() {
                                arr.push(processed_value);
                            }
                        },
                        ResultFormat::Cols => {
                            col_data.entry(var.clone()).or_insert_with(|| Value::Array(Vec::new())).as_array_mut().unwrap().push(processed_value);
                        },
                    }
                }
            }

            if !skip_row {
                match res_format {
                    ResultFormat::Full | ResultFormat::Rows => jrows.push(jrow),
                    _ => (),
                }
            }
            row_count += 1;
        }

        match res_format {
            ResultFormat::Full | ResultFormat::Rows => {
                jres["cols"] = json!(v_cols);
                jres["rows"] = json!(jrows);
            },
            ResultFormat::Cols => {
                if authorization_level == AuthorizationLevel::RowColumn {
                    for (_col_name, col_values) in col_data.iter_mut() {
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

                jres = json!(col_data);
            },
        }

        Ok(jres)
    }
}
