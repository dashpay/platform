mod fetch_platform_state_bytes;
mod store_platform_state_bytes;

pub use fetch_execution_state_bytes::*;
pub use store_execution_state_bytes::*;

const EXECUTION_STORAGE_STATE_KEY: &[u8; 11] = b"saved_state";
