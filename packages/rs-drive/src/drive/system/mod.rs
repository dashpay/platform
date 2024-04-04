mod estimation_costs;
/// Genesis time module
#[cfg(feature = "server")]
pub mod genesis_time;
/// Protocol version module
pub mod protocol_version;

use crate::drive::RootTree;

/// misc path
pub(crate) fn misc_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Misc)]
}

/// misc path vector
pub(crate) fn misc_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Misc).to_vec()]
}
