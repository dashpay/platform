use crate::drive::RootTree;

/// Handles operations related to adding transaction history.
mod add_transaction_history_operations;

/// Defines logic for applying status updates within the system.
pub mod apply_status;

/// Manages operations related to balance handling.
pub mod balance;

/// Implements functionality for burning tokens.
pub mod burn;

/// Computes estimated costs for various operations.
pub mod estimated_costs;

/// Manages freezing operations in the system.
pub mod freeze;

/// Identity token info module, like if someone is frozen
pub mod info;

/// Implements minting operations for creating new tokens.
pub mod mint;

/// Manages system-level operations and utilities.
pub mod system;

/// Handles transfer operations, including token movement.
pub mod transfer;

/// Manages unfreezing operations within the system.
pub mod unfreeze;

/// Calculates the total token balance across all accounts.
pub mod calculate_total_tokens_balance;

/// Token status module, like if the token is paused
pub mod status;

/// Key for accessing token status information.
pub const TOKEN_STATUS_INFO_KEY: u8 = 96;

/// Key for accessing token identity information tree.
pub const TOKEN_IDENTITY_INFO_KEY: u8 = 160;

/// Key for accessing token balances tree.
pub const TOKEN_BALANCES_KEY: u8 = 128;

/// The path for the balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub fn tokens_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Tokens)]
}

/// The path for the balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub fn tokens_root_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Tokens).to_vec()]
}

/// The root path of token balances tree, this refers to a big sum tree
#[cfg(any(feature = "server", feature = "verify"))]
pub fn token_balances_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_BALANCES_KEY],
    ]
}

/// The root path of token balances tree, this refers to a big sum tree
#[cfg(any(feature = "server", feature = "verify"))]
pub fn token_balances_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_BALANCES_KEY]]
}
/// Returns the root path for token identity information as a fixed-size array of byte slices.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn token_identity_infos_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_IDENTITY_INFO_KEY],
    ]
}

/// Returns the root path for token identity information as a vector of byte vectors.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn token_identity_infos_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_IDENTITY_INFO_KEY]]
}

/// Returns the root path for token statuses as a fixed-size array of byte slices.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn token_statuses_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_STATUS_INFO_KEY],
    ]
}

/// Returns the root path for token statuses as a vector of byte vectors.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn token_statuses_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_STATUS_INFO_KEY]]
}

/// The path for the token balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub fn token_balances_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_BALANCES_KEY],
        token_id,
    ]
}

/// The path for the token balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub fn token_balances_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_BALANCES_KEY],
        token_id.to_vec(),
    ]
}

/// The path for the token info tree
#[cfg(any(feature = "server", feature = "verify"))]
pub fn token_identity_infos_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_IDENTITY_INFO_KEY],
        token_id,
    ]
}

/// The path for the token info tree
#[cfg(any(feature = "server", feature = "verify"))]
pub fn token_identity_infos_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_IDENTITY_INFO_KEY],
        token_id.to_vec(),
    ]
}
