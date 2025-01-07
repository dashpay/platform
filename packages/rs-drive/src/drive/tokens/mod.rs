use crate::drive::RootTree;

mod add_transaction_history_operations;
pub mod apply_status;
pub mod balance;
pub mod burn;
pub mod estimated_costs;
pub mod freeze;
mod info;
pub mod mint;
pub mod system;
pub mod transfer;
pub mod unfreeze;

pub const TOKEN_STATUS_INFO_KEY: u8 = 96;
pub const TOKEN_IDENTITY_INFO_KEY: u8 = 64;
pub const TOKEN_BALANCES_KEY: u8 = 128;

/// The path for the balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn tokens_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Tokens)]
}

/// The path for the balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn tokens_root_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Tokens).to_vec()]
}

/// The path for the token tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn token_path(token_id: &[u8; 32]) -> [&[u8]; 2] {
    [Into::<&[u8; 1]>::into(RootTree::Tokens), token_id]
}

/// The path for the token tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn token_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], token_id.to_vec()]
}

/// The path for the token balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn token_balances_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        token_id,
        &[TOKEN_BALANCES_KEY],
    ]
}

/// The path for the token balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn token_balances_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        token_id.to_vec(),
        vec![TOKEN_BALANCES_KEY],
    ]
}

/// The path for the token info tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn token_identity_infos_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        token_id,
        &[TOKEN_IDENTITY_INFO_KEY],
    ]
}

/// The path for the token info tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn token_identity_infos_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        token_id.to_vec(),
        vec![TOKEN_IDENTITY_INFO_KEY],
    ]
}
