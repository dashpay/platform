//! The remaining budget of budgeted identity keys.
//!
//! A public key may carry a `budget`: the total credits that state transitions signed with it may
//! take from the identity. The key itself is immutable, so what is left of the budget lives next
//! to it, in the identity's key budgets subtree:
//!
//! `Identities / <identity id> / IdentityTreeKeyBudgets / <key id> -> remaining credits`
//!
//! The subtree is created the first time an identity is given a budgeted key. The value is a
//! fixed 8 byte big endian integer so that deducting from it never changes what is stored, only
//! replaces it.

mod add_estimation_costs_for_key_budgets;
mod deduct_from_identity_key_budget;
mod fetch_identity_key_remaining_budget;
mod insert_identity_key_budget;

use crate::drive::identity::IdentityRootStructure;
use crate::drive::RootTree;

/// The size of an encoded remaining budget
pub(crate) const KEY_BUDGET_SIZE: u32 = 8;

/// The path to the key budgets subtree of an identity
pub(crate) fn identity_key_budgets_path(identity_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Identities),
        identity_id,
        Into::<&[u8; 1]>::into(IdentityRootStructure::IdentityTreeKeyBudgets),
    ]
}

/// The path to the key budgets subtree of an identity as a vec
pub fn identity_key_budgets_path_vec(identity_id: &[u8]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Identities as u8],
        identity_id.to_vec(),
        vec![IdentityRootStructure::IdentityTreeKeyBudgets as u8],
    ]
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::fee::Credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyID};
    use dpp::version::PlatformVersion;

    const BUDGETED_KEY_ID: KeyID = 5;

    fn block() -> BlockInfo {
        BlockInfo::default_with_epoch(Epoch::new(0).expect("expected epoch 0"))
    }

    fn budgeted_key(key_id: KeyID, seed: u64, budget: Option<Credits>) -> IdentityPublicKey {
        let platform_version = PlatformVersion::latest();
        IdentityPublicKey::random_authentication_keys(key_id, 1, Some(seed), platform_version)
            .remove(0)
            .with_limits(budget, None)
    }

    /// A drive holding an identity with five ordinary keys, ids 0 to 4.
    fn setup_identity(platform_version: &PlatformVersion) -> (Drive, [u8; 32]) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert identity");
        (drive, identity.id().to_buffer())
    }

    #[test]
    fn should_start_a_budgeted_key_with_its_whole_budget() {
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup_identity(platform_version);

        drive
            .add_new_unique_keys_to_identity(
                identity_id,
                vec![budgeted_key(BUDGETED_KEY_ID, 15, Some(1_000_000))],
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the budgeted key");

        assert_eq!(
            drive
                .fetch_identity_key_remaining_budget(
                    identity_id,
                    BUDGETED_KEY_ID,
                    None,
                    platform_version
                )
                .expect("expected to fetch"),
            Some(1_000_000)
        );
    }

    #[test]
    fn should_have_no_remaining_budget_for_a_key_without_one() {
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup_identity(platform_version);

        // The identity has never been given a budgeted key: the subtree itself is missing.
        assert_eq!(
            drive
                .fetch_identity_key_remaining_budget(identity_id, 0, None, platform_version)
                .expect("expected to fetch"),
            None
        );

        // Once the subtree exists, a key with only an expiry still has no entry in it.
        drive
            .add_new_unique_keys_to_identity(
                identity_id,
                vec![
                    budgeted_key(BUDGETED_KEY_ID, 15, Some(7)),
                    budgeted_key(BUDGETED_KEY_ID + 1, 16, None),
                ],
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add keys");
        assert_eq!(
            drive
                .fetch_identity_key_remaining_budget(
                    identity_id,
                    BUDGETED_KEY_ID + 1,
                    None,
                    platform_version
                )
                .expect("expected to fetch"),
            None
        );
    }

    #[test]
    fn should_keep_separate_budgets_for_keys_added_together() {
        // Both keys arrive in one batch, so the second insert sees the subtree only as a
        // pending operation of the first.
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup_identity(platform_version);

        drive
            .add_new_unique_keys_to_identity(
                identity_id,
                vec![
                    budgeted_key(BUDGETED_KEY_ID, 15, Some(100)),
                    budgeted_key(BUDGETED_KEY_ID + 1, 16, Some(200)),
                ],
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add both budgeted keys");

        drive
            .deduct_from_identity_key_budget(
                identity_id,
                BUDGETED_KEY_ID,
                30,
                None,
                platform_version,
            )
            .expect("expected to deduct");

        let remaining = |key_id| {
            drive
                .fetch_identity_key_remaining_budget(identity_id, key_id, None, platform_version)
                .expect("expected to fetch")
        };
        assert_eq!(remaining(BUDGETED_KEY_ID), Some(70));
        assert_eq!(remaining(BUDGETED_KEY_ID + 1), Some(200));
    }

    #[test]
    fn should_write_the_budget_of_a_key_a_new_identity_is_created_with() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let mut identity = Identity::random_identity(3, Some(777), platform_version)
            .expect("expected a random identity");
        let key = budgeted_key(3, 18, Some(55_000));
        identity.add_public_key(key);

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert identity");

        assert_eq!(
            drive
                .fetch_identity_key_remaining_budget(
                    identity.id().to_buffer(),
                    3,
                    None,
                    platform_version
                )
                .expect("expected to fetch"),
            Some(55_000)
        );
    }

    #[test]
    fn should_stop_at_zero_when_more_than_the_remaining_budget_is_deducted() {
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup_identity(platform_version);
        drive
            .add_new_unique_keys_to_identity(
                identity_id,
                vec![budgeted_key(BUDGETED_KEY_ID, 15, Some(1_000))],
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the budgeted key");

        let deduct = |amount| {
            drive
                .deduct_from_identity_key_budget(
                    identity_id,
                    BUDGETED_KEY_ID,
                    amount,
                    None,
                    platform_version,
                )
                .expect("expected to deduct")
        };
        assert_eq!(deduct(400), 600);
        assert_eq!(deduct(0), 600);
        // Processing may take a key over its budget: what remains is zero, not an error.
        assert_eq!(deduct(5_000), 0);
        assert_eq!(deduct(1), 0);
        assert_eq!(
            drive
                .fetch_identity_key_remaining_budget(
                    identity_id,
                    BUDGETED_KEY_ID,
                    None,
                    platform_version
                )
                .expect("expected to fetch"),
            Some(0)
        );
    }

    #[test]
    fn should_not_change_what_is_stored_when_deducting() {
        // The deduction is applied outside of the fee of the state transition, like the balance
        // change, so it must never add or remove storage.
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup_identity(platform_version);
        drive
            .add_new_unique_keys_to_identity(
                identity_id,
                vec![budgeted_key(BUDGETED_KEY_ID, 15, Some(u64::MAX))],
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the budgeted key");

        let (operations, remaining) = drive
            .deduct_from_identity_key_budget_operations(
                identity_id,
                BUDGETED_KEY_ID,
                u64::MAX - 1,
                None,
                platform_version,
            )
            .expect("expected operations");
        assert_eq!(remaining, 1);

        let mut applied_operations = vec![];
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut applied_operations,
                &platform_version.drive,
            )
            .expect("expected to apply");
        let fee = Drive::calculate_fee(
            None,
            Some(applied_operations),
            &block().epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )
        .expect("expected a fee");
        assert_eq!(fee.storage_fee, 0);
        assert!(fee.fee_refunds.0.is_empty());
    }

    #[test]
    fn should_refuse_to_deduct_from_a_key_without_a_budget() {
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup_identity(platform_version);
        let result =
            drive.deduct_from_identity_key_budget(identity_id, 0, 10, None, platform_version);
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }

    #[test]
    fn should_estimate_at_least_the_cost_of_adding_a_budgeted_key() {
        let platform_version = PlatformVersion::latest();
        let add = |apply: bool| {
            let (drive, identity_id) = setup_identity(platform_version);
            drive
                .add_new_unique_keys_to_identity(
                    identity_id,
                    vec![budgeted_key(BUDGETED_KEY_ID, 15, Some(1_000_000))],
                    &block(),
                    apply,
                    None,
                    platform_version,
                )
                .expect("expected a fee")
        };
        let actual = add(true);
        let estimated = add(false);
        assert!(
            estimated.storage_fee >= actual.storage_fee,
            "estimated storage {} is below the actual {}",
            estimated.storage_fee,
            actual.storage_fee
        );
        assert!(estimated.processing_fee >= actual.processing_fee);

        // The budget entry and, for the first budgeted key, its subtree are paid for by whoever
        // adds the key.
        let (drive, identity_id) = setup_identity(platform_version);
        let unbudgeted = drive
            .add_new_unique_keys_to_identity(
                identity_id,
                vec![budgeted_key(BUDGETED_KEY_ID, 15, None)],
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected a fee");
        assert!(actual.storage_fee > unbudgeted.storage_fee);
    }

    #[test]
    fn should_not_write_a_budget_before_protocol_version_14() {
        // The version 1 key cannot reach Drive before protocol version 14, where the transition
        // carrying it is not active. Should it ever, the historical insert stays what it was.
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        let (drive, identity_id) = setup_identity(platform_version);
        drive
            .add_new_unique_keys_to_identity(
                identity_id,
                vec![budgeted_key(BUDGETED_KEY_ID, 15, Some(1_000))],
                &block(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the key");

        let stored = drive
            .fetch_identity_key_remaining_budget(
                identity_id,
                BUDGETED_KEY_ID,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch");
        assert_eq!(stored, None);
        assert!(matches!(
            drive.fetch_identity_key_remaining_budget(
                identity_id,
                BUDGETED_KEY_ID,
                None,
                platform_version
            ),
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
    }
}
