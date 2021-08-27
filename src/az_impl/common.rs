use crate::az_impl::az_lmdb::_f_authorize;
use crate::v_authorization::common::Trace;

pub fn f_authorize(uri: &str, user_uri: &str, request_access: u8, _is_check_for_reload: bool, trace: Option<&mut Trace>) -> Result<u8, i64> {
    _f_authorize(uri, user_uri, request_access, _is_check_for_reload, trace)
}
