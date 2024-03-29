#[derive(Eq, PartialEq, Debug, Clone)]
#[repr(u8)]
pub enum DataType {
    Uri = 1,
    String = 2,
    Integer = 4,
    Datetime = 8,
    Decimal = 32,
    Boolean = 64,
    Binary = 128,
}

impl DataType {
    pub fn new_from_str(l: &str) -> Option<DataType> {
        match l {
            "Uri" => Some(DataType::Uri),
            "String" => Some(DataType::String),
            "Integer" => Some(DataType::Integer),
            "Datetime" => Some(DataType::Datetime),
            "Decimal" => Some(DataType::Decimal),
            "Boolean" => Some(DataType::Boolean),
            "Binary" => Some(DataType::Binary),
            _ => None,
        }
    }

    pub fn new_from_u64(l: u64) -> Option<DataType> {
        match l {
            1 => Some(DataType::Uri),
            2 => Some(DataType::String),
            4 => Some(DataType::Integer),
            8 => Some(DataType::Datetime),
            32 => Some(DataType::Decimal),
            64 => Some(DataType::Boolean),
            128 => Some(DataType::Binary),
            _ => None,
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Lang {
    data: Option<String>,
}

impl Lang {
    pub fn none() -> Lang {
        Lang {
            data: None,
        }
    }

    pub fn is_some(&self) -> bool {
        self.data.is_some()
    }

    pub fn new_from_str(l: &str) -> Lang {
        let lu = l.to_uppercase();

        if lu == "NONE" || lu == "UNDEFINED" {
            Lang::none()
        } else {
            Lang {
                data: Some(lu),
            }
        }
    }

    pub fn new_from_i64(l: i64) -> Lang {
        match l {
            1 => Lang {
                data: Some("RU".to_owned()),
            },
            2 => Lang {
                data: Some("EN".to_owned()),
            },
            _ => Lang::none(),
        }
    }

    pub fn to_string(&self) -> &str {
        if let Some(v) = &self.data {
            v
        } else {
            "NONE"
        }
    }
}

pub fn exponent_to_scale(m: &i64, e: &i64) -> (i64, u32) {
    let scale = if *e < 0 {
        (*e * -1) as u32
    } else {
        0
    };

    let num = if *e > 0 {
        *m * 10_i64.pow(*e as u32)
    } else {
        *m
    };

    (num, scale)
}
