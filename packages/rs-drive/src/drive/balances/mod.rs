//! This module defines functions within the Drive struct related to balances.
//! Functions include inserting verifying balances between various trees.
//!

#[cfg(feature = "full")]
mod add_to_system_credits;
#[cfg(feature = "full")]
pub use add_to_system_credits::*;

#[cfg(feature = "full")]
mod add_to_system_credits_operations;
#[cfg(feature = "full")]
pub use add_to_system_credits_operations::*;

#[cfg(feature = "full")]
mod remove_from_system_credits;
#[cfg(feature = "full")]
pub use remove_from_system_credits::*;

#[cfg(feature = "full")]
mod remove_from_system_credits_operations;
#[cfg(feature = "full")]
pub use remove_from_system_credits_operations::*;

#[cfg(feature = "full")]
mod calculate_total_credits_balance;
#[cfg(feature = "full")]
pub use calculate_total_credits_balance::*;

#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::RootTree;

/// Storage fee pool key
#[cfg(feature = "full")]
pub const TOTAL_SYSTEM_CREDITS_STORAGE_KEY: &[u8; 1] = b"D";

/// The path for all the credits in the system
#[cfg(feature = "full")]
pub fn total_credits_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Misc),
        TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
    ]
}

/// The path for the balances tree
#[cfg(any(feature = "full", feature = "verify"))]
pub(crate) fn balance_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Balances)]
}

/// The path for the balances tree
#[cfg(any(feature = "full", feature = "verify"))]
pub(crate) fn balance_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Balances).to_vec()]
}

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use crate::drive::Drive;

    use dpp::version::PlatformVersion;
    use tempfile::TempDir;

    #[test]
    fn verify_total_credits_structure() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");
        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::first();
        drive
            .create_initial_state_structure(Some(&db_transaction), platform_version)
            .expect("expected to create root tree successfully");

        let credits_match_expected = drive
            .calculate_total_credits_balance(Some(&db_transaction), &platform_version.drive)
            .expect("expected to get the result of the verification");
        assert!(credits_match_expected.ok().expect("no overflow"));
    }
}
