//! The remaining budget of budgeted identity keys.
//!
//! A public key may carry a `total_budget`: the total credits that state transitions signed with it may
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
mod fetch_identity_keys_remaining_budgets;
mod insert_identity_key_budget;
mod prove_identity_keys_remaining_budgets;

pub(crate) use crate::drive::identity::identity_key_budgets_path;
pub use crate::drive::identity::identity_key_budgets_path_vec;

/// The size of an encoded remaining budget
pub(crate) const KEY_BUDGET_SIZE: u32 = 8;

/// The most bytes a key id takes as the key of a budget entry: a `u32` as a varint
pub(crate) const KEY_ID_MAX_ENCODED_SIZE: u8 = 5;

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

    mod remaining_budgets_query {
        use super::*;
        use std::collections::BTreeMap;

        fn prove_and_verify(
            drive: &Drive,
            identity_id: [u8; 32],
            key_ids: &[KeyID],
        ) -> BTreeMap<KeyID, Option<Credits>> {
            let platform_version = PlatformVersion::latest();
            let proof = drive
                .prove_identity_keys_remaining_budgets(identity_id, key_ids, None, platform_version)
                .expect("expected a proof");
            let (root_hash, proved): (_, BTreeMap<KeyID, Option<Credits>>) =
                Drive::verify_identity_keys_remaining_budgets(
                    proof.as_slice(),
                    identity_id,
                    key_ids,
                    false,
                    platform_version,
                )
                .expect("expected the proof to verify");
            assert_eq!(
                root_hash,
                drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .expect("expected a root hash"),
                "the proof must commit to the current state"
            );

            // What the node answers without a proof must be what the proof says.
            let fetched = drive
                .fetch_identity_keys_remaining_budgets(identity_id, key_ids, None, platform_version)
                .expect("expected to fetch");
            assert_eq!(fetched, proved);
            proved
        }

        #[test]
        fn should_prove_budgeted_keys_next_to_keys_without_a_budget() {
            let platform_version = PlatformVersion::latest();
            let (drive, identity_id) = setup_identity(platform_version);
            drive
                .add_new_unique_keys_to_identity(
                    identity_id,
                    vec![
                        budgeted_key(BUDGETED_KEY_ID, 15, Some(1_000)),
                        budgeted_key(BUDGETED_KEY_ID + 1, 16, None),
                        budgeted_key(300, 17, Some(77)),
                    ],
                    &block(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add keys");
            drive
                .deduct_from_identity_key_budget(
                    identity_id,
                    BUDGETED_KEY_ID,
                    400,
                    None,
                    platform_version,
                )
                .expect("expected to deduct");

            // A budgeted key, an ordinary key of the identity, a key that only has no budget, a
            // key id that takes two varint bytes, and a key id the identity does not have.
            let proved = prove_and_verify(
                &drive,
                identity_id,
                &[BUDGETED_KEY_ID, 0, BUDGETED_KEY_ID + 1, 300, 9_999],
            );
            assert_eq!(
                proved,
                BTreeMap::from([
                    (0, None),
                    (BUDGETED_KEY_ID, Some(600)),
                    (BUDGETED_KEY_ID + 1, None),
                    (300, Some(77)),
                    (9_999, None),
                ])
            );
        }

        #[test]
        fn should_prove_the_absence_of_a_budget_when_the_identity_has_no_budgeted_key() {
            // The key budgets subtree itself does not exist for such an identity.
            let platform_version = PlatformVersion::latest();
            let (drive, identity_id) = setup_identity(platform_version);
            let proved = prove_and_verify(&drive, identity_id, &[0, 1]);
            assert_eq!(proved, BTreeMap::from([(0, None), (1, None)]));
        }

        #[test]
        fn should_prove_the_absence_of_a_budget_for_an_identity_that_does_not_exist() {
            let platform_version = PlatformVersion::latest();
            let (drive, _) = setup_identity(platform_version);
            let proved = prove_and_verify(&drive, [9u8; 32], &[0]);
            assert_eq!(proved, BTreeMap::from([(0, None)]));
        }

        #[test]
        fn should_never_verify_to_an_answer_that_is_not_in_state() {
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
                .expect("expected to add the key");
            let proof = drive
                .prove_identity_keys_remaining_budgets(
                    identity_id,
                    &[BUDGETED_KEY_ID],
                    None,
                    platform_version,
                )
                .expect("expected a proof");
            let state_root = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("expected a root hash");

            // Replayed against another query, a proof may still verify: a one entry subtree is
            // revealed whole, so it truthfully shows that its neighbour is absent. What it must
            // never do is answer with a budget that key does not have.
            for (claimed_identity, claimed_key) in [
                (identity_id, BUDGETED_KEY_ID + 1),
                ([9u8; 32], BUDGETED_KEY_ID),
            ] {
                let result: Result<(_, BTreeMap<KeyID, Option<Credits>>), _> =
                    Drive::verify_identity_keys_remaining_budgets(
                        proof.as_slice(),
                        claimed_identity,
                        &[claimed_key],
                        false,
                        platform_version,
                    );
                if let Ok((root_hash, proved)) = result {
                    assert_eq!(root_hash, state_root);
                    assert_eq!(proved, BTreeMap::from([(claimed_key, None)]));
                }
            }

            // A proof whose stored value was altered no longer commits to the state.
            let needle = 1_000u64.to_be_bytes();
            let position = proof
                .windows(needle.len())
                .position(|window| window == needle)
                .expect("expected the remaining budget in the proof");
            let mut forged = proof.clone();
            forged[position..position + needle.len()].copy_from_slice(&999_999u64.to_be_bytes());
            let result: Result<(_, BTreeMap<KeyID, Option<Credits>>), _> =
                Drive::verify_identity_keys_remaining_budgets(
                    forged.as_slice(),
                    identity_id,
                    &[BUDGETED_KEY_ID],
                    false,
                    platform_version,
                );
            assert!(
                result.map_or(true, |(root_hash, _)| root_hash != state_root),
                "a forged budget must not verify against the state root"
            );
        }
    }
}
