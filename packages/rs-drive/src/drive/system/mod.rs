mod estimation_costs;
mod fetch_elements;
/// Genesis time module
#[cfg(feature = "server")]
pub mod genesis_time;
/// Protocol version module
pub mod protocol_version;

use crate::drive::RootTree;

/// misc path
pub fn misc_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Misc)]
}

/// misc path vector
pub fn misc_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Misc).to_vec()]
}
