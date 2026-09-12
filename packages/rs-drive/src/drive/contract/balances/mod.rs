//! Contract balances.
//!
//! Contracts are not identities and hold no identity balance. Their credits
//! live in a dedicated root sum tree, `RootTree::ContractCredits`, laid out
//! as `[ContractCredits] / contract_id (SumTree) / bucket_key (SumItem)`.
//! The root aggregate is therefore the total of live contract credits and is
//! read by credit conservation as its own term. A contract whose credits
//! were wiped keeps its subtree wrapped in a not-summed element, so the root
//! aggregate never counts retained balances.
//!
//! This module holds the path helpers. The bucket operations, the wiped
//! lifecycle wrapper and the proofs arrive with later changes.

use crate::drive::RootTree;

/// The path to the contract credits root tree.
pub fn contract_credits_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::ContractCredits)]
}

/// The path to the contract credits root tree as a vec.
pub fn contract_credits_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::ContractCredits as u8]]
}

/// The path to the credit bucket sum tree of one contract.
pub fn contract_credits_path(contract_id: &[u8; 32]) -> [&[u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractCredits),
        contract_id,
    ]
}

/// The path to the credit bucket sum tree of one contract as a vec.
pub fn contract_credits_path_vec(contract_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![vec![RootTree::ContractCredits as u8], contract_id.to_vec()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_place_the_contract_credits_root_at_the_root_key() {
        let root = contract_credits_root_path();
        assert_eq!(root, [&[RootTree::ContractCredits as u8][..]]);
        assert_eq!(
            contract_credits_root_path_vec(),
            root.iter().map(|part| part.to_vec()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn should_place_a_contract_tree_directly_under_the_root() {
        let contract_id = [7u8; 32];
        let path = contract_credits_path(&contract_id);
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[RootTree::ContractCredits as u8][..]);
        assert_eq!(path[1], &contract_id[..]);
        assert_eq!(
            contract_credits_path_vec(contract_id),
            path.iter().map(|part| part.to_vec()).collect::<Vec<_>>()
        );
    }
}
