use chrono::Utc;
use chrono::{DateTime, NaiveDate};
use std::collections::HashMap;
use string_builder::Builder;
use v_authorization::common::{Access, ACCESS_8_FULL_LIST, ACCESS_C_FULL_LIST, M_IGNORE_EXCLUSIVE, M_IS_EXCLUSIVE};
use v_authorization::{ACLRecord, ACLRecordSet};

pub(crate) fn access_from_char(c: char) -> Option<Access> {
    match c {
        'M' => Some(Access::CanCreate),
        'R' => Some(Access::CanRead),
        'U' => Some(Access::CanUpdate),
        'P' => Some(Access::CanDelete),
        'm' => Some(Access::CantCreate),
        'r' => Some(Access::CantRead),
        'u' => Some(Access::CantUpdate),
        'p' => Some(Access::CantDelete),
        _ => None,
    }
}

pub(crate) fn access8_from_char(c: char) -> Option<u8> {
    match c {
        'M' => Some(1),
        'R' => Some(2),
        'U' => Some(4),
        'P' => Some(8),
        'm' => Some(16),
        'r' => Some(32),
        'u' => Some(64),
        'p' => Some(128),
        _ => None,
    }
}

pub(crate) fn access8_to_char(a: u8) -> Option<char> {
    match a {
        1 => Some('M'),
        2 => Some('R'),
        4 => Some('U'),
        8 => Some('P'),
        16 => Some('m'),
        32 => Some('r'),
        64 => Some('u'),
        128 => Some('p'),
        _ => None,
    }
}

fn encode_value_v1(right: &ACLRecord, outbuff: &mut Builder) {
    outbuff.append(format!("{:X}", right.access));

    if right.marker == M_IS_EXCLUSIVE || right.marker == M_IGNORE_EXCLUSIVE {
        outbuff.append(right.marker);
    }
}

fn encode_value_v2(right: &ACLRecord, outbuff: &mut Builder) {
    let mut set_access = 0;
    for (tag, count) in right.counters.iter() {
        if let Some(c) = access_from_char(*tag) {
            if *count > 0 {
                set_access |= c as u8;
                outbuff.append(*tag);
                if *count > 1 {
                    outbuff.append(count.to_string());
                }
            }
        }
    }

    for a in ACCESS_8_FULL_LIST.iter() {
        if a & right.access & set_access == 0 {
            if let Some(c) = access8_to_char(a & right.access) {
                outbuff.append(c);
            }
        }
    }

    if right.marker == M_IS_EXCLUSIVE || right.marker == M_IGNORE_EXCLUSIVE {
        outbuff.append(right.marker);
    }

    //println!("{} -> {}", access_to_pretty_string(right.access), outbuff);
}

pub fn encode_record(date: Option<DateTime<Utc>>, new_rights: &ACLRecordSet, version_of_index_format: u8) -> String {
    let mut builder = Builder::new(16);

    if let Some(d) = date {
        let formatted_date = d.format("T%y%m%d").to_string();
        builder.append(formatted_date);
        builder.append(',');
    }

    let mut count = 0;

    for key in new_rights.keys() {
        if let Some(right) = new_rights.get(key) {
            if !right.is_deleted {
                builder.append(right.id.clone());
                builder.append(';');

                if version_of_index_format == 1 {
                    encode_value_v1(right, &mut builder);
                } else {
                    encode_value_v2(right, &mut builder);
                }

                builder.append(';');
                count += 1;
            }
        }
    }

    if count == 0 {
        builder.append('X');
    }

    if let Ok(s) = builder.string() {
        s
    } else {
        "X".to_string()
    }
}

fn decode_value_v2(value: &str, rr: &mut ACLRecord, with_count: bool) {
    let mut access = 0;

    let mut tag: Option<char> = None;
    let mut val = String::new();
    for c in value.chars() {
        if c == 'M' || c == 'R' || c == 'U' || c == 'P' || c == 'm' || c == 'r' || c == 'u' || c == 'p' || c == M_IS_EXCLUSIVE || c == M_IGNORE_EXCLUSIVE {
            if c == M_IS_EXCLUSIVE || c == M_IGNORE_EXCLUSIVE {
                rr.marker = c;
            } else {
                if let Some(a) = access_from_char(c) {
                    access |= a as u8;
                }
            }

            if with_count {
                if let Some(t) = tag {
                    rr.counters.insert(t, val.parse::<u16>().unwrap_or(1));
                }
            }

            tag = Some(c);
            val = String::new();
        } else {
            val.push(c);
        }
    }

    if with_count {
        if let Some(t) = tag {
            rr.counters.insert(t, val.parse::<u16>().unwrap_or(1));
        }
    }

    rr.access = access;
}

fn decode_value_v1(value: &str, rr: &mut ACLRecord, with_count: bool) {
    let mut access = 0;
    let mut marker = 0 as char;

    // format value, ver 1
    let mut shift = 0;
    for c in value.chars() {
        if c == M_IS_EXCLUSIVE || c == M_IGNORE_EXCLUSIVE {
            marker = c;
        } else {
            match c.to_digit(16) {
                Some(v) => access |= v << shift,
                None => {
                    eprintln!("ERR! decode_value_v1, fail parse, access is not hex digit {}", value);
                    continue;
                },
            }
            shift += 4;
        }
    }

    rr.access = access as u8;
    rr.marker = marker;

    if with_count {
        for a in ACCESS_8_FULL_LIST.iter() {
            if a & rr.access > 0 {
                if let Some(ac) = access8_to_char(*a) {
                    rr.counters.insert(ac, 1);
                }
            }
        }
    }
}

fn extract_date(s: &str) -> (Option<DateTime<Utc>>, String) {
    if let Some(date_str) = s.strip_prefix('T') {
        if let Some((date_str, rest)) = date_str.split_once(',') {
            if let Ok(date) = NaiveDate::parse_from_str(date_str, "%y%m%d") {
                let datetime = date.and_hms_opt(0, 0, 0).map(|dt| DateTime::<Utc>::from_utc(dt, Utc));
                return (datetime, rest.to_string());
            }
        }
    }

    return (None, s.to_string());
}

pub fn decode_filter(filter_value: String) -> (Option<ACLRecord>, Option<DateTime<Utc>>) {
    let (date, filter_value) = extract_date(&filter_value);

    if filter_value.len() < 3 {
        return (Some(ACLRecord::new_with_access("", 0)), date);
    }

    let mut filters_set: Vec<ACLRecord> = Vec::new();
    decode_rec_to_rights(&filter_value, &mut filters_set);

    if filters_set.is_empty() {
        (Some(ACLRecord::new_with_access(&filter_value, 0)), date)
    } else {
        let el = &mut filters_set[0];
        (Some(ACLRecord::new_with_access(&el.id.clone(), el.access)), date)
    }
}

fn decode_index_record<F>(src: &str, with_counter: bool, mut drain: F) -> (bool, Option<DateTime<Utc>>)
where
    F: FnMut(&str, ACLRecord),
{
    let (date, rest) = extract_date(src);

    if rest.is_empty() {
        return (false, date);
    }

    let tokens: Vec<&str> = rest.split(';').collect();

    let mut idx = 0;
    loop {
        if idx + 1 < tokens.len() {
            let key = tokens[idx];
            let value = tokens[idx + 1];

            if !value.is_empty() {
                let mut rr = ACLRecord::new(key);

                if access_from_char(value.chars().next().unwrap()).is_none() {
                    decode_value_v1(value, &mut rr, with_counter);
                } else {
                    decode_value_v2(value, &mut rr, with_counter);
                }

                //println!("{} -> {}", value, access_to_pretty_string(rr.access));

                drain(key, rr);
            }
        } else {
            break;
        }

        idx += 2;
        if idx >= tokens.len() {
            break;
        }
    }

    (true, date)
}

pub fn decode_rec_to_rights(src: &str, result: &mut Vec<ACLRecord>) -> (bool, Option<DateTime<Utc>>) {
    decode_index_record(src, false, |_key, right| {
        result.push(right);
    })
}

pub fn decode_rec_to_rightset(src: &str, new_rights: &mut ACLRecordSet) -> (bool, Option<DateTime<Utc>>) {
    decode_index_record(src, true, |key, right| {
        new_rights.insert(key.to_owned(), right);
    })
}

pub fn update_counters(counters: &mut HashMap<char, u16>, prev_access: u8, cur_access: u8, is_deleted: bool, is_drop_count: bool) -> u8 {
    let mut out_access = cur_access;

    for access_c in ACCESS_C_FULL_LIST.iter() {
        if let Some(check_bit) = access8_from_char(*access_c) {
            if let Some(cc) = counters.get_mut(access_c) {
                if out_access & check_bit > 0 {
                    if is_drop_count {
                        if is_deleted {
                            *cc = 0;
                            out_access &= !check_bit;
                        } else {
                            *cc = 1;
                            out_access |= check_bit;
                        }
                    } else {
                        if is_deleted {
                            if prev_access & check_bit > 0 {
                                *cc -= 1;
                                if *cc == 0 {
                                    out_access &= !check_bit;
                                }
                            }
                        } else {
                            *cc += 1;
                            out_access |= check_bit;
                        }
                    }
                } else {
                    if is_drop_count {
                        if *cc > 0 {
                            out_access |= check_bit;
                        }
                    }
                }
            } else {
                if !is_deleted && (out_access & check_bit > 0) {
                    out_access |= check_bit;
                    counters.insert(*access_c, 1);
                }
            }
        }
    }

    out_access
}
