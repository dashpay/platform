use super::{insert_notes, insert_nullifiers, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::identity_create_from_shielded_pool::IdentityCreateFromShieldedPoolTransitionAction;
use crate::util::batch::DriveOperation;
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::IdentityOperationType;
use dpp::block::epoch::Epoch;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityCreateFromShieldedPoolTransitionAction {
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
            .identity_create_from_shielded_pool_transition
        {
            0 => match self {
                IdentityCreateFromShieldedPoolTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    // Defense in depth: the new identity is created holding the full denomination
                    // (`AddNewIdentity{ balance }`) AND the system-credits / pool accounting is keyed
                    // off `denomination`. If those two ever diverged, the emitted ops would mint or
                    // burn credits. The transformer builds the identity with `balance = denomination`,
                    // so this can't happen — but assert it before emitting any op rather than trust
                    // two separate sources of truth.
                    if v0.identity.balance() != v0.denomination {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                            "identity balance {} must equal the shielded exit denomination {}",
                            v0.identity.balance(),
                            v0.denomination
                        ))));
                    }

                    // 1. Insert each nullifier (validated to not already exist) — double-spend
                    //    prevention. These also serve as the id-derivation preimage.
                    insert_nullifiers(&mut ops, &v0.notes);

                    // 2. Create the new identity holding the FULL denomination, funded by the pool
                    //    decrement in step 4. This is a move BETWEEN two right-hand-side terms of the
                    //    credit-conservation equation (shielded-pool balance -> identity balance),
                    //    exactly like Unshield (pool -> address): the credits already exist in the
                    //    supply (the shielded pool is itself a counted balance tree), so the global
                    //    system-credits scalar (the conservation equation's LHS) must NOT change.
                    //    `AddToSystemCredits`/`RemoveFromSystemCredits` are correct ONLY when credits
                    //    cross the platform boundary (asset-lock inflow: IdentityCreate /
                    //    ShieldFromAssetLock; Core-withdrawal outflow: ShieldedWithdrawal) — NOT for
                    //    this pool-internal move. Emitting one here would over-mint by `denomination`
                    //    and halt the chain at the end-of-block sum-tree check. The fee is later moved
                    //    from this balance into the fee pools at execution (also RHS-internal), so the
                    //    identity ends with `denomination - fee_amount` and credits are conserved.
                    ops.push(IdentityOperation(IdentityOperationType::AddNewIdentity {
                        identity: v0.identity,
                        is_masternode_identity: false,
                    }));

                    // 3. Insert each action's output note into the CommitmentTree (change re-enters
                    //    the pool as an ordinary, indistinguishable Orchard output).
                    insert_notes(&mut ops, &v0.notes);

                    // 4. Decrement the shielded pool by exactly `denomination` (= the Orchard
                    //    value_balance leaving the pool; change stays internal to the bundle). This
                    //    RHS decrement exactly offsets the new identity's balance (step 2), so the
                    //    converter is conservation-neutral with no change to the system-credits scalar.
                    let new_total_balance = v0
                        .current_total_balance
                        .checked_sub(v0.denomination)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting identity-create denomination"
                                    .to_string(),
                            ))
                        })?;
                    update_balance(&mut ops, new_total_balance);

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "IdentityCreateFromShieldedPoolTransitionAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::shielded::identity_create_from_shielded_pool::v0::IdentityCreateFromShieldedPoolTransitionActionV0;
    use crate::state_transition_action::shielded::ShieldedActionNote;
    use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
    use crate::util::batch::DriveOperation::SystemOperation;
    use crate::util::batch::SystemOperationType;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_note(i: u8) -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [i; 32],
            cmx: [i.wrapping_add(100); 32],
            cv_net: [i.wrapping_add(50); 32],
            encrypted_note: vec![0x77; 216],
        }
    }

    fn make_identity(balance: u64) -> Identity {
        let platform_version = PlatformVersion::latest();
        let mut identity = Identity::new_with_id_and_keys(
            Identifier::from([0xAA; 32]),
            BTreeMap::new(),
            platform_version,
        )
        .expect("identity");
        use dpp::identity::accessors::IdentitySettersV0;
        identity.set_balance(balance);
        identity
    }

    fn make_action(
        denomination: u64,
        fee_amount: u64,
        pool: u64,
    ) -> IdentityCreateFromShieldedPoolTransitionAction {
        IdentityCreateFromShieldedPoolTransitionAction::V0(
            IdentityCreateFromShieldedPoolTransitionActionV0 {
                identity: make_identity(denomination),
                notes: vec![make_note(1)],
                anchor: [0xAA; 32],
                denomination,
                fee_amount,
                current_total_balance: pool,
            },
        )
    }

    #[test]
    fn test_produces_expected_ops() {
        let action = make_action(10_000_000_000, 500_000_000, 50_000_000_000);
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();
        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("ops");
        // InsertNullifiers + AddNewIdentity + InsertNote(1) + UpdateTotalBalance
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn test_does_not_touch_system_credits() {
        // CRITICAL conservation invariant: this is a pool -> identity move BETWEEN two RHS balance
        // trees (like Unshield), so it must NOT emit AddToSystemCredits / RemoveFromSystemCredits.
        // The shielded-pool credits already exist in the supply; re-minting them on the LHS scalar
        // over-counts by `denomination` and halts the chain at the end-of-block sum-tree check.
        let action = make_action(10_000_000_000, 500_000_000, 50_000_000_000);
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();
        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("ops");

        let touches_system_credits = ops.iter().any(|op| {
            matches!(
                op,
                SystemOperation(SystemOperationType::AddToSystemCredits { .. })
                    | SystemOperation(SystemOperationType::RemoveFromSystemCredits { .. })
            )
        });
        assert!(
            !touches_system_credits,
            "a pool -> identity move must NOT mint/burn system credits (the LHS scalar)"
        );
    }

    #[test]
    fn test_identity_balance_is_full_denomination() {
        let denomination = 25_000_000_000u64;
        let action = make_action(denomination, 500_000_000, 50_000_000_000);
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();
        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("ops");
        for op in &ops {
            if let IdentityOperation(IdentityOperationType::AddNewIdentity { identity, .. }) = op {
                assert_eq!(identity.balance(), denomination);
            }
        }
    }

    #[test]
    fn test_pool_decrements_by_denomination() {
        let denomination = 10_000_000_000u64;
        let pool = 50_000_000_000u64;
        let action = make_action(denomination, 500_000_000, pool);
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();
        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("ops");
        match ops.last().unwrap() {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => assert_eq!(*new_total_balance, pool - denomination),
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    /// Op-level conservation modeling the REAL sum-tree equation
    /// `total_credits_in_platform (LHS) == pools + identity_balances + specialized + addresses +
    /// shielded_balances (RHS)`. This converter must leave the LHS scalar unchanged (no
    /// AddToSystemCredits/RemoveFromSystemCredits) and offset the new identity's balance (an RHS
    /// `Balances` credit) exactly against the shielded-pool decrement (an RHS `ShieldedBalances`
    /// debit), so the net credit-supply change is ZERO. (The fee is later moved from the identity
    /// balance into the fee pools at execution — an RHS-internal transfer, not a mint/burn.)
    #[test]
    fn test_conservation_rhs_internal_no_lhs_change() {
        let denomination = 10_000_000_000u64;
        let pool = 50_000_000_000u64;
        let action = make_action(denomination, 500_000_000, pool);
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();
        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("ops");

        let mut lhs_delta: i128 = 0; // AddToSystemCredits / RemoveFromSystemCredits (the LHS scalar)
        let mut identity_balance_delta: i128 = 0; // AddNewIdentity balance (RHS Balances)
        let mut pool_delta: i128 = 0; // pool UpdateTotalBalance (RHS ShieldedBalances)
        for op in &ops {
            match op {
                SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
                    lhs_delta += *amount as i128
                }
                SystemOperation(SystemOperationType::RemoveFromSystemCredits { amount }) => {
                    lhs_delta -= *amount as i128
                }
                IdentityOperation(IdentityOperationType::AddNewIdentity { identity, .. }) => {
                    identity_balance_delta += identity.balance() as i128
                }
                DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
                ) => pool_delta = *new_total_balance as i128 - pool as i128,
                _ => {}
            }
        }
        assert_eq!(
            lhs_delta, 0,
            "a pool -> identity move must NOT change the system-credits scalar (LHS)"
        );
        assert_eq!(
            identity_balance_delta + pool_delta,
            0,
            "the new identity balance (RHS) must be exactly funded by the shielded-pool decrement (RHS)"
        );
    }

    #[test]
    fn test_pool_underflow_errors() {
        let action = make_action(50_000_000_000, 500_000_000, 10_000_000_000); // pool < denomination
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();
        assert!(action
            .into_high_level_drive_operations(&epoch, platform_version)
            .is_err());
    }
}
