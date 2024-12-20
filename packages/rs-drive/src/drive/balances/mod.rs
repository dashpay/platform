//! This module defines functions within the Drive struct related to balances.
//! Functions include inserting verifying balances between various trees.
//!

#[cfg(feature = "server")]
mod add_to_system_credits;

#[cfg(feature = "server")]
mod add_to_system_credits_operations;

#[cfg(feature = "server")]
mod remove_from_system_credits;

#[cfg(feature = "server")]
mod remove_from_system_credits_operations;

#[cfg(feature = "server")]
mod calculate_total_credits_balance;

#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::RootTree;
use crate::query::Query;
use grovedb::{PathQuery, SizedQuery};

/// Total system credits storage
#[cfg(any(feature = "server", feature = "verify"))]
pub const TOTAL_SYSTEM_CREDITS_STORAGE_KEY: &[u8; 1] = b"D";

/// Total token supplies storage
#[cfg(any(feature = "server", feature = "verify"))]
pub const TOTAL_TOKEN_SUPPLIES_STORAGE_KEY: &[u8; 1] = b"T";

/// The path for all the credits in the system
#[cfg(any(feature = "server", feature = "verify"))]
pub fn total_credits_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Misc),
        TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
    ]
}

/// The path as a vec for all the credits in the system
#[cfg(any(feature = "server", feature = "verify"))]
pub fn total_credits_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Misc as u8],
        TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec(),
    ]
}

/// A path query helper to get the total credits on Platform
#[cfg(any(feature = "server", feature = "verify"))]
pub fn total_credits_on_platform_path_query() -> PathQuery {
    PathQuery {
        path: vec![vec![RootTree::Misc as u8]],
        query: SizedQuery {
            query: Query::new_single_key(TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec()),
            limit: Some(1),
            offset: None,
        },
    }
}

/// The path for the root of all token supplies
#[cfg(any(feature = "server", feature = "verify"))]
pub fn total_tokens_root_supply_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Misc),
        TOTAL_TOKEN_SUPPLIES_STORAGE_KEY,
    ]
}

/// The path as a vec for the root of all token supplies
#[cfg(any(feature = "server", feature = "verify"))]
pub fn total_tokens_root_supply_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Misc as u8],
        TOTAL_TOKEN_SUPPLIES_STORAGE_KEY.to_vec(),
    ]
}

/// The path for the token supply for a given token
#[cfg(any(feature = "server", feature = "verify"))]
pub fn total_token_supply_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Misc),
        TOTAL_TOKEN_SUPPLIES_STORAGE_KEY,
        token_id,
    ]
}

/// The path as a vec for the token supply for a given token
#[cfg(any(feature = "server", feature = "verify"))]
pub fn total_token_supply_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Misc as u8],
        TOTAL_TOKEN_SUPPLIES_STORAGE_KEY.to_vec(),
        token_id.to_vec(),
    ]
}

/// A path query helper to get the total token supply for a given token on Platform
#[cfg(any(feature = "server", feature = "verify"))]
pub fn total_supply_for_token_on_platform_path_query(token_id: [u8; 32]) -> PathQuery {
    PathQuery {
        path: vec![
            vec![RootTree::Misc as u8],
            TOTAL_TOKEN_SUPPLIES_STORAGE_KEY.to_vec(),
        ],
        query: SizedQuery {
            query: Query::new_single_key(token_id.to_vec()),
            limit: Some(1),
            offset: None,
        },
    }
}

/// The path for the balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn balance_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Balances)]
}

/// The path for the balances tree
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn balance_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Balances).to_vec()]
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::Drive;

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[test]
    fn verify_total_credits_structure() {
        let drive: Drive = setup_drive_with_initial_state_structure(None);
        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let credits_match_expected = drive
            .calculate_total_credits_balance(Some(&db_transaction), &platform_version.drive)
            .expect("expected to get the result of the verification");
        assert!(credits_match_expected.ok().expect("no overflow"));
    }
}
