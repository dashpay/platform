//? Should the [`RootTree`] and related methods be part of DPP?

#[repr(u8)]
pub enum RootTree {
    // Input data errors
    Identities = 0,
    ContractDocuments = 1,
    PublicKeyHashesToIdentities = 2,
    Misc = 3,
}

impl From<RootTree> for u8 {
    fn from(root_tree: RootTree) -> Self {
        root_tree as u8
    }
}

impl From<RootTree> for [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        [root_tree as u8]
    }
}

impl From<RootTree> for &'static [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        match root_tree {
            RootTree::Identities => &[0],
            RootTree::ContractDocuments => &[1],
            RootTree::PublicKeyHashesToIdentities => &[2],
            RootTree::Misc => &[3],
        }
    }
}
