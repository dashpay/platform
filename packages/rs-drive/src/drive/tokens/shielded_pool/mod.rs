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

mod burn_from_pool;
mod create_pool_trees;
mod estimated_costs;
mod insert_root_tree;
mod mint_to_pool;
mod shield;
mod shielded_transfer;
mod unshield;

use crate::drive::shielded::estimated_costs::TOTAL_BALANCE_VALUE_SIZE;
use crate::drive::tokens::paths::token_shielded_pools_root_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::shielded::ShieldedActionNote;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
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
    /// Whether the token owns a shielded pool subtree, without touching it. The read is
    /// collected into `drive_operations` so a caller validating a state transition can charge it.
    pub fn has_token_shielded_pool(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        self.grove_has_raw(
            (&token_shielded_pools_root_path()).into(),
            &token_id,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }

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

        // Moving the total means reading it, and fee validation quotes a batch without state
        // while execution meters the same batch against it, so a read only the apply path books
        // is charged to nobody. An estimate therefore books the read as well, as the stateless
        // query every other estimated read in Drive is priced by: it prices the read from the
        // pool's modelled layer instead of reading the value stored there. Pricing it from the
        // model rather than from the value is also what keeps the fee from depending on how much
        // of the token is shielded. A shielded transfer leaves the total alone and so reads
        // nothing in either mode.
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(TOTAL_BALANCE_VALUE_SIZE),
            }
        };

        // The total is read from state and written back as an absolute value, so two pool
        // operations on the same token in one batch would each start from the pre-batch total
        // and the second write would discard the first. Batch application refuses that:
        // `DriveOperation::refuse_repeated_token_balance_writes` reports the pool total every
        // pool operation writes and rejects a batch that writes one twice. A batch that has to
        // carry two of them needs this to become a delta on the sum item instead.
        let new_total_balance = match balance_change {
            TokenPoolBalanceChange::Unchanged => None,
            TokenPoolBalanceChange::Add(amount) => {
                // A stateless read returns 0, which is also the balance an estimate assumes: a
                // fresh pool is the worst case for the write.
                let current = self.read_token_shielded_pool_total_balance(
                    &token_id,
                    direct_query_type,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                Some(current.checked_add(amount).ok_or_else(|| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "token shielded pool total balance overflow when shielding".to_string(),
                    ))
                })?)
            }
            TokenPoolBalanceChange::Remove(amount) => {
                let current = self.read_token_shielded_pool_total_balance(
                    &token_id,
                    direct_query_type,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                // A stateless read returns 0, which would take the subtraction below negative,
                // so an estimate prices the write of a pool holding exactly what leaves it. The
                // sum item is a fixed width, so the value it carries does not change the cost.
                let current = if apply { current } else { amount };
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
        // Keep the supply consistent with the balance handed to the identity, as a mint would.
        drive
            .add_to_token_total_supply(
                TOKEN_ID,
                balance,
                true,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("add to total supply");
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
                DirectQueryType::StatefulDirectQuery,
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
        // An outputs-only bundle spends nothing, yet its dummy nullifiers are recorded: they are
        // what lets validation refuse the same bundle entering this pool again.
        assert!(nullifier_is_spent(&drive, &notes[0].nullifier));
        assert!(nullifier_is_spent(&drive, &notes[1].nullifier));
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
    fn should_mint_into_the_pool_raising_the_supply() {
        let (drive, identity_id) = setup(1_000);
        let platform_version = PlatformVersion::latest();

        let notes = [note(1), note(2)];
        let operations = drive
            .token_mint_to_pool_operations(
                TOKEN_ID,
                250,
                false,
                &notes,
                &mut None,
                None,
                platform_version,
            )
            .expect("mint to pool operations");
        apply(&drive, operations);

        assert_eq!(pool_balance(&drive), 250);
        assert_eq!(notes_count(&drive), 2);
        assert_eq!(
            drive
                .fetch_token_total_supply(TOKEN_ID, None, platform_version)
                .expect("supply"),
            Some(1_250)
        );
        // The minter's own balance is not involved.
        assert_eq!(token_balance(&drive, identity_id), Some(1_000));
        // The bundle's dummy nullifiers are recorded, as for a shield.
        assert!(nullifier_is_spent(&drive, &notes[0].nullifier));
        assert!(nullifier_is_spent(&drive, &notes[1].nullifier));
        assert!(drive
            .has_token_shielded_pool(TOKEN_ID, None, &mut vec![], platform_version)
            .expect("pool lookup"));
        assert!(!drive
            .has_token_shielded_pool([9u8; 32], None, &mut vec![], platform_version)
            .expect("pool lookup"));
    }

    #[test]
    fn should_burn_from_the_pool_lowering_the_supply() {
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

        let nullifiers = [[31u8; 32], [32u8; 32]];
        let operations = drive
            .token_burn_from_pool_operations(
                TOKEN_ID,
                150,
                &nullifiers,
                &[note(7)],
                &mut None,
                None,
                platform_version,
            )
            .expect("burn from pool operations");
        apply(&drive, operations);

        assert_eq!(pool_balance(&drive), 250);
        assert_eq!(notes_count(&drive), 3);
        assert_eq!(
            drive
                .fetch_token_total_supply(TOKEN_ID, None, platform_version)
                .expect("supply"),
            Some(850)
        );
        assert_eq!(token_balance(&drive, identity_id), Some(600));
        assert!(nullifier_is_spent(&drive, &nullifiers[0]));
        assert!(nullifier_is_spent(&drive, &nullifiers[1]));
    }

    #[test]
    fn should_reject_burning_more_than_the_pool_holds() {
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

        let result = drive.token_burn_from_pool_operations(
            TOKEN_ID,
            101,
            &[[41u8; 32]],
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

    /// Fee validation quotes a batch with `apply = false` and execution meters the same batch
    /// against state, so a read execution performs that the quote never booked is charged to
    /// nobody. The worst-case layer information the estimate registers covers the batch's
    /// writes, and that margin narrows as a pool fills towards the modelled depth, so it is not
    /// what pays for a read: every read the apply path performs has to be metered on the
    /// estimate path too.
    ///
    /// Only a change of the total reads it. A shielded transfer leaves the total alone and so
    /// meters no read in either mode, which is what keeps what a transfer costs from revealing
    /// how much of the token is shielded.
    ///
    /// What is asserted is that the read is booked, in credits, and not merely that both modes
    /// count one read: the count is equal whenever the read happens at all, so it cannot speak
    /// to what the quote covers.
    ///
    /// It does not yet assert that the quote covers the charge, because it does not. The two
    /// modes price the same read differently — the estimate from the pool's modelled layer,
    /// execution from the tree — and the estimate books less than execution charges: 7,460
    /// credits against 14,180, for a shield and an unshield alike, a shortfall nobody pays.
    /// Closing it sets what a pool operation costs, which is a decision about the fee schedule
    /// and not one this test can make; `estimated_fee >= actual_fee` is the assertion that
    /// belongs here once it is taken.
    #[test]
    fn should_meter_the_pool_total_read_when_estimating_as_well_as_when_applying() {
        let platform_version = PlatformVersion::latest();
        let (drive, identity_id) = setup(1_000);

        // Fill the pool first, so the total the apply path reads is a stored value rather than
        // the absent key a fresh pool has: that read is the one the estimate has to cover.
        let seed = drive
            .token_shield_operations(
                TOKEN_ID,
                identity_id,
                1_000,
                &[note(1)],
                &mut None,
                None,
                platform_version,
            )
            .expect("seed shield operations");
        apply(&drive, seed);

        // What the pool update meters, apart from the grove operations of the batch: those are
        // priced when the batch is applied, against the layer information when estimating and
        // against the trees when applying, so they are no part of what a read costs.
        let metered = |balance_change: TokenPoolBalanceChange, estimating: bool| {
            let mut layer_info = estimating.then(HashMap::new);
            let costs: Vec<LowLevelDriveOperation> = drive
                .token_shielded_pool_update_operations(
                    TOKEN_ID,
                    balance_change,
                    &[],
                    &[note(3)],
                    &mut layer_info,
                    None,
                    platform_version,
                )
                .expect("pool update operations")
                .into_iter()
                .filter(|operation| {
                    matches!(
                        operation,
                        LowLevelDriveOperation::CalculatedCostOperation(_)
                    )
                })
                .collect();
            let reads = costs.len();
            let fee = Drive::calculate_fee(
                None,
                Some(costs),
                &BlockInfo::default().epoch,
                drive.config.epochs_per_era,
                platform_version,
                None,
            )
            .expect("price the metered operations")
            .processing_fee;
            (reads, fee)
        };

        // A shield and a mint into the pool add to the total; an unshield and a burn remove from
        // it. Either way the total is read, so either way the estimate has to meter that read.
        for balance_change in [
            TokenPoolBalanceChange::Add(1),
            TokenPoolBalanceChange::Remove(1),
        ] {
            let (estimated_reads, estimated_fee) = metered(balance_change, true);
            let (actual_reads, actual_fee) = metered(balance_change, false);

            // One read of the total per pool update, in both modes: a pool update moves the
            // total once, so it reads it once.
            assert_eq!(
                (estimated_reads, actual_reads),
                (1, 1),
                "{balance_change:?} must read the total exactly once in either mode"
            );

            // Execution charges for the read, so the quote has to book credits for it. Zero
            // would mean the read reaches state on a path nobody was quoted for.
            assert!(
                actual_fee > 0,
                "{balance_change:?} reads the total when applied but that read costs nothing, so \
                 there is no charge for the estimate to cover"
            );
            assert!(
                estimated_fee > 0,
                "{balance_change:?} meters a read worth {actual_fee} credits when applied and \
                 books {estimated_fee} when estimated, so the read is charged to nobody"
            );
        }

        assert_eq!(
            metered(TokenPoolBalanceChange::Unchanged, true),
            (0, 0),
            "a shielded transfer must meter no read while estimating"
        );
        assert_eq!(
            metered(TokenPoolBalanceChange::Unchanged, false),
            (0, 0),
            "a shielded transfer must meter no read while applying, so that what it costs cannot \
             reveal the pool total"
        );
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
