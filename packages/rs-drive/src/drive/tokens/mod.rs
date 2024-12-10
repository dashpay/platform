use crate::drive::RootTree;

pub mod balance;
pub mod burn;
pub mod mint;
pub mod system;
pub mod transfer;

/// The path for the balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn token_balances_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::TokenBalances)]
}

/// The path for the balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn token_balances_root_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::TokenBalances).to_vec()]
}

/// The path for the balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn token_balances_path(token_id: &[u8; 32]) -> [&[u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::DataContractDocuments),
        token_id,
    ]
}

/// The path for the balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn token_balances_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![vec![RootTree::TokenBalances as u8], token_id.to_vec()]
}
