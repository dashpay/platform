mod fetch_platform_state_bytes;
mod store_platform_state_bytes;

pub use fetch_platform_state_bytes::*;
pub use store_platform_state_bytes::*;

const PLATFORM_STATE_KEY: &[u8; 11] = b"saved_state";
