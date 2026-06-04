use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::read_pool_total_balance;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shield::ShieldTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

/// Reallocates the per-input remaining balances so that the TOTAL amount debited
/// from the source addresses equals exactly `shield_amount` (the value entering the
/// shielded pool), instead of the sum of every input's `requested` value.
///
/// # Why this is needed
///
/// Each `ShieldTransitionV0::inputs` entry maps an address to `(nonce, requested)`,
/// where `requested` is a **max contribution**, not an exact debit (see the field
/// doc on `ShieldTransitionV0::inputs`: "The total across all inputs must cover
/// |value_balance| + fees. Excess credits remain in the source addresses."). The
/// shielded pool is only credited `shield_amount` (the Orchard `value_balance`), and
/// the Orchard binding signature only constrains that the pool receives exactly
/// `shield_amount` — it does **not** constrain how the transparent side is split
/// across the individual transparent inputs.
///
/// The shared address-balance validation
/// (`validate_address_balances_and_nonces_internal_validation`) produces
/// `inputs_with_remaining_balance[addr] = actual - requested`, i.e. it debits the
/// FULL `requested` from every input. If we used that directly, the addresses would
/// lose `Σrequested` while the pool gains only `shield_amount`, destroying the excess
/// `Σrequested - shield_amount` credits and tripping the block-end sum-tree
/// conservation check (`CorruptedCreditsNotBalanced`), halting the chain.
///
/// # The allocation
///
/// We greedily consume `shield_amount` across the inputs in their natural
/// `BTreeMap` (address) order — deterministic across all nodes, since `BTreeMap`
/// iteration order is fully determined by the keys. For each input we recover its
/// `actual` balance (`remaining_after_full_debit + requested`) and debit at most its
/// own `requested` (so no input is over-spent beyond what its witness authorized).
/// The structure validation has already enforced `Σrequested >= shield_amount`, so
/// the loop always consumes the full `shield_amount`; the trailing assertion is a
/// defensive, should-be-unreachable guard.
///
/// The remaining `requested - consumed` per input is simply left in the source
/// address. The existing `PaidFromAddressInputs` fee machinery then deducts the fee
/// on top of these adjusted balances, so addresses lose exactly
/// `shield_amount + fee`, the pool gains `shield_amount`, and the fee pools gain
/// `fee` — credits are conserved.
fn reallocate_inputs_for_shield_amount(
    transition: &ShieldTransition,
    inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    shield_amount: Credits,
) -> Result<BTreeMap<PlatformAddress, (AddressNonce, Credits)>, Error> {
    let requested_inputs = match transition {
        ShieldTransition::V0(v0) => &v0.inputs,
    };

    let mut adjusted = BTreeMap::new();
    let mut remaining_to_consume = shield_amount;

    for (addr, (nonce, remaining_after_full_debit)) in inputs_with_remaining_balance {
        // Look up the per-input max contribution (`requested`) from the transition.
        let (_input_nonce, requested) = requested_inputs.get(&addr).ok_or_else(|| {
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "shield input address present in remaining balances is missing from transition inputs",
            ))
        })?;
        let requested = *requested;

        // Recover the address's actual on-chain balance before this shield.
        // `remaining_after_full_debit == actual - requested`, so `actual` is the sum.
        let actual = remaining_after_full_debit
            .checked_add(requested)
            .ok_or_else(|| {
                Error::Execution(ExecutionError::Overflow(
                    "overflow recovering actual address balance from shield remaining balance",
                ))
            })?;

        // Greedily consume up to this input's max contribution.
        let consumed = remaining_to_consume.min(requested);
        remaining_to_consume -= consumed;

        // `consumed <= requested <= actual`, so this cannot underflow; use checked
        // arithmetic defensively in case of corrupted state.
        let new_remaining = actual.checked_sub(consumed).ok_or_else(|| {
            Error::Execution(ExecutionError::Overflow(
                "underflow computing adjusted shield remaining balance",
            ))
        })?;

        adjusted.insert(addr, (nonce, new_remaining));
    }

    // Structure validation already enforces `Σrequested >= shield_amount`, so the
    // greedy loop must have consumed the entire amount. A non-zero remainder would
    // mean the pool is credited more than the addresses are debited (minting credits),
    // so reject defensively. This branch should be unreachable.
    if remaining_to_consume != 0 {
        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "shield amount exceeds the sum of input contributions after reallocation",
        )));
    }

    Ok(adjusted)
}

pub(in crate::execution::validation::state_transition::state_transitions::shield) trait ShieldStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ShieldStateTransitionTransformIntoActionValidationV0 for ShieldTransition {
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let shield_amount: Credits = match self {
            ShieldTransition::V0(v0) => v0.amount,
        };

        // The shared address-balance validation debits the FULL per-input `requested`
        // (a max contribution), but the shielded pool only receives `shield_amount`.
        // Reallocate so the total debited across inputs equals exactly `shield_amount`,
        // leaving the excess in the source addresses. This keeps credits conserved
        // (addresses lose `shield_amount` + fee, pool gains `shield_amount`).
        let inputs_with_remaining_balance = reallocate_inputs_for_shield_amount(
            self,
            inputs_with_remaining_balance,
            shield_amount,
        )?;

        // Read current shielded pool state from GroveDB
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

        // Calculate fees from the GroveDB operations
        let fee = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        let result = ShieldTransitionAction::try_from_transition(
            self,
            inputs_with_remaining_balance,
            shield_amount,
            current_total_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::PlatformAddress;
    use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
    use std::collections::BTreeMap;

    /// Builds a minimal `ShieldTransition` whose `inputs` map carries the per-input
    /// `(nonce, requested)` pairs the reallocation needs. All cryptographic/structural
    /// fields are dummy values — they are irrelevant to the reallocation logic.
    fn shield_transition_with_inputs(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        amount: Credits,
    ) -> ShieldTransition {
        ShieldTransition::V0(ShieldTransitionV0 {
            inputs,
            actions: vec![],
            amount,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
            fee_strategy: vec![],
            user_fee_increase: 0,
            input_witnesses: vec![],
        })
    }

    /// For a single input where `actual == requested`, exactly `shield_amount` should be
    /// debited, leaving `actual - shield_amount` in the address.
    #[test]
    fn test_reallocation_single_input_debits_exactly_shield_amount() {
        let addr = PlatformAddress::P2pkh([0xAA; 20]);
        let actual: Credits = 1_000_000;
        let requested: Credits = 1_000_000;
        let shield_amount: Credits = 500_000;

        // Transition inputs carry `requested` as the max contribution.
        let mut transition_inputs = BTreeMap::new();
        transition_inputs.insert(addr, (1u32, requested));
        let transition = shield_transition_with_inputs(transition_inputs, shield_amount);

        // `inputs_with_remaining_balance` from the shared validation = actual - requested.
        let mut remaining = BTreeMap::new();
        remaining.insert(addr, (1u32, actual - requested));

        let adjusted =
            reallocate_inputs_for_shield_amount(&transition, remaining, shield_amount).unwrap();

        let (nonce, new_remaining) = adjusted.get(&addr).copied().unwrap();
        assert_eq!(nonce, 1u32, "nonce must be preserved");
        assert_eq!(
            new_remaining,
            actual - shield_amount,
            "single input must retain actual - shield_amount (={})",
            actual - shield_amount
        );
        assert_eq!(new_remaining, 500_000);

        // Total debited across inputs equals exactly shield_amount.
        let total_debited = actual - new_remaining;
        assert_eq!(total_debited, shield_amount);
    }

    /// For multiple inputs, the SUM of (actual - adjusted) across inputs must equal
    /// exactly `shield_amount`, each adjusted balance must be >= its full-debit remaining,
    /// and the greedy allocation must consume the entire amount (remaining_to_consume == 0).
    #[test]
    fn test_reallocation_multi_input_sum_of_debits_equals_shield_amount() {
        let addr_a = PlatformAddress::P2pkh([0x01; 20]);
        let addr_b = PlatformAddress::P2pkh([0x02; 20]);
        let addr_c = PlatformAddress::P2pkh([0x03; 20]);

        // requested (max contributions): 100k / 200k / 300k, total 600k.
        let req_a: Credits = 100_000;
        let req_b: Credits = 200_000;
        let req_c: Credits = 300_000;

        // Give each address a generous actual balance so `actual = remaining + requested`.
        let actual_a: Credits = 100_000;
        let actual_b: Credits = 250_000;
        let actual_c: Credits = 400_000;

        let shield_amount: Credits = 50_000;

        let mut transition_inputs = BTreeMap::new();
        transition_inputs.insert(addr_a, (1u32, req_a));
        transition_inputs.insert(addr_b, (1u32, req_b));
        transition_inputs.insert(addr_c, (1u32, req_c));
        let transition = shield_transition_with_inputs(transition_inputs, shield_amount);

        // remaining_after_full_debit = actual - requested per input.
        let rem_a = actual_a - req_a; // 0
        let rem_b = actual_b - req_b; // 50_000
        let rem_c = actual_c - req_c; // 100_000

        let mut remaining = BTreeMap::new();
        remaining.insert(addr_a, (1u32, rem_a));
        remaining.insert(addr_b, (1u32, rem_b));
        remaining.insert(addr_c, (1u32, rem_c));

        let adjusted =
            reallocate_inputs_for_shield_amount(&transition, remaining.clone(), shield_amount)
                .unwrap();

        // The SUM of (actual - adjusted) across inputs == shield_amount exactly.
        let actuals = [(addr_a, actual_a), (addr_b, actual_b), (addr_c, actual_c)];
        let mut total_debited: Credits = 0;
        for (addr, actual) in actuals {
            let (_nonce, adj) = adjusted.get(&addr).copied().unwrap();
            total_debited += actual - adj;
        }
        assert_eq!(
            total_debited, shield_amount,
            "sum of debits across inputs must equal shield_amount exactly"
        );

        // Each adjusted balance >= its full-debit remaining (we debit <= requested).
        for (addr, (_n, full_debit_remaining)) in &remaining {
            let (_nonce, adj) = adjusted.get(addr).copied().unwrap();
            assert!(
                adj >= *full_debit_remaining,
                "adjusted remaining ({}) must be >= full-debit remaining ({}) for {:?}",
                adj,
                full_debit_remaining,
                addr
            );
        }

        // Greedy allocation: first input (addr_a, requested 100k) covers the whole 50k.
        let (_n_a, adj_a) = adjusted.get(&addr_a).copied().unwrap();
        assert_eq!(
            adj_a,
            actual_a - shield_amount,
            "first input absorbs all 50k"
        );
        // The other two inputs are untouched (left at their actual balances).
        let (_n_b, adj_b) = adjusted.get(&addr_b).copied().unwrap();
        let (_n_c, adj_c) = adjusted.get(&addr_c).copied().unwrap();
        assert_eq!(adj_b, actual_b, "second input untouched");
        assert_eq!(adj_c, actual_c, "third input untouched");
    }

    /// When the amount spills past the first input's max contribution, allocation
    /// continues greedily into subsequent inputs in BTreeMap order.
    #[test]
    fn test_reallocation_multi_input_spillover_across_inputs() {
        let addr_a = PlatformAddress::P2pkh([0x01; 20]);
        let addr_b = PlatformAddress::P2pkh([0x02; 20]);

        let req_a: Credits = 100_000;
        let req_b: Credits = 200_000;
        let actual_a: Credits = 100_000;
        let actual_b: Credits = 200_000;

        // 250k needs all of addr_a (100k) plus 150k of addr_b.
        let shield_amount: Credits = 250_000;

        let mut transition_inputs = BTreeMap::new();
        transition_inputs.insert(addr_a, (1u32, req_a));
        transition_inputs.insert(addr_b, (1u32, req_b));
        let transition = shield_transition_with_inputs(transition_inputs, shield_amount);

        let mut remaining = BTreeMap::new();
        remaining.insert(addr_a, (1u32, actual_a - req_a));
        remaining.insert(addr_b, (1u32, actual_b - req_b));

        let adjusted =
            reallocate_inputs_for_shield_amount(&transition, remaining, shield_amount).unwrap();

        let (_n_a, adj_a) = adjusted.get(&addr_a).copied().unwrap();
        let (_n_b, adj_b) = adjusted.get(&addr_b).copied().unwrap();

        // addr_a fully consumed (100k), addr_b consumed 150k -> 50k left.
        assert_eq!(adj_a, 0);
        assert_eq!(adj_b, actual_b - 150_000);

        let total_debited = (actual_a - adj_a) + (actual_b - adj_b);
        assert_eq!(total_debited, shield_amount);
    }

    /// Missing transition input for an address present in remaining balances is an error.
    #[test]
    fn test_reallocation_missing_transition_input_errors() {
        let addr = PlatformAddress::P2pkh([0xAA; 20]);
        // Transition has NO inputs.
        let transition = shield_transition_with_inputs(BTreeMap::new(), 1000);

        let mut remaining = BTreeMap::new();
        remaining.insert(addr, (1u32, 5000));

        let result = reallocate_inputs_for_shield_amount(&transition, remaining, 1000);
        assert!(result.is_err(), "missing transition input must error");
    }
}
