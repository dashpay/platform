use drive::drive::RootTree;

mod fetch_execution_state;
mod store_execution_state;

pub use fetch_execution_state::*;
pub(in crate::execution) use store_execution_state::*;

const STORAGE_PATH: [[u8; 1]; 1] = [Into::<[u8; 1]>::into(RootTree::Misc)];
const STORAGE_KEY: &[u8; 1] = b"E";
