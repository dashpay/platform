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

/// Storage fee pool key
#[cfg(any(feature = "server", feature = "verify"))]
pub const TOTAL_SYSTEM_CREDITS_STORAGE_KEY: &[u8; 1] = b"D";

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

    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[test]
    fn verify_total_credits_structure() {
        let drive: Drive = setup_drive_with_initial_state_structure();
        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let credits_match_expected = drive
            .calculate_total_credits_balance(Some(&db_transaction), &platform_version.drive)
            .expect("expected to get the result of the verification");
        assert!(credits_match_expected.ok().expect("no overflow"));
    }
}
