//! Per-token shielded pools.
//!
//! A token whose configuration has `has_shielded_pool` owns an Orchard pool at
//! `[Tokens, TOKEN_SHIELDED_POOLS_KEY, token_id]`, laid out exactly like the credit shielded
//! pool: a note commitment tree, a nullifier set, the recorded anchors (by anchor and by height)
//! and a total balance SumItem. The pool SumTrees hang under one BigSumTree so the amount of every
//! token currently shielded is one sum, which the token conservation check adds to the identity
//! balances.
//!
//! The primitives (note / nullifier insertion, balance update, anchor bookkeeping, membership
//! reads) are the credit pool's, re-rooted under the token: see `crate::drive::shielded`. This
//! module holds what is token-specific: pool creation, cost estimation, and the three composite
//! operations (`shield`, `unshield`, `shielded_transfer`) the batch token transitions lower to.

mod create_pool_trees;
mod estimated_costs;
mod insert_root_tree;
mod shield;
mod shielded_transfer;
mod unshield;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// How a pool operation moves the pool's total balance.
#[derive(Debug, Clone, Copy)]
pub(in crate::drive::tokens) enum TokenPoolBalanceChange {
    /// Tokens enter the pool (shield).
    Add(TokenAmount),
    /// Tokens leave the pool (unshield).
    Remove(TokenAmount),
    /// The balance is unchanged (shielded transfer).
    Unchanged,
}

impl Drive {
    /// The operations every token pool write shares: nullifier insertion, note appends and the
    /// total balance update, with the pool's cost estimation registered when estimating.
    ///
    /// The balance is read from state only when applying; an estimate assumes a fresh pool, the
    /// worst case for the write.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::drive::tokens) fn token_shielded_pool_update_operations(
        &self,
        token_id: [u8; 32],
        balance_change: TokenPoolBalanceChange,
        nullifiers: &[[u8; 32]],
        notes: &[ShieldedActionNote],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_token_shielded_pool_operations(
                token_id,
                estimated_costs_only_with_layer_info,
            );
        }

        if !nullifiers.is_empty() {
            drive_operations.extend(Self::insert_token_pool_nullifiers(
                token_id,
                nullifiers,
                platform_version,
            )?);
        }

        for note in notes {
            drive_operations.extend(Self::insert_token_pool_note_op(
                token_id,
                note.nullifier,
                note.cmx,
                note.cv_net,
                note.encrypted_note.clone(),
                platform_version,
            )?);
        }

        let apply = estimated_costs_only_with_layer_info.is_none();

        let new_total_balance = match balance_change {
            TokenPoolBalanceChange::Unchanged => None,
            TokenPoolBalanceChange::Add(amount) => {
                let current = if apply {
                    self.read_token_shielded_pool_total_balance(
                        &token_id,
                        transaction,
                        &mut drive_operations,
                        platform_version,
                    )?
                } else {
                    0
                };
                Some(current.checked_add(amount).ok_or_else(|| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "token shielded pool total balance overflow when shielding".to_string(),
                    ))
                })?)
            }
            TokenPoolBalanceChange::Remove(amount) => {
                let current = if apply {
                    self.read_token_shielded_pool_total_balance(
                        &token_id,
                        transaction,
                        &mut drive_operations,
                        platform_version,
                    )?
                } else {
                    amount
                };
                Some(current.checked_sub(amount).ok_or_else(|| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "token shielded pool total balance would go negative when unshielding"
                            .to_string(),
                    ))
                })?)
            }
        };

        if let Some(new_total_balance) = new_total_balance {
            drive_operations.extend(Self::update_token_pool_total_balance_op(
                token_id,
                new_total_balance,
                platform_version,
            )?);
        }

        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::Identifier;

    const TOKEN_ID: [u8; 32] = [7u8; 32];

    fn note(tag: u8) -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [tag; 32],
            cmx: [tag.wrapping_add(1); 32],
            cv_net: [tag.wrapping_add(2); 32],
            encrypted_note: vec![tag; 216],
        }
    }

    /// A drive with an identity holding `balance` of the token and an empty pool for it.
    fn setup(balance: TokenAmount) -> (Drive, [u8; 32]) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();

        let identity =
            Identity::random_identity(3, Some(1), platform_version).expect("random identity");
        drive
            .add_new_identity(
                identity.clone(),
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("insert identity");
        drive
            .create_token_trees(
                Identifier::from([5u8; 32]),
                0,
                TOKEN_ID,
                false,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("create token trees");
        drive
            .add_to_identity_token_balance(
                TOKEN_ID,
                identity.id().to_buffer(),
                balance,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("add token balance");

        let operations = drive
            .create_token_shielded_pool_trees_operations(
                TOKEN_ID,
                false,
                &mut None,
                None,
                platform_version,
            )
            .expect("pool tree operations");
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("create pool trees");

        (drive, identity.id().to_buffer())
    }

    fn apply(drive: &Drive, operations: Vec<LowLevelDriveOperation>) {
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("apply operations");
    }

    fn pool_balance(drive: &Drive) -> TokenAmount {
        drive
            .read_token_shielded_pool_total_balance(
                &TOKEN_ID,
                None,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("pool balance")
    }

    fn notes_count(drive: &Drive) -> u64 {
        drive
            .token_shielded_pool_notes_count(
                &TOKEN_ID,
                None,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("notes count")
    }

    fn token_balance(drive: &Drive, identity_id: [u8; 32]) -> Option<TokenAmount> {
        drive
            .fetch_identity_token_balance(TOKEN_ID, identity_id, None, PlatformVersion::latest())
            .expect("token balance")
    }

    fn nullifier_is_spent(drive: &Drive, nullifier: &[u8; 32]) -> bool {
        drive
            .has_token_pool_nullifier(
                &TOKEN_ID,
                nullifier,
                None,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("nullifier lookup")
    }

    #[test]
    fn should_create_an_empty_pool_once() {
        let (drive, _) = setup(1_000);
        let platform_version = PlatformVersion::latest();

        assert_eq!(pool_balance(&drive), 0);
        assert_eq!(notes_count(&drive), 0);

        // Creating the same pool again is only allowed when the caller opts into it.
        let operations = drive
            .create_token_shielded_pool_trees_operations(
                TOKEN_ID,
                true,
                &mut None,
                None,
                platform_version,
            )
            .expect("idempotent pool tree operations");
        apply(&drive, operations);
        assert_eq!(pool_balance(&drive), 0);

        // Without the opt-in, an existing pool is a corrupted-state error at operation time.
        assert!(
            matches!(
                drive.create_token_shielded_pool_trees_operations(
                    TOKEN_ID,
                    false,
                    &mut None,
                    None,
                    platform_version,
                ),
                Err(Error::Drive(DriveError::CorruptedDriveState(_)))
            ),
            "recreating an existing pool must fail"
        );
    }

    #[test]
    fn should_shield_from_the_identity_into_the_pool() {
        let (drive, identity_id) = setup(1_000);
        let platform_version = PlatformVersion::latest();
        let notes = [note(1), note(2)];

        let operations = drive
            .token_shield_operations(
                TOKEN_ID,
                identity_id,
                400,
                &notes,
                &mut None,
                None,
                platform_version,
            )
            .expect("shield operations");
        apply(&drive, operations);

        assert_eq!(token_balance(&drive, identity_id), Some(600));
        assert_eq!(pool_balance(&drive), 400);
        assert_eq!(notes_count(&drive), 2);
        // Outputs-only bundles never spend: their dummy nullifiers are not recorded.
        assert!(!nullifier_is_spent(&drive, &notes[0].nullifier));
    }

    #[test]
    fn should_reject_shielding_more_than_the_identity_holds() {
        let (drive, identity_id) = setup(100);
        let platform_version = PlatformVersion::latest();

        let result = drive.token_shield_operations(
            TOKEN_ID,
            identity_id,
            101,
            &[note(1)],
            &mut None,
            None,
            platform_version,
        );
        assert!(result.is_err());
        assert_eq!(pool_balance(&drive), 0);
    }

    #[test]
    fn should_unshield_from_the_pool_to_the_recipient() {
        let (drive, identity_id) = setup(1_000);
        let platform_version = PlatformVersion::latest();
        let recipient_id = [9u8; 32];

        let operations = drive
            .token_shield_operations(
                TOKEN_ID,
                identity_id,
                400,
                &[note(1), note(2)],
                &mut None,
                None,
                platform_version,
            )
            .expect("shield operations");
        apply(&drive, operations);

        let nullifiers = [[11u8; 32], [12u8; 32]];
        let change_notes = [note(3), note(4)];
        let operations = drive
            .token_unshield_operations(
                TOKEN_ID,
                recipient_id,
                150,
                &nullifiers,
                &change_notes,
                &mut None,
                None,
                platform_version,
            )
            .expect("unshield operations");
        apply(&drive, operations);

        assert_eq!(token_balance(&drive, recipient_id), Some(150));
        assert_eq!(token_balance(&drive, identity_id), Some(600));
        assert_eq!(pool_balance(&drive), 250);
        assert_eq!(notes_count(&drive), 4);
        assert!(nullifier_is_spent(&drive, &nullifiers[0]));
        assert!(nullifier_is_spent(&drive, &nullifiers[1]));
        assert!(!nullifier_is_spent(&drive, &[13u8; 32]));
    }

    #[test]
    fn should_reject_unshielding_more_than_the_pool_holds() {
        let (drive, identity_id) = setup(1_000);
        let platform_version = PlatformVersion::latest();

        let operations = drive
            .token_shield_operations(
                TOKEN_ID,
                identity_id,
                100,
                &[note(1)],
                &mut None,
                None,
                platform_version,
            )
            .expect("shield operations");
        apply(&drive, operations);

        let result = drive.token_unshield_operations(
            TOKEN_ID,
            [9u8; 32],
            101,
            &[[11u8; 32]],
            &[note(2)],
            &mut None,
            None,
            platform_version,
        );
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
        assert_eq!(pool_balance(&drive), 100);
    }

    #[test]
    fn should_keep_the_pool_balance_on_a_shielded_transfer() {
        let (drive, identity_id) = setup(1_000);
        let platform_version = PlatformVersion::latest();

        let operations = drive
            .token_shield_operations(
                TOKEN_ID,
                identity_id,
                400,
                &[note(1), note(2)],
                &mut None,
                None,
                platform_version,
            )
            .expect("shield operations");
        apply(&drive, operations);

        let nullifiers = [[21u8; 32], [22u8; 32]];
        let operations = drive
            .token_shielded_transfer_operations(
                TOKEN_ID,
                &nullifiers,
                &[note(5), note(6)],
                &mut None,
                None,
                platform_version,
            )
            .expect("shielded transfer operations");
        apply(&drive, operations);

        assert_eq!(pool_balance(&drive), 400);
        assert_eq!(notes_count(&drive), 4);
        assert_eq!(token_balance(&drive, identity_id), Some(600));
        assert!(nullifier_is_spent(&drive, &nullifiers[0]));
        assert!(nullifier_is_spent(&drive, &nullifiers[1]));
    }

    #[test]
    fn should_estimate_without_touching_state() {
        let (drive, identity_id) = setup(1_000);
        let platform_version = PlatformVersion::latest();

        let mut layer_info = Some(HashMap::new());
        let operations = drive
            .token_unshield_operations(
                TOKEN_ID,
                [9u8; 32],
                5_000,
                &[[11u8; 32]],
                &[note(2)],
                &mut layer_info,
                None,
                platform_version,
            )
            .expect("estimated unshield operations");
        assert!(!operations.is_empty());
        assert!(
            !layer_info.expect("layer info").is_empty(),
            "estimation must register the pool layers"
        );
        assert_eq!(pool_balance(&drive), 0);
        assert_eq!(token_balance(&drive, identity_id), Some(1_000));
    }

    #[test]
    fn should_record_and_prune_pool_anchors() {
        let (drive, identity_id) = setup(1_000);
        let platform_version = PlatformVersion::latest();

        let operations = drive
            .token_shield_operations(
                TOKEN_ID,
                identity_id,
                100,
                &[note(1), note(2)],
                &mut None,
                None,
                platform_version,
            )
            .expect("shield operations");
        apply(&drive, operations);

        let transaction = drive.grove.start_transaction();
        drive
            .record_token_shielded_pool_anchor_if_changed(
                TOKEN_ID,
                1,
                &transaction,
                platform_version,
            )
            .expect("record anchor");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");

        let first_anchor = drive
            .read_latest_recorded_token_shielded_pool_anchor(TOKEN_ID, None, platform_version)
            .expect("read anchor")
            .expect("an anchor is recorded after the first write");
        assert!(drive
            .has_token_pool_anchor(
                &TOKEN_ID,
                &first_anchor,
                None,
                &mut vec![],
                platform_version
            )
            .expect("anchor lookup"));

        // Another write changes the tree and therefore the anchor.
        let operations = drive
            .token_shielded_transfer_operations(
                TOKEN_ID,
                &[[21u8; 32]],
                &[note(5)],
                &mut None,
                None,
                platform_version,
            )
            .expect("shielded transfer operations");
        apply(&drive, operations);

        let transaction = drive.grove.start_transaction();
        drive
            .record_token_shielded_pool_anchor_if_changed(
                TOKEN_ID,
                2,
                &transaction,
                platform_version,
            )
            .expect("record anchor");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");

        let second_anchor = drive
            .read_latest_recorded_token_shielded_pool_anchor(TOKEN_ID, None, platform_version)
            .expect("read anchor")
            .expect("an anchor is recorded");
        assert_ne!(first_anchor, second_anchor);

        // Pruning below height 2 drops the first anchor and keeps the newest one.
        let transaction = drive.grove.start_transaction();
        drive
            .prune_token_shielded_pool_anchors(TOKEN_ID, 2, &transaction, platform_version)
            .expect("prune anchors");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");

        assert!(!drive
            .has_token_pool_anchor(
                &TOKEN_ID,
                &first_anchor,
                None,
                &mut vec![],
                platform_version
            )
            .expect("anchor lookup"));
        assert!(drive
            .has_token_pool_anchor(
                &TOKEN_ID,
                &second_anchor,
                None,
                &mut vec![],
                platform_version
            )
            .expect("anchor lookup"));

        // The newest anchor survives even when the cutoff is past its height.
        let transaction = drive.grove.start_transaction();
        drive
            .prune_token_shielded_pool_anchors(TOKEN_ID, 100, &transaction, platform_version)
            .expect("prune anchors");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");
        assert!(drive
            .has_token_pool_anchor(
                &TOKEN_ID,
                &second_anchor,
                None,
                &mut vec![],
                platform_version
            )
            .expect("anchor lookup"));
    }
}
