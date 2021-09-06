use crate::onto::individual::*;
use crate::onto::msgpack2individual::*;

#[derive(PartialEq, Debug)]
pub enum RawType {
    Cbor,
    Json,
    Msgpack,
    Unknown,
}

pub fn parse_to_predicate(expect_predicate: &str, iraw: &mut Individual) -> bool {
    if iraw.raw.raw_type == RawType::Msgpack {
        if let Err(e) = parse_msgpack_to_predicate(expect_predicate, iraw) {
            if !e.is_empty() {
                error!("parse for [{}], err={}", expect_predicate, e);
            }
            return false;
        }
        return true;
    } else if iraw.raw.raw_type == RawType::Cbor {
    }

    false
}

const MSGPACK_MAGIC_HEADER: u8 = 146;

pub fn parse_raw(iraw: &mut Individual) -> Result<(), i8> {
    if iraw.raw.data.is_empty() {
        return Ok(());
    }

    let traw: &[u8] = iraw.raw.data.as_slice();

    if traw[0] == MSGPACK_MAGIC_HEADER {
        iraw.raw.raw_type = RawType::Msgpack;
    } else {
        iraw.raw.raw_type = RawType::Cbor;
    }

    let res = if iraw.raw.raw_type == RawType::Msgpack {
        parse_msgpack(&mut iraw.raw)
    } else {
        Err(-1)
    };

    if let Ok(uri) = res {
        iraw.obj.uri = uri;
        return Ok(());
    }

    Err(-1)
}
