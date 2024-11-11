// Select the correct types based on feature flags
#[cfg(feature = "awc_2")]
mod inner {
    pub use awc_old as awc;
}

#[cfg(feature = "awc_3")]
mod inner {
    pub use awc_new as awc;
}

// Re-export what we need through a single path
pub use inner::awc::{
    Client,
    http::header::{ACCEPT, CONTENT_TYPE, HeaderValue},
};
