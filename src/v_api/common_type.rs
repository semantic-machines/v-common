use serde::de::Error;
use serde::de::Visitor;
use serde::Deserializer;
use serde::Serializer;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
#[repr(u16)]
pub enum ResultCode {
    /// 0
    Zero = 0,

    /// 200
    Ok = 200,

    /// 201
    Created = 201,

    /// 204
    NoContent = 204,

    /// 400
    BadRequest = 400,

    /// 403
    Forbidden = 403,

    /// 404
    NotFound = 404,

    /// 422
    UnprocessableEntity = 422,

    /// 423
    Locked = 423,

    /// 429
    TooManyRequests = 429,

    /// 430
    TooManyRequestsChangePassword = 430,

    /// 463
    ChangePasswordForbidden = 463,

    /// 464
    SecretExpired = 464,

    /// 465
    EmptyPassword = 465,

    /// 466
    NewPasswordIsEqualToOld = 466,

    /// 467
    InvalidPassword = 467,

    /// 468
    InvalidSecret = 468,

    /// 469
    PasswordExpired = 469,

    /// 470
    TicketNotFound = 470,

    /// 471
    TicketExpired = 471,

    /// 472
    NotAuthorized = 472,

    /// 473
    AuthenticationFailed = 473,

    /// 474
    NotReady = 474,

    /// 475
    FailOpenTransaction = 475,

    /// 476
    FailCommit = 476,

    /// 477
    FailStore = 477,

    /// 500
    InternalServerError = 500,

    /// 501
    NotImplemented = 501,

    /// 503
    ServiceUnavailable = 503,

    InvalidIdentifier = 904,

    /// 999
    DatabaseModifiedError = 999,

    /// 1021
    DiskFull = 1021,

    /// 1022
    DuplicateKey = 1022,

    /// 1118
    SizeTooLarge = 1118,

    /// 4000
    ConnectError = 4000,
}

#[derive(Debug, Eq, PartialEq)]
pub enum OptAuthorize {
    NO,
    YES,
}

impl From<std::io::Error> for ResultCode {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => ResultCode::NotFound,
            std::io::ErrorKind::PermissionDenied => ResultCode::Forbidden,
            // ... other std::io::ErrorKind variants ...
            _ => ResultCode::InternalServerError,
        }
    }
}

impl Serialize for ResultCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(*self as u16)
    }
}

impl<'de> Deserialize<'de> for ResultCode {
    fn deserialize<D>(deserializer: D) -> Result<ResultCode, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = ResultCode;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("!!!")
            }

            fn visit_u64<E>(self, v: u64) -> Result<ResultCode, E>
            where
                E: Error,
            {
                Ok(ResultCode::from_i64(v as i64))
            }
        }

        deserializer.deserialize_any(FieldVisitor)
    }
}

impl ResultCode {
    pub fn from_i64(value: i64) -> ResultCode {
        match value {
            0 => ResultCode::Zero,
            200 => ResultCode::Ok,
            201 => ResultCode::Created,
            204 => ResultCode::NoContent,
            400 => ResultCode::BadRequest,
            403 => ResultCode::Forbidden,
            404 => ResultCode::NotFound,
            422 => ResultCode::UnprocessableEntity,
            429 => ResultCode::TooManyRequests,
            463 => ResultCode::ChangePasswordForbidden,
            464 => ResultCode::SecretExpired,
            465 => ResultCode::EmptyPassword,
            466 => ResultCode::NewPasswordIsEqualToOld,
            467 => ResultCode::InvalidPassword,
            468 => ResultCode::InvalidSecret,
            469 => ResultCode::PasswordExpired,
            470 => ResultCode::TicketNotFound,
            471 => ResultCode::TicketExpired,
            472 => ResultCode::NotAuthorized,
            473 => ResultCode::AuthenticationFailed,
            474 => ResultCode::NotReady,
            475 => ResultCode::FailOpenTransaction,
            476 => ResultCode::FailCommit,
            477 => ResultCode::FailStore,
            500 => ResultCode::InternalServerError,
            501 => ResultCode::NotImplemented,
            503 => ResultCode::ServiceUnavailable,
            904 => ResultCode::InvalidIdentifier,
            999 => ResultCode::DatabaseModifiedError,
            1021 => ResultCode::DiskFull,
            1022 => ResultCode::DuplicateKey,
            1118 => ResultCode::SizeTooLarge,
            4000 => ResultCode::ConnectError,
            // ...
            _ => ResultCode::Zero,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_code_values() {
        assert_eq!(ResultCode::Ok as u16, 200);
        assert_eq!(ResultCode::NotFound as u16, 404);
        assert_eq!(ResultCode::InternalServerError as u16, 500);
    }

    #[test]
    fn test_from_i64() {
        assert_eq!(ResultCode::from_i64(200), ResultCode::Ok);
        assert_eq!(ResultCode::from_i64(404), ResultCode::NotFound);
        assert_eq!(ResultCode::from_i64(500), ResultCode::InternalServerError);
        assert_eq!(ResultCode::from_i64(999999), ResultCode::Zero);
    }

    #[test]
    fn test_from_io_error() {
        let not_found_error = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        assert_eq!(ResultCode::from(not_found_error), ResultCode::NotFound);

        let permission_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "test");
        assert_eq!(ResultCode::from(permission_error), ResultCode::Forbidden);
    }

    #[test]
    fn test_opt_authorize() {
        assert_eq!(OptAuthorize::YES, OptAuthorize::YES);
        assert_ne!(OptAuthorize::YES, OptAuthorize::NO);
    }
}
