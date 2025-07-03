use nanoid::nanoid;

// Re-export from v_result_code crate
pub use v_result_code::{ResultCode, OptAuthorize};

pub fn generate_unique_uri(prefix: &str, postfix: &str) -> String {
    let alphabet: [char; 36] = [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
        't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    format!("{}{}{}", prefix, nanoid!(24, &alphabet), postfix)
}
