use drive::drive::RootTree;

mod fetch_execution_state;
mod store_execution_state;

pub use fetch_execution_state::*;
pub use store_execution_state::*;

const EXECUTION_STORAGE_PATH: [[u8; 1]; 1] = [[RootTree::Misc as u8]];
const EXECUTION_STORAGE_STATE_KEY: &[u8; 1] = b"S";
