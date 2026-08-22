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

    /// Invariant: the flat `compute_minimum_shielded_fee` covers the **amortized**
    /// real GroveDB write cost of a shielded transfer, and stays above the
    /// amortized real storage so the pool-paid booking split
    /// (`storage_fee = min(actual_storage, flat)`, `processing = flat -
    /// storage_fee`) never starves the proposer.
    ///
    /// Amortized, because the fee is flat and pool-paid: the note appends that
    /// compact a full dense-buffer epoch into a chunk blob cost far more than
    /// the others (the epoch's bytes are rewritten as replaced storage, at the
    /// processing rate), and the pool absorbs that by design — the other
    /// `epoch - 1` appends overpay it, and a client cannot land on the
    /// compaction more than once per epoch. The per-append worst-case
    /// *estimate* is therefore not the floor for a pool-paid fee; the
    /// measured epoch average is. Measured on a real pool, one epoch of real
    /// appends including the compacting one (see `fee_floor_support`).
    ///
    /// The booking split is checked at its worst point too: even the
    /// compacting append's real storage must stay below the flat fee, so
    /// `min()` never zeroes the proposer's processing share on any append.
    #[test]
    fn test_minimum_shielded_fee_covers_actual_grovedb_write_cost() {
        use super::super::fee_floor_support::transfer_epoch;
        use dpp::shielded::compute_minimum_shielded_fee;

        let platform_version = PlatformVersion::latest();
        let epoch = transfer_epoch();

        for num_actions in [1u64, 8, 16] {
            let fee_amount = compute_minimum_shielded_fee(num_actions as usize, platform_version)
                .expect("fee computation should not overflow");
            // Each action is one note append + one nullifier; the amortized
            // per-append cost scales linearly, and the flat fee also carries
            // the fixed proof-verification term, so this holds with margin.
            assert!(
                fee_amount >= num_actions * epoch.avg_total,
                "compute_minimum_shielded_fee({num_actions}) = {fee_amount} must cover the \
                 amortized real write cost {} x {} (epoch of {} appends; compacting append \
                 total {}, ordinary append total {})",
                num_actions,
                epoch.avg_total,
                epoch.appends,
                epoch.boundary_total,
                epoch.ordinary_total
            );
            assert!(
                fee_amount > num_actions * epoch.avg_storage,
                "flat fee {fee_amount} must exceed the amortized real storage {} x {} so the \
                 pool-paid booking split never starves the proposer",
                num_actions,
                epoch.avg_storage
            );
        }
        let fee_one = compute_minimum_shielded_fee(1, platform_version).expect("fee");
        assert!(
            epoch.boundary_storage < fee_one,
            "even the compacting append's real storage ({}) must stay below the flat fee \
             ({fee_one}), or `min(actual_storage, flat)` would zero the proposer's share",
            epoch.boundary_storage
        );
    }
}
