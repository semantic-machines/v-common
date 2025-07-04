#[cfg(feature = "tt_2")]
mod inner {
    pub use rusty_tarantool_2 as rusty_tarantool;
}

#[cfg(feature = "tt_3")]
mod inner {
    pub use rusty_tarantool_3 as rusty_tarantool;
}

pub use inner::rusty_tarantool::tarantool::{Client, ClientConfig, IteratorType};
