mod fetch_execution_state;
mod store_execution_state;

pub use fetch_execution_state::*;
pub use store_execution_state::*;

const EXECUTION_STORAGE_STATE_KEY: &[u8; 11] = b"saved_state";
