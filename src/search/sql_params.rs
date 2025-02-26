use v_individual_model::onto::individual::Individual;
use v_individual_model::onto::resource::Value;
use crate::search::sql_lex_tree::tr_statement;
use klickhouse::query_parser::parse_query_arguments;
use regex::Regex;
use sqlparser::dialect::AnsiDialect;
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;
use std::io::{Error, ErrorKind};

pub fn parse_sql_query_arguments(query: &str, params: &mut Individual, dialect: &str) -> Result<String, Error> {
    match dialect {
        "clickhouse" => {
            let re = Regex::new(r"(\{[^\}]+\}|'\{[^\}]+\}')").unwrap();

            let mut arg_names = Vec::new();
            let mut arg_values = Vec::new();
            let res_query = re
                .replace_all(query, |caps: &regex::Captures| {
                    let arg_name_with_braces = caps.get(1).unwrap().as_str();
                    let arg_name = arg_name_with_braces.trim_start_matches("'{").trim_end_matches("}'").trim_start_matches("{").trim_end_matches("}");
                    arg_names.push(arg_name.to_string());
                    let arg_index = arg_names.len();
                    format!("${}", arg_index)
                })
                .to_string();

            //info!("@res_query={:?}", res_query);
            //info!("@arg_names={:?}", arg_names);

            for arg_name in &arg_names {
                //info!("@arg_name={}", arg_name);
                if let Some(res) = params.get_obj().get_resources().get(arg_name) {
                    let arg_value = match &res[0].value {
                        Value::Uri(v) | Value::Str(v, _) => klickhouse::Value::string(v),
                        Value::Int(v) => klickhouse::Value::Int64(*v),
                        Value::Bool(v) => klickhouse::Value::UInt8(*v as u8),
                        Value::Num(_m, _d) => klickhouse::Value::Float64(res[0].get_float()),
                        Value::Datetime(v) => klickhouse::Value::DateTime(klickhouse::DateTime(klickhouse::Tz::UTC, *v as u32)),
                        _ => return Err(Error::new(ErrorKind::Other, format!("Unsupported value type {:?}", res[0].value))),
                    };
                    arg_values.push(arg_value);
                } else {
                    return Err(Error::new(ErrorKind::Other, format!("Variable {} not found in params", arg_name)));
                }
            }

            let res_query = parse_query_arguments(&res_query, &arg_values);
            //info!("@res_query={:?}", res_query);

            return Ok(res_query);
        },
        "mysql" | _ => {
            let lex_tree = match dialect {
                "mysql" => Parser::parse_sql(&MySqlDialect {}, query),
                _ => Parser::parse_sql(&AnsiDialect {}, query),
            };

            match lex_tree {
                Ok(mut ast) => {
                    if let Some(el) = ast.iter_mut().next() {
                        tr_statement(el, params)?;
                        debug!("NEW: {}", el);
                        return match dialect {
                            "mysql" => Ok(el.to_string()),
                            _ => Err(Error::new(ErrorKind::Other, "unknown SQL dialect")),
                        };
                    }
                },
                Err(e) => {
                    error!("{:?}", e);
                },
            }
            Err(Error::new(ErrorKind::Other, "?"))
        },
    }
}
