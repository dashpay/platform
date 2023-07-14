mod estimation_costs;
/// Genesis time module
#[cfg(feature = "full")]
pub mod genesis_time;
/// Constants for the misc tree
pub mod misc_tree_constants;
/// Protocol version module
pub mod protocol_version;

use crate::drive::RootTree;

pub(crate) fn misc_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Misc)]
}

pub(crate) fn misc_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Misc).to_vec()]
}
