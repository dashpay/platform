use super::{insert_notes, insert_nullifiers, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::unshield::UnshieldTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for UnshieldTransitionAction {
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
            .unshield_transition
        {
            0 => match self {
                UnshieldTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    // 1. Insert each nullifier (known to not exist after validation)
                    insert_nullifiers(&mut ops, &v0.notes);

                    // 2. Credit the output address with the NET unshielded amount.
                    //    `amount` (= unshielding_amount) is the total leaving the pool; the
                    //    fee is carved out of it and routed to the fee pools via the
                    //    PaidFromShieldedPool execution event, so the recipient receives
                    //    `amount - fee_amount`. Validation (validate_minimum_shielded_fee)
                    //    guarantees `amount >= fee_amount`, so this subtraction cannot
                    //    underflow; we still guard defensively.
                    let net_recipient_amount =
                        v0.amount.checked_sub(v0.fee_amount).ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "unshield fee exceeds unshielding amount".to_string(),
                            ))
                        })?;
                    // Only credit the output address when the net is positive. A
                    // net-zero unshield (the whole `unshielding_amount` was consumed by
                    // the fee) would otherwise insert a spurious zero-balance address
                    // entry. Skipping it is conservation-neutral (adding 0 changes
                    // nothing): the pool still decreases by `amount` and the fee still
                    // flows to the fee pools.
                    if net_recipient_amount > 0 {
                        ops.push(DriveOperation::AddressFundsOperation(
                            AddressFundsOperationType::AddBalanceToAddress {
                                address: v0.output_address,
                                balance_to_add: net_recipient_amount,
                            },
                        ));
                    }

                    // 3. Insert notes into CommitmentTree (change outputs)
                    insert_notes(&mut ops, &v0.notes);

                    // 4. Update total balance.
                    //    The pool decreases by the full `amount` (= unshielding_amount).
                    //    Of that, `amount - fee_amount` is credited to the output address
                    //    above and `fee_amount` flows to the fee pools at block finalization,
                    //    so credits are conserved.
                    let new_total_balance =
                        v0.current_total_balance
                            .checked_sub(v0.amount)
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting unshield amount"
                                    .to_string(),
                            ))
                            })?;
                    update_balance(&mut ops, new_total_balance);

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "UnshieldTransitionAction::into_high_level_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
    use crate::state_transition_action::shielded::ShieldedActionNote;
    use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
    use dpp::address_funds::PlatformAddress;
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

    fn make_action() -> UnshieldTransitionAction {
        UnshieldTransitionAction::V0(UnshieldTransitionActionV0 {
            output_address: PlatformAddress::P2pkh([0xBB; 20]),
            amount: 3000,
            notes: vec![make_note()],
            anchor: [0xAA; 32],
            fee_amount: 500,
            current_total_balance: 10000,
            chargeable_failure: false,
        })
    }

    #[test]
    fn test_produces_nullifiers_add_balance_notes_and_update_balance() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // InsertNullifiers + AddBalanceToAddress + InsertNote (1) + UpdateTotalBalance
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn test_add_net_balance_to_output_address() {
        // The output address receives the NET amount (amount - fee), not the full amount.
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[1] {
            DriveOperation::AddressFundsOperation(
                AddressFundsOperationType::AddBalanceToAddress {
                    address,
                    balance_to_add,
                },
            ) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0xBB; 20]));
                assert_eq!(*balance_to_add, 2500); // 3000 amount - 500 fee
            }
            other => panic!("expected AddBalanceToAddress, got {:?}", other),
        }
    }

    #[test]
    fn test_balance_decreases_by_amount_only() {
        // The pool decrements by exactly `amount` (= unshielding_amount). The fee is
        // carved out of that amount, not charged on top of it.
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
                assert_eq!(*new_total_balance, 7000); // 10000 - 3000 (amount only)
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_fee_amount_is_non_zero() {
        // Regression guard for the fee-bypass bug: unshield must charge a non-zero fee.
        let action = make_action();
        match action {
            UnshieldTransitionAction::V0(v0) => assert!(v0.fee_amount > 0),
        }
    }

    #[test]
    fn test_conservation_sum_tree_delta_equals_negative_fee() {
        // Conservation at the operation level: the address sum tree gains (amount - fee)
        // and the shielded pool sum tree loses `amount`, so the net sum-tree change is
        // -fee. The fee is reconciled into the fee pools at block finalization.
        let amount = 3000u64;
        let fee = 500u64;
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        let mut address_delta: i128 = 0;
        let mut shielded_delta: i128 = 0;
        for op in &ops {
            match op {
                DriveOperation::AddressFundsOperation(
                    AddressFundsOperationType::AddBalanceToAddress { balance_to_add, .. },
                ) => address_delta += *balance_to_add as i128,
                DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
                ) => shielded_delta = *new_total_balance as i128 - 10000i128,
                _ => {}
            }
        }

        assert_eq!(address_delta, (amount - fee) as i128);
        assert_eq!(shielded_delta, -(amount as i128));
        // Net sum-tree change (before fee distribution into pools) is -fee.
        assert_eq!(address_delta + shielded_delta, -(fee as i128));
    }

    #[test]
    fn test_fee_exceeds_amount_returns_error() {
        // Defensive guard: if fee somehow exceeds amount, the net-recipient subtraction
        // must error rather than underflow.
        let action = UnshieldTransitionAction::V0(UnshieldTransitionActionV0 {
            output_address: PlatformAddress::P2pkh([0xBB; 20]),
            amount: 100,
            notes: vec![],
            anchor: [0x00; 32],
            fee_amount: 500, // fee > amount
            current_total_balance: 10000,
            chargeable_failure: false,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }

    #[test]
    fn test_balance_underflow_returns_error() {
        let action = UnshieldTransitionAction::V0(UnshieldTransitionActionV0 {
            output_address: PlatformAddress::P2pkh([0xBB; 20]),
            amount: 5000,
            notes: vec![],
            anchor: [0x00; 32],
            fee_amount: 500,
            current_total_balance: 4000, // 4000 < 5000 (amount)
            chargeable_failure: false,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }

    /// Invariant: the flat `compute_shielded_unshield_fee` covers the **amortized**
    /// real GroveDB write cost of an Unshield, and stays above the amortized
    /// real storage so the pool-paid booking split never starves the
    /// proposer — the same amortized reading as the ShieldedTransfer test
    /// (see there, and `fee_floor_support`, for why a flat pool-paid fee is
    /// held to the epoch average rather than the per-append worst case).
    ///
    /// An Unshield is a transfer's note/nullifier writes plus the
    /// `AddBalanceToAddress` output write, so its amortized cost is the
    /// transfer epoch average plus the measured per-transition delta between
    /// an ordinary unshield and an ordinary transfer.
    #[test]
    fn test_minimum_shielded_fee_covers_actual_grovedb_write_cost() {
        use super::super::fee_floor_support::{note, transfer_action, transfer_epoch};
        use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
        use dpp::block::block_info::BlockInfo;
        use dpp::shielded::compute_shielded_unshield_fee;

        let platform_version = PlatformVersion::latest();
        let epoch_info = Epoch::new(0).unwrap();
        let epoch = transfer_epoch();

        // The per-transition delta of an Unshield over a ShieldedTransfer at
        // an ordinary (non-compacting) position: the output address write.
        let measure = |ops: Vec<crate::util::batch::DriveOperation>| {
            let drive = setup_drive_with_initial_state_structure(None);
            drive
                .apply_drive_operations(
                    ops,
                    true,
                    &BlockInfo::default(),
                    None,
                    platform_version,
                    None,
                )
                .expect("apply")
        };
        let fee_one = compute_shielded_unshield_fee(1, platform_version).expect("fee");
        let amount = fee_one + 1_000_000;
        let unshield_one = measure(
            UnshieldTransitionAction::V0(UnshieldTransitionActionV0 {
                output_address: PlatformAddress::P2pkh([0xBB; 20]),
                amount,
                notes: vec![note(7)],
                anchor: [0xAA; 32],
                fee_amount: fee_one,
                current_total_balance: amount + 1_000_000,
                chargeable_failure: false,
            })
            .into_high_level_drive_operations(&epoch_info, platform_version)
            .expect("operations"),
        );
        let transfer_one = measure(
            transfer_action(7, fee_one)
                .into_high_level_drive_operations(&epoch_info, platform_version)
                .expect("operations"),
        );
        let delta_total = unshield_one
            .total_base_fee()
            .saturating_sub(transfer_one.total_base_fee());
        let delta_storage = unshield_one
            .storage_fee
            .saturating_sub(transfer_one.storage_fee);

        for num_actions in [1u64, 8, 16] {
            let fee_amount = compute_shielded_unshield_fee(num_actions as usize, platform_version)
                .expect("fee computation should not overflow");
            assert!(
                fee_amount >= num_actions * epoch.avg_total + delta_total,
                "compute_shielded_unshield_fee({num_actions}) = {fee_amount} must cover the \
                 amortized real Unshield write cost {} x {} + {} (output write)",
                num_actions,
                epoch.avg_total,
                delta_total
            );
            assert!(
                fee_amount > num_actions * epoch.avg_storage + delta_storage,
                "flat fee {fee_amount} must exceed the amortized real storage {} x {} + {} so \
                 the pool-paid booking split never starves the proposer",
                num_actions,
                epoch.avg_storage,
                delta_storage
            );
        }
        assert!(
            epoch.boundary_storage + delta_storage < fee_one,
            "even the compacting append's real storage ({} + {}) must stay below the flat fee \
             ({fee_one}), or `min(actual_storage, flat)` would zero the proposer's share",
            epoch.boundary_storage,
            delta_storage
        );
    }

    #[test]
    fn test_net_zero_unshield_skips_address_credit() {
        // When the whole `unshielding_amount` is consumed by the fee (net == 0),
        // the conversion must NOT emit a zero-credit AddBalanceToAddress (which
        // would create a spurious dust address entry). The pool still decrements
        // by `amount` and the fee still flows to the fee pools.
        let action = UnshieldTransitionAction::V0(UnshieldTransitionActionV0 {
            output_address: PlatformAddress::P2pkh([0xBB; 20]),
            amount: 500,
            notes: vec![make_note()],
            anchor: [0xAA; 32],
            fee_amount: 500, // net = amount - fee = 0
            current_total_balance: 10000,
            chargeable_failure: false,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert!(
            !ops.iter().any(|op| matches!(
                op,
                DriveOperation::AddressFundsOperation(
                    AddressFundsOperationType::AddBalanceToAddress { .. }
                )
            )),
            "net-zero unshield must not emit a zero-credit AddBalanceToAddress"
        );

        // The pool is still decremented by exactly `amount` (10000 - 500 = 9500).
        match ops.last().unwrap() {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => assert_eq!(*new_total_balance, 9500),
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }
}
