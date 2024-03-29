extern crate rmp as msgpack;
use crate::onto::datatype::DataType;
use crate::onto::individual::Individual;
use crate::onto::resource::Resource;
use msgpack::encode::*;
use std::io::Error;
use std::io::Write;

fn write_resource(out: &mut Vec<u8>, r: &Resource) -> Result<(), Error> {
    match r.rtype {
        DataType::Integer => {
            write_array_len(out, 2)?;
            write_u8(out, r.rtype.clone() as u8)?;
            write_sint(out, r.get_int())?;
        },
        DataType::Binary => {
            write_array_len(out, 2)?;
            write_u8(out, r.rtype.clone() as u8)?;
            let data = r.get_binary();
            write_str_len(out, data.len() as u32)?;
            out.write_all(data).map_err(ValueWriteError::InvalidDataWrite)?;
        },
        DataType::Boolean => {
            write_array_len(out, 2)?;
            write_u8(out, r.rtype.clone() as u8)?;
            write_bool(out, r.get_bool())?;
        },
        DataType::Datetime => {
            write_array_len(out, 2)?;
            write_u8(out, r.rtype.clone() as u8)?;
            write_sint(out, r.get_datetime())?;
        },
        DataType::Decimal => {
            write_array_len(out, 3)?;
            write_u8(out, r.rtype.clone() as u8)?;
            let dec = r.get_num();
            write_sint(out, dec.0)?;
            write_sint(out, dec.1)?;
        },
        DataType::String => {
            let s = r.get_str();
            let l = r.get_lang();

            if l.is_some() {
                write_array_len(out, 3)?;
            } else {
                write_array_len(out, 2)?;
            }
            write_u8(out, r.rtype.clone() as u8)?;

            if s.is_empty() {
                write_nil(out)?;
            } else {
                write_str(out, s)?;
            }

            if l.is_some() {
                match l.to_string() {
                    "RU" => {
                        write_u8(out, 1_u8)?;
                    },
                    "EN" => {
                        write_u8(out, 2_u8)?;
                    },
                    "NONE" => {
                        write_u8(out, 0_u8)?;
                    },
                    v => {
                        write_str(out, v)?;
                    },
                }
            }
        },
        DataType::Uri => {
            let s = r.get_uri();
            write_array_len(out, 2)?;
            write_u8(out, r.rtype.clone() as u8)?;
            if s.is_empty() {
                write_nil(out)?;
            } else {
                write_str(out, s)?;
            }
        },
    }

    Ok(())
}

pub fn to_msgpack(indv: &Individual, out: &mut Vec<u8>) -> Result<(), Error> {
    write_array_len(out, 2)?;
    write_str(out, &indv.obj.uri)?;
    write_map_len(out, indv.obj.resources.len() as u32)?;
    for (predicate, resources) in &indv.obj.resources {
        write_str(out, predicate)?;
        write_array_len(out, resources.len() as u32)?;

        for r in resources {
            write_resource(out, r)?;
        }
    }

    Ok(())
}
