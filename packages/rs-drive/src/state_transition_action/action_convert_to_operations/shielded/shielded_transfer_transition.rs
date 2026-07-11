use super::{insert_notes, insert_nullifiers, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shielded_transfer::ShieldedTransferTransitionAction;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for ShieldedTransferTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .shielded_transfer_transition
        {
            0 => match self {
                ShieldedTransferTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    // 1. Insert each nullifier (known to not exist after validation)
                    insert_nullifiers(&mut ops, &v0.notes);

                    // 2. Insert notes into CommitmentTree
                    insert_notes(&mut ops, &v0.notes);

                    // 3. Update total balance (pool decreases by fee_amount)
                    let new_total_balance = v0
                        .current_total_balance
                        .checked_sub(v0.fee_amount)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting fee_amount"
                                    .to_string(),
                            ))
                        })?;
                    update_balance(&mut ops, new_total_balance);

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ShieldedTransferTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::shielded::shielded_transfer::v0::ShieldedTransferTransitionActionV0;
    use crate::state_transition_action::shielded::ShieldedActionNote;
    use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x11; 32],
            cmx: [0x22; 32],
            cv_net: [0x22; 32],
            encrypted_note: vec![1, 2, 3],
        }
    }

    fn make_action() -> ShieldedTransferTransitionAction {
        ShieldedTransferTransitionAction::V0(ShieldedTransferTransitionActionV0 {
            notes: vec![make_note()],
            anchor: [0xAA; 32],
            fee_amount: 500,
            current_total_balance: 10000,
        })
    }

    #[test]
    fn test_produces_nullifiers_notes_and_balance_update() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // InsertNullifiers + InsertNote (1 note) + UpdateTotalBalance
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_first_op_is_insert_nullifiers() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::InsertNullifiers { nullifiers },
            ) => {
                assert_eq!(nullifiers.len(), 1);
                assert_eq!(nullifiers[0], [0x11; 32]);
            }
            other => panic!("expected InsertNullifiers, got {:?}", other),
        }
    }

    #[test]
    fn test_balance_decreases_by_fee_amount() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match ops.last().unwrap() {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => {
                assert_eq!(*new_total_balance, 9500); // 10000 - 500
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_underflow_returns_error() {
        let action = ShieldedTransferTransitionAction::V0(ShieldedTransferTransitionActionV0 {
            notes: vec![],
            anchor: [0x00; 32],
            fee_amount: 10001,
            current_total_balance: 10000,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }

    /// Invariant: the flat `compute_minimum_shielded_fee` must cover the *actual* GroveDB write
    /// cost (`Drive::calculate_fee`) of a shielded transition's operations.
    ///
    /// A shielded transfer is the cleanest per-action case — insert nullifiers + notes +
    /// pool-balance update, with no contract document — so it isolates the dominant variable
    /// cost (note storage). We use production-sized notes (216-byte encrypted note → 280-byte
    /// commitment-tree item) and measure the real cost in estimation mode (`apply = false`).
    ///
    /// The flat fee covers it with large margin because it also bundles a flat 100M
    /// proof-verification fee that `calculate_fee` never charges (Halo 2 verification is CPU,
    /// not a GroveDB op). 16 is the max actions per bundle.
    #[test]
    fn test_minimum_shielded_fee_covers_actual_grovedb_write_cost() {
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
        use dpp::block::block_info::BlockInfo;
        use dpp::shielded::compute_minimum_shielded_fee;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(0).unwrap();

        // Production-sized note: 216-byte encrypted note, distinct nullifier/cmx per action.
        let realistic_note = |i: u8| ShieldedActionNote {
            nullifier: [i.wrapping_add(1); 32],
            cmx: [i.wrapping_add(101); 32],
            cv_net: [i.wrapping_add(201); 32],
            encrypted_note: vec![0x77; 216],
        };

        for num_actions in [1usize, 8, 16] {
            let fee_amount = compute_minimum_shielded_fee(num_actions, platform_version)
                .expect("fee computation should not overflow");
            let notes: Vec<_> = (0..num_actions as u8).map(realistic_note).collect();
            let action = ShieldedTransferTransitionAction::V0(ShieldedTransferTransitionActionV0 {
                notes,
                anchor: [0xAA; 32],
                fee_amount,
                current_total_balance: fee_amount + 1_000_000,
            });

            let ops = action
                .into_high_level_drive_operations(&epoch, platform_version)
                .expect("operations");

            // apply = false → estimation mode: no DB mutation, returns the real cost.
            let fee_result = drive
                .apply_drive_operations(
                    ops,
                    false,
                    &BlockInfo::default(),
                    None,
                    platform_version,
                    None,
                )
                .expect("estimate write cost");
            let actual_cost = fee_result.total_base_fee();

            // The fee must cover the real write cost. Measured margins over GroveDB cost
            // (estimation mode, production-sized notes): ~10.9x at 1 action down to ~5.8x at
            // the 16-action max. The margin is large and stays well above 1x because the
            // per-action fee also prices the per-action Halo 2 verification CPU (which
            // calculate_fee does not charge), so it exceeds the per-action GroveDB cost by
            // design; see `shielded_per_action_processing_fee`.
            assert!(
                fee_amount >= actual_cost,
                "compute_minimum_shielded_fee({num_actions}) = {fee_amount} must cover the actual \
                 GroveDB write cost {actual_cost} (storage {} + processing {})",
                fee_result.storage_fee,
                fee_result.processing_fee
            );

            // Pin the booking-split invariant directly. The pool-paid booking in
            // `execute_event/v0` splits the flat carved fee as
            //   storage_fee = min(real_metered_storage, flat_fee)
            //   processing_fee = flat_fee - storage_fee
            // The `min()` only ever binds — zeroing the proposer's processing reward and
            // undercharging storage — if the real metered storage EXCEEDS the flat fee. Asserting
            // `flat_fee > real_metered_storage` here is exactly the condition that guarantees the
            // `min()` is a no-op, so the proposer is always paid the processing remainder and
            // storage is never undercharged. (Strict `>` because the flat fee also bundles the 100M
            // proof-verification fee that GroveDB never meters.)
            assert!(
                fee_amount > fee_result.storage_fee,
                "compute_minimum_shielded_fee({num_actions}) = {fee_amount} must strictly exceed the \
                 real metered storage {} so the booking split's min(real_storage, flat_fee) never \
                 binds (proposer processing reward never zeroed, storage never undercharged)",
                fee_result.storage_fee
            );
        }
    }
}
