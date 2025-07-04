use crate::az_impl::az_lmdb::LmdbAzContext;
use crate::ft_xapian::key2slot::Key2Slot;
use crate::ft_xapian::to_lower_and_replace_delimiters;
use crate::ft_xapian::vql::{Decor, TTA};
use v_individual_model::onto::onto_impl::Onto;
use crate::search::common::{FTQuery, QueryResult};
use crate::v_api::common_type::ResultCode;
use crate::v_api::common_type::OptAuthorize;
use crate::v_authorization::common::AuthorizationContext;
use chrono::NaiveDateTime;
use regex::Regex;
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use std::time::Instant;
use v_authorization::common::Access;
use xapian_rusty::*;

#[derive(Debug, PartialEq)]
enum TokenType {
    Text,
    Number,
    Date,
    Boolean,
}

pub(crate) async fn exec_xapian_query_and_queue_authorize<T>(
    query: &FTQuery,
    xapian_enquire: &mut Enquire,
    add_out_element: fn(uri: &str, ctx: &mut T),
    op_auth: OptAuthorize,
    out_list: &mut T,
    az: &mut LmdbAzContext,
) -> QueryResult {
    let mut sr = QueryResult::default();
    match exec(query, xapian_enquire, add_out_element, op_auth, out_list, az).await {
        Ok(res) => return res,
        Err(e) => match e {
            XError::Xapian(err_code) => {
                if err_code == -10 || err_code == -21 {
                    sr.result_code = ResultCode::DatabaseModifiedError;
                } else {
                    error!("{}, err_code={}", get_xapian_err_type(err_code), err_code);
                    sr.result_code = ResultCode::InternalServerError;
                }
            },
            _ => {
                sr.result_code = ResultCode::InternalServerError;
            },
        },
    }
    sr
}

async fn exec<T>(
    query: &FTQuery,
    xapian_enquire: &mut Enquire,
    add_out_element: fn(uri: &str, ctx: &mut T),
    op_auth: OptAuthorize,
    out_list: &mut T,
    az: &mut LmdbAzContext,
) -> Result<QueryResult> {
    let mut sr = QueryResult::default();

    if query.user.is_empty() {
        sr.result_code = ResultCode::TicketNotFound;
        return Ok(sr);
    }

    let top = if query.top == 0 {
        10000
    } else {
        query.top
    };

    let limit = if query.limit == 0 {
        10000
    } else {
        query.limit
    };

    let mut read_count = 0;

    let mut matches = xapian_enquire.get_mset(query.from, limit)?;
    let mut processed: i32 = 0;

    sr.estimated = matches.get_matches_estimated()? as i64;

    let mut it = matches.iterator()?;

    let mut auth_time_ms = 0u64;

    while it.is_next()? {
        processed += 1;
        if (processed % 1000) == 0 {
            info!("processed {}", processed);
        }

        let subject_id = it.get_document_data()?;

        if subject_id.is_empty() {
            it.next()?;
            continue;
        }

        let mut is_passed = true;

        if op_auth == OptAuthorize::YES {
            async {
                let auth_start = Instant::now();
                match az.authorize(&subject_id, &query.user, Access::CanRead as u8, true) {
                    Ok(result) => {
                        if result != Access::CanRead as u8 {
                            is_passed = false;
                        } else {
                            debug!("subject_id=[{}] authorized for user_id=[{}]", subject_id, query.user);
                        }
                    },
                    Err(e) => {
                        is_passed = false;
                        error!("Ошибка при проверке авторизации: subject_id=[{}], user_id=[{}], error=[{:?}]", subject_id, query.user, e);
                    }
                }
                auth_time_ms += auth_start.elapsed().as_millis() as u64;
            }
            .await;
        }

        if is_passed {
            add_out_element(&subject_id, out_list);
            read_count += 1;
            if read_count >= top {
                break;
            }
        }
        it.next()?;
    }

    sr.result_code = ResultCode::Ok;
    sr.processed = processed as i64;
    sr.count = read_count as i64;
    sr.cursor = (query.from + processed) as i64;
    sr.authorize_time = auth_time_ms as i64;

    Ok(sr)
}

pub(crate) struct AuxContext<'a> {
    pub(crate) key2slot: &'a Key2Slot,
    pub(crate) qp: &'a mut QueryParser,
    pub(crate) onto: &'a Onto,
}

pub(crate) fn transform_vql_to_xapian(
    ctx: &mut AuxContext,
    tta: &mut TTA,
    l_token: Option<&mut String>,
    op: Option<&mut String>,
    query: &mut Query,
    _rd: &mut f64,
    _level: i32,
) -> Result<String> {
    let mut query_r = Query::new()?;
    let mut query_l = Query::new()?;
    let mut rd = 0.0;
    let mut ld = 0.0;

    if tta.op == ">" || tta.op == "<" {
        if tta.l.is_none() || tta.r.is_none() {
            return Err(XError::from(Error::new(ErrorKind::Other, format!("transform_vql_to_xapian, invalid tta=[{}]", tta))));
        }

        let mut ls = String::new();
        if let Some(l) = &mut tta.l {
            ls = transform_vql_to_xapian(ctx, l, None, None, &mut query_l, &mut ld, _level + 1)?;
        }

        let mut rs = String::new();
        if let Some(r) = &mut tta.r {
            rs = transform_vql_to_xapian(ctx, r, None, None, &mut query_r, &mut rd, _level + 1)?;
        }

        let (rs_type, value) = get_token_type(&rs);
        if rs_type == TokenType::Date || rs_type == TokenType::Number {
            if let Some(l_out) = l_token {
                *l_out = ls;
            }
            if let Some(op_out) = op {
                *op_out = tta.op.clone();
            }

            *_rd = value;
            return Ok(rs);
        }
    } else if tta.op == "==" || tta.op == "!=" || tta.op == "===" {
        let mut is_strict_equality = false;
        if tta.op == "===" {
            is_strict_equality = true;
            tta.op = "==".to_string();
        }

        if tta.l.is_none() || tta.r.is_none() {
            return Err(XError::from(Error::new(ErrorKind::Other, format!("transform_vql_to_xapian, invalid tta=[{}]", tta))));
        }

        let mut ls = String::new();
        if let Some(l) = &mut tta.l {
            ls = transform_vql_to_xapian(ctx, l, None, None, &mut query_l, &mut ld, _level + 1)?;
        }

        let mut rs = String::new();
        if let Some(r) = &mut tta.r {
            rs = transform_vql_to_xapian(ctx, r, None, None, &mut query_r, &mut rd, _level + 1)?;
        }

        if !is_strict_equality {
            if let Some(s) = add_subclasses_to_query(&rs, ctx.onto) {
                rs = s;
            }
        }

        if query_l.is_empty() && query_r.is_empty() {
            if ls != "*" {
                if ls == "@" {
                    let mut flags = FeatureFlag::FlagDefault as i16 | FeatureFlag::FlagWildcard as i16;
                    let mut query_str = format!("uid_{}", to_lower_and_replace_delimiters(&rs));

                    if tta.op == "!=" {
                        flags |= FeatureFlag::FlagPureNot as i16;
                        query_str = "NOT ".to_owned() + &query_str;
                    }

                    *query = parse_query(ctx.qp, &query_str, flags)?;
                } else {
                    let mut rs_first_byte = 0;

                    let rs_last_byte = if !rs.is_empty() {
                        rs_first_byte = rs.as_bytes()[0];
                        rs.as_bytes()[rs.len() - 1]
                    } else {
                        0
                    };

                    let wslot = if !rs.is_empty() && rs_first_byte == b'*' && is_good_token(&rs) {
                        ctx.key2slot.get_slot(&(ls + "#F"))
                    } else {
                        ctx.key2slot.get_slot(&ls)
                    };

                    if let Some(slot) = wslot {
                        let (rs_type, value) = get_token_type(&rs);

                        if rs_type == TokenType::Boolean {
                            let xtr = format!("X{}D", slot);
                            let query_str = if value == 0.0 {
                                "F"
                            } else {
                                "T"
                            };
                            let flags = FeatureFlag::FlagDefault as i16 | FeatureFlag::FlagPhrase as i16 | FeatureFlag::FlagLovehate as i16;

                            *query = parse_query_with_prefix(ctx.qp, query_str, flags, &xtr)?;
                        } else if let Some(r) = &tta.r {
                            if r.token_decor == Decor::QUOTED || (rs.find('*').is_some() && is_good_token(&rs)) {
                                if rs.find('*').is_some() && rs_first_byte == b'+' && !is_good_token(&rs) {
                                    rs = Regex::new(r"[*]").unwrap().replace_all(&rs, "").to_string();
                                }

                                let mut query_str = rs;
                                if rs_first_byte == b'*' {
                                    query_str = query_str.chars().rev().collect();
                                }

                                if let Some(s) = normalize_if_uri(&query_str) {
                                    query_str = s;
                                }

                                let xtr = format!("X{}X", slot);
                                let mut flags = FeatureFlag::FlagDefault as i16
                                    | FeatureFlag::FlagWildcard as i16
                                    | FeatureFlag::FlagPhrase as i16
                                    | FeatureFlag::FlagLovehate as i16;

                                if tta.op == "!=" {
                                    flags |= FeatureFlag::FlagPureNot as i16;
                                    query_str = "NOT ".to_owned() + &query_str;
                                }

                                *query = parse_query_with_prefix(ctx.qp, &query_str, flags, &xtr)?;
                            } else if r.token_decor == Decor::RANGE {
                                let vals: Vec<&str> = rs.split(',').collect();
                                if vals.len() == 2 {
                                    let (tt, c_from) = get_token_type(vals.first().unwrap());
                                    if tt == TokenType::Date || tt == TokenType::Number {
                                        let (tt, c_to) = get_token_type(vals.get(1).unwrap());
                                        if tt == TokenType::Date || tt == TokenType::Number {
                                            *query = Query::new_range(XapianOp::OpValueRange, slot, c_from, c_to)?;
                                        }
                                    }
                                } else if vals.len() == 1 {
                                    let mut el = rs.clone();
                                    if rs_first_byte == b'\'' && el.len() > 2 && rs_last_byte == b'\'' {
                                        if let Some(s) = &rs.get(1..rs.len() - 1) {
                                            el = (*s).to_string();
                                        }
                                    }

                                    let mut query_str = el;
                                    let xtr = format!("X{}X", slot);

                                    if !is_strict_equality {
                                        if let Some(s) = add_subclasses_to_query(&rs, ctx.onto) {
                                            query_str = s;
                                        }
                                    }

                                    let flags = FeatureFlag::FlagDefault as i16
                                        | FeatureFlag::FlagWildcard as i16
                                        | FeatureFlag::FlagPhrase as i16
                                        | FeatureFlag::FlagLovehate as i16;

                                    *query = parse_query_with_prefix(ctx.qp, &query_str, flags, &xtr)?;
                                } else {
                                    match rs.parse::<f64>() {
                                        Ok(d) => {
                                            *query = Query::new_double_with_prefix(&format!("X{}X", slot), d)?;
                                        },
                                        Err(_) => {
                                            return Err(XError::from(Error::new(
                                                ErrorKind::Other,
                                                format!("transform_vql_to_xapian, invalid tta=[{}]", tta),
                                            )));
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                let mut query_str = rs;
                if let Some(s) = normalize_if_uri(&query_str) {
                    query_str = s;
                }

                if query_str.find('*').is_some() && is_good_token(&query_str) {
                    let mut flags = FeatureFlag::FlagDefault as i16 | FeatureFlag::FlagWildcard as i16 | FeatureFlag::FlagPhrase as i16;

                    if tta.op == "!=" {
                        flags |= FeatureFlag::FlagPureNot as i16;
                        query_str = "NOT ".to_owned() + &query_str;
                    }

                    *query = parse_query(ctx.qp, &query_str, flags)?;
                } else {
                    let flags = FeatureFlag::FlagDefault as i16;

                    *query = parse_query(ctx.qp, &query_str, flags)?;
                }
            }
        }
    } else if tta.op == "&&" {
        let mut t_op_l = String::new();
        let mut t_op_r = String::new();
        let mut token_l = String::new();

        //let mut tta_r = String::new();
        if let Some(t) = &mut tta.r {
            //tta_r =
            transform_vql_to_xapian(ctx, t, Some(&mut token_l), Some(&mut t_op_r), &mut query_r, &mut rd, _level + 1)?;
        }

        if !t_op_r.is_empty() {
            if let Some(v) = op {
                *v = t_op_r.clone();
            }
        }

        let mut tta_l = String::new();
        if let Some(t) = &mut tta.l {
            tta_l = transform_vql_to_xapian(ctx, t, None, Some(&mut t_op_l), &mut query_l, &mut ld, _level + 1)?;
        }

        //        if !t_op_l.is_empty() {
        //            if let Some(v) = op {
        //                *v = t_op_l.clone();
        //            }
        //        }

        if !token_l.is_empty() && !tta_l.is_empty() {
            let mut c_from = if t_op_r == ">" {
                rd
            } else {
                0.0
            };

            let mut c_to = if t_op_r == "<" {
                rd
            } else {
                0.0
            };

            if t_op_l == ">" {
                c_from = ld;
            }
            if t_op_l == "<" {
                c_to = ld;
            }

            if let Some(slot) = ctx.key2slot.get_slot(&token_l) {
                query_r = Query::new_range(XapianOp::OpValueRange, slot, c_from, c_to)?;

                if query_l.is_empty() {
                    *query = query_r;
                } else if !query_r.is_empty() {
                    *query = query_l.add_right(XapianOp::OpAnd, &mut query_r)?;
                }
            }
            if query.is_empty() {
                return Err(XError::from(Error::new(ErrorKind::Other, format!("transform_vql_to_xapian, invalid tta=[{}]", tta))));
            }
        } else if !query_r.is_empty() {
            if query_l.is_empty() {
                *query = query_r;
            } else if !query_r.is_empty() {
                *query = query_l.add_right(XapianOp::OpAnd, &mut query_r)?;
            }

            if query.is_empty() {
                return Err(XError::from(Error::new(ErrorKind::Other, format!("transform_vql_to_xapian, invalid tta=[{}]", tta))));
            }
        } else {
            *query = query_l;
        }

        if !tta_l.is_empty() && tta_l.is_empty() {
            *_rd = rd;
            return Ok(tta_l);
        }

        if !tta_l.is_empty() && tta_l.is_empty() {
            *_rd = ld;
            return Ok(tta_l);
        }
    } else if tta.op == "||" {
        //let mut tta_r = String::new();
        if let Some(t) = &mut tta.r {
            //tta_r =
            transform_vql_to_xapian(ctx, t, None, None, &mut query_r, &mut rd, _level + 1)?;
        }

        //let mut tta_l = String::new();
        if let Some(t) = &mut tta.l {
            //tta_l =
            transform_vql_to_xapian(ctx, t, None, None, &mut query_l, &mut ld, _level + 1)?;
        }

        if !query_l.is_empty() && !query_r.is_empty() {
            *query = query_l.add_right(XapianOp::OpOr, &mut query_r)?;
        }

        if query.is_empty() {
            return Err(XError::from(Error::new(ErrorKind::Other, format!("transform_vql_to_xapian, invalid tta=[{}]", tta))));
        }
    } else {
        return Ok(tta.op.clone());
    }

    Ok(Default::default())
}

pub fn get_sorter(sort: &str, key2slot: &Key2Slot) -> Result<Option<MultiValueKeyMaker>> {
    if !sort.is_empty() {
        let fields: Vec<&str> = sort.split(',').collect();
        for f in fields {
            let el: Vec<&str> = f.trim().split(' ').collect();

            if el.len() == 2 {
                let key = el.first().unwrap().replace('\'', " ");

                let direction = el.get(1).unwrap().trim();
                let asc_desc = direction != "desc";

                if let Some(slot) = key2slot.get_slot(key.trim()) {
                    debug!("use sort {} {}", key, asc_desc);
                    let mut sorter = MultiValueKeyMaker::new()?;
                    sorter.add_value(slot, asc_desc)?;
                    return Ok(Some(sorter));
                }
            } else {
                warn!("ignore invalid sort [{}]", f);
            }
        }
    }
    Ok(None)
}

fn add_subclasses_to_query(rs: &str, onto: &Onto) -> Option<String> {
    if rs.find(':').is_some() && rs.find(',').is_none() {
        let mut new_rs = rs.to_string();
        let mut subclasses = HashSet::new();
        onto.get_subs(&new_rs, &mut subclasses);

        for cs in subclasses.iter() {
            new_rs.push_str(" OR ");
            new_rs.push_str(cs);
        }

        return Some(to_lower_and_replace_delimiters(&new_rs));
    }
    None
}

fn parse_query_with_prefix(qp: &mut QueryParser, query_str: &str, flags: i16, prefix: &str) -> Result<Query> {
    if let Ok(q) = qp.parse_query_with_prefix(query_str, flags, prefix) {
        Ok(q)
    } else {
        warn!("The search query has been changed: [{}]->[\"{}\"]", query_str, query_str);
        Ok(qp.parse_query(&(format!("\"{}\"", query_str)), flags)?)
    }
}

fn parse_query(qp: &mut QueryParser, query_str: &str, flags: i16) -> Result<Query> {
    if let Ok(q) = qp.parse_query(query_str, flags) {
        Ok(q)
    } else {
        warn!("The search query has been changed: [{}]->[\"{}\"]", query_str, query_str);
        Ok(qp.parse_query(&(format!("\"{}\"", query_str)), flags)?)
    }
}

// In xapian_vql.rs, we need to modify the get_token_type function.
// Here's the fixed version of the relevant part:

fn get_token_type(token_in: &str) -> (TokenType, f64) {
    debug!("token=[{}]", token_in);

    let token = token_in.trim().as_bytes();

    if token == b"true" {
        return (TokenType::Boolean, 1.0);
    } else if token == b"false" {
        return (TokenType::Boolean, 0.0);
    } else if token.len() == 19 && token[4] == b'-' && token[7] == b'-' && token[10] == b'T' && token[13] == b':' && token[16] == b':' {
        if let Ok(nv) = NaiveDateTime::parse_from_str(token_in, "%Y-%m-%dT%H:%M:%S") {
            return (TokenType::Date, nv.and_utc().timestamp() as f64);
        }
    } else if token.len() == 24 && token[4] == b'-' && token[7] == b'-' && token[10] == b'T' && token[13] == b':' && token[16] == b':' && token[19] == b'.'
    {
        let (token, _) = token_in.split_at(token_in.len() - 1);
        if let Ok(nv) = NaiveDateTime::parse_from_str(token, "%Y-%m-%dT%H:%M:%S%.f") {
            return (TokenType::Date, nv.and_utc().timestamp() as f64);
        }
    }

    if let Ok(v) = token_in.parse::<i64>() {
        return (TokenType::Number, v as f64);
    }

    if let Ok(v) = token_in.parse::<f64>() {
        return (TokenType::Number, v);
    }

    (TokenType::Text, 0.0)
}

fn is_good_token(str: &str) -> bool {
    let mut count_alpha = 0;
    let mut count_number = 0;

    for dd in str.chars() {
        if dd.is_alphabetic() {
            count_alpha += 1;
        }
        if dd.is_numeric() {
            count_number += 1;
        }
    }

    debug!("get_count_alpha, str=[{}], count_alpha=[{}]", str, count_alpha);

    if count_alpha + count_number < 3 {
        return false;
    }

    if count_alpha + count_number < 4 && count_number == 3 {
        return false;
    }

    true
}

fn normalize_if_uri(str: &str) -> Option<String> {
    if Regex::new(r"^[a-zA-Z0-9_-]+:[a-zA-Z0-9_-]*$").unwrap().is_match(str) {
        return Some(to_lower_and_replace_delimiters(str));
    }
    None
}
