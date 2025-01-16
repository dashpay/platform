use crate::drive::RootTree;

/// Key for accessing token status information.
pub const TOKEN_STATUS_INFO_KEY: u8 = 96;
/// Key for accessing token identity information tree.
pub const TOKEN_IDENTITY_INFO_KEY: u8 = 160;
/// Key for accessing token balances tree.
pub const TOKEN_BALANCES_KEY: u8 = 128;

/// The path for the balances tree

pub fn tokens_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Tokens)]
}

/// The path for the balances tree

pub fn tokens_root_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Tokens).to_vec()]
}

/// The root path of token balances tree, this refers to a big sum tree

pub fn token_balances_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_BALANCES_KEY],
    ]
}

/// The root path of token balances tree, this refers to a big sum tree

pub fn token_balances_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_BALANCES_KEY]]
}

/// Returns the root path for token identity information as a fixed-size array of byte slices.

pub fn token_identity_infos_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_IDENTITY_INFO_KEY],
    ]
}

/// Returns the root path for token identity information as a vector of byte vectors.

pub fn token_identity_infos_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_IDENTITY_INFO_KEY]]
}

/// Returns the root path for token statuses as a fixed-size array of byte slices.

pub fn token_statuses_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_STATUS_INFO_KEY],
    ]
}

/// Returns the root path for token statuses as a vector of byte vectors.

pub fn token_statuses_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_STATUS_INFO_KEY]]
}

/// The path for the token balances tree

pub fn token_balances_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_BALANCES_KEY],
        token_id,
    ]
}

/// The path for the token balances tree

pub fn token_balances_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_BALANCES_KEY],
        token_id.to_vec(),
    ]
}

/// The path for the token info tree

pub fn token_identity_infos_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_IDENTITY_INFO_KEY],
        token_id,
    ]
}

/// The path for the token info tree

pub fn token_identity_infos_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_IDENTITY_INFO_KEY],
        token_id.to_vec(),
    ]
}
