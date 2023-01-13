mod estimation_costs;

use crate::drive::RootTree;

pub(crate) fn misc_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Misc)]
}

pub(crate) fn misc_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Misc).to_vec()]
}
