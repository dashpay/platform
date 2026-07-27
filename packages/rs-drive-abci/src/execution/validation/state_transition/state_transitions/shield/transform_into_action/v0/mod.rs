use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::shielded_common::read_pool_total_balance;
use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::shield::ShieldTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::{BTreeMap, BTreeSet};

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
/// We consume `shield_amount` across the inputs deterministically (so all nodes
/// agree), in two ordered passes:
///   1. inputs **not** referenced by the fee strategy, then
///   2. inputs the fee strategy will deduct the fee from,
///
/// each pass in `BTreeMap` (address) order. Consuming the fee-payer input(s) last
/// leaves them with the maximum residue to cover the fee, so a client that did not
/// pre-reserve fee headroom on its fee-strategy input is far less likely to be
/// spuriously rejected for insufficient funds. This ordering does **not** affect
/// conservation — the total consumed is still exactly `shield_amount`; it only
/// changes *which* addresses are debited.
///
/// Each input is debited at most its own `requested` (so no input is over-spent beyond
/// what its witness authorized): we consume `consumed = min(remaining_to_consume,
/// requested)` from it and leave the unconsumed `requested - consumed` in the address.
/// The structure validation has already enforced `Σrequested >= shield_amount`, so the
/// passes always consume the full `shield_amount`; the trailing check is a fail-closed
/// backstop (see `# Invariant` below).
///
/// The remaining `requested - consumed` per input is simply left in the source
/// address. The existing `PaidFromAddressInputs` fee machinery then deducts the fee
/// on top of these adjusted balances, so addresses lose exactly
/// `shield_amount + fee`, the pool gains `shield_amount`, and the fee pools gain
/// `fee` — credits are conserved.
///
/// # Invariant
///
/// Correctness depends on `Σrequested >= shield_amount`, enforced by the structure
/// validation in
/// `rs-dpp/src/state_transition/state_transitions/shielded/shield_transition/v0/state_transition_validation.rs`
/// (`ShieldTransitionV0::validate_structure`). The trailing `remaining_to_consume != 0` guard
/// below is the load-bearing defense if a future path ever reaches this transform without that
/// invariant — it would otherwise mint credits (pool credited `shield_amount`, addresses debited
/// less).
fn reallocate_inputs_for_shield_amount(
    transition: &ShieldTransitionV0,
    inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    shield_amount: Credits,
) -> Result<BTreeMap<PlatformAddress, (AddressNonce, Credits)>, Error> {
    let requested_inputs = &transition.inputs;
    let fee_strategy = &transition.fee_strategy;

    // Identify which input addresses the fee strategy deducts the fee from. A
    // `DeductFromInput(index)` step refers to the address at that position in the
    // input set's `BTreeMap` (address) order — the SAME order the fee machinery
    // (`deduct_fee_from_outputs_or_remaining_balance_of_inputs`) resolves the index
    // in, because this reallocation never adds or removes addresses. (`ReduceOutput`
    // is not applicable to shields, which have no outputs.)
    let input_addresses: Vec<PlatformAddress> =
        inputs_with_remaining_balance.keys().copied().collect();
    let mut fee_input_addresses: BTreeSet<PlatformAddress> = BTreeSet::new();
    for step in fee_strategy {
        if let AddressFundsFeeStrategyStep::DeductFromInput(index) = step {
            if let Some(addr) = input_addresses.get(*index as usize) {
                fee_input_addresses.insert(*addr);
            }
        }
    }

    let mut adjusted = BTreeMap::new();
    let mut remaining_to_consume = shield_amount;

    // Pass 1: non-fee-strategy inputs; Pass 2: fee-strategy inputs. Both in
    // BTreeMap (address) order — fully deterministic.
    //
    // NOTE: Pass 2 drains the fee inputs in BTreeMap (address) order, which does not follow the fee
    // strategy's own priority order. This only changes WHICH fee input keeps the leftover residue,
    // never whether the fee is covered: pass 1 consumes all non-fee inputs first, so the amount
    // drawn from the fee set is fixed at `shield_amount - Σ(non-fee requested)`, and the total
    // residue left across the fee inputs (`Σ(fee-input actual) - that amount`) is identical for any
    // intra-pass order. The fee machinery then deducts the fee from that residue in the strategy's
    // own priority order. So fee coverage is order-invariant — this ordering has no conservation and
    // no fee-coverage (spurious-rejection) effect.
    for consume_fee_inputs in [false, true] {
        for (addr, (nonce, remaining_after_full_debit)) in &inputs_with_remaining_balance {
            if fee_input_addresses.contains(addr) != consume_fee_inputs {
                continue;
            }

            // Look up the per-input max contribution (`requested`) from the transition.
            let (input_nonce, requested) = requested_inputs.get(addr).ok_or_else(|| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "shield input address present in remaining balances is missing from transition inputs",
                ))
            })?;
            // The shared address-balance validation keeps the transition-side nonce and the
            // remaining-balance nonce in lock-step; assert it so any future divergence (an upstream
            // bug or a new version dispatcher) surfaces in debug/test builds. Free in release, where
            // we proceed with the remaining-balance nonce as before.
            debug_assert_eq!(
                *input_nonce, *nonce,
                "shield input nonce mismatch between transition inputs and remaining balances"
            );
            let requested = *requested;

            // Consume up to this input's max contribution.
            let consumed = remaining_to_consume.min(requested);
            remaining_to_consume -= consumed;

            // Leave the unconsumed portion of this input's authorized contribution in the address.
            // `remaining_after_full_debit` already had the *full* `requested` debited, so add back the
            // part we did NOT consume: `new_remaining = remaining_after_full_debit + (requested -
            // consumed)`. Since `consumed = min(remaining_to_consume, requested) <= requested`, the
            // inner subtraction never underflows, and the result equals the pre-shield balance minus
            // `consumed` (<= the original balance), so the add cannot overflow. Both are checked
            // defensively in case of corrupted state.
            let unconsumed = requested.checked_sub(consumed).ok_or_else(|| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "consumed exceeds requested computing unconsumed shield contribution",
                ))
            })?;
            let new_remaining = remaining_after_full_debit
                .checked_add(unconsumed)
                .ok_or_else(|| {
                    Error::Execution(ExecutionError::Overflow(
                        "overflow leaving unconsumed shield contribution in remaining balance",
                    ))
                })?;

            adjusted.insert(*addr, (*nonce, new_remaining));
        }
    }

    // Structure validation enforces `Σrequested >= shield_amount` before this runs on the block and
    // FirstTimeCheck paths, so the passes consume the entire amount there. This is the fail-closed
    // backstop: a non-zero remainder would mean the pool is credited more than the addresses are
    // debited (minting), so we reject rather than mint. It is load-bearing on any path that reaches
    // this transform without basic-structure validation (e.g. the check_tx Recheck path, which is
    // mempool-only with no state commit).
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
        _block_info: &BlockInfo,
        _execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let ShieldTransition::V0(transition_v0) = self;
        let shield_amount: Credits = transition_v0.amount;

        // The shared address-balance validation debits the FULL per-input `requested`
        // (a max contribution), but the shielded pool only receives `shield_amount`.
        // Reallocate so the total debited across inputs equals exactly `shield_amount`,
        // leaving the excess in the source addresses. This keeps credits conserved
        // (addresses lose `shield_amount` + fee, pool gains `shield_amount`).
        let inputs_with_remaining_balance = reallocate_inputs_for_shield_amount(
            transition_v0,
            inputs_with_remaining_balance,
            shield_amount,
        )?;

        // Read current shielded pool state from GroveDB.
        //
        // Shield is metered + compute: GroveDB meters the real storage and processing of the
        // note/nullifier writes, and the execution-event layer adds the shielded COMPUTE fee
        // `compute_shielded_verification_fee(num_actions)` (proof verification + per-action processing)
        // on top as `additional_fixed_fee_cost` (see `ExecutionEvent` construction). These pool
        // reads flow through the standard metering with the rest of the operations.
        let mut drive_operations = vec![];
        let current_total_balance =
            read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;

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

    /// Builds a minimal `ShieldTransitionV0` whose `inputs` map carries the per-input
    /// `(nonce, requested)` pairs the reallocation needs. All cryptographic/structural
    /// fields are dummy values — they are irrelevant to the reallocation logic.
    fn shield_transition_with_inputs(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        amount: Credits,
    ) -> ShieldTransitionV0 {
        ShieldTransitionV0 {
            inputs,
            actions: vec![],
            amount,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
            fee_strategy: vec![],
            user_fee_increase: 0,
            input_witnesses: vec![],
        }
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

    /// Builds a `ShieldTransitionV0` with an explicit fee strategy (the default
    /// fixture uses an empty strategy).
    fn shield_transition_with_fee_strategy(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        amount: Credits,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
    ) -> ShieldTransitionV0 {
        ShieldTransitionV0 {
            inputs,
            actions: vec![],
            amount,
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
            fee_strategy,
            user_fee_increase: 0,
            input_witnesses: vec![],
        }
    }

    /// The reallocation consumes the shield amount from NON-fee-strategy inputs
    /// first, so the fee-payer input keeps maximum residue to cover the fee. Here
    /// input 0 (the fee-strategy input) has `actual == requested == shield_amount`:
    /// the naive "lowest address first" greedy would drain it to 0 (leaving nothing
    /// for the fee), but the fee-aware ordering funds the whole amount from input 1
    /// and leaves input 0 fully intact. Conservation is preserved.
    #[test]
    fn test_reallocation_consumes_non_fee_inputs_first() {
        let addr_a = PlatformAddress::P2pkh([0x01; 20]); // index 0 = fee input
        let addr_b = PlatformAddress::P2pkh([0x02; 20]); // index 1 = non-fee

        let shield_amount: Credits = 500_000;

        // A: actual == requested == shield_amount (the naive greedy would drain it).
        let (req_a, actual_a): (Credits, Credits) = (500_000, 500_000);
        // B: plenty of headroom.
        let (req_b, actual_b): (Credits, Credits) = (500_000, 1_000_000);

        let mut transition_inputs = BTreeMap::new();
        transition_inputs.insert(addr_a, (1u32, req_a));
        transition_inputs.insert(addr_b, (1u32, req_b));
        // Fee strategy deducts from input 0 (addr_a, the lowest address).
        let transition = shield_transition_with_fee_strategy(
            transition_inputs,
            shield_amount,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
        );

        let mut remaining = BTreeMap::new();
        remaining.insert(addr_a, (1u32, actual_a - req_a)); // 0
        remaining.insert(addr_b, (1u32, actual_b - req_b)); // 500_000

        let adjusted =
            reallocate_inputs_for_shield_amount(&transition, remaining, shield_amount).unwrap();

        let (_n_a, adj_a) = adjusted.get(&addr_a).copied().unwrap();
        let (_n_b, adj_b) = adjusted.get(&addr_b).copied().unwrap();

        // The fee-payer input (A) is left fully intact — the amount came from B.
        assert_eq!(
            adj_a, actual_a,
            "fee-strategy input must NOT be drained when a non-fee input can cover the amount"
        );
        // Input B funded the whole shield amount.
        assert_eq!(adj_b, actual_b - shield_amount);

        // Conservation: total debited == shield_amount.
        assert_eq!((actual_a - adj_a) + (actual_b - adj_b), shield_amount);
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

    /// The anti-mint backstop: if `shield_amount` exceeds the sum of input contributions
    /// (`Σrequested`) — only possible if the structure-validation invariant is bypassed — the
    /// reallocation rejects rather than minting credits (it would otherwise credit the pool more
    /// than the addresses are debited). Directly covers the trailing `remaining_to_consume != 0`
    /// guard, which the end-to-end tests never reach (structure validation rejects first).
    #[test]
    fn test_reallocation_rejects_when_shield_amount_exceeds_input_sum() {
        let addr = PlatformAddress::P2pkh([0xAA; 20]);
        let requested: Credits = 1_000;
        let shield_amount: Credits = 5_000; // > Σrequested = 1_000

        let mut transition_inputs = BTreeMap::new();
        transition_inputs.insert(addr, (1u32, requested));
        let transition = shield_transition_with_inputs(transition_inputs, shield_amount);

        let mut remaining = BTreeMap::new();
        remaining.insert(addr, (1u32, 0)); // actual - requested

        let err = reallocate_inputs_for_shield_amount(&transition, remaining, shield_amount)
            .expect_err("shield_amount > Σrequested must be rejected, not minted");
        match err {
            Error::Execution(ExecutionError::CorruptedCodeExecution(msg)) => assert!(
                msg.contains("exceeds the sum of input contributions"),
                "unexpected error message: {msg}"
            ),
            other => panic!("expected the CorruptedCodeExecution anti-mint guard, got {other:?}"),
        }
    }

    /// In debug builds, a nonce mismatch between the transition inputs and the remaining-balance
    /// map trips the lock-step `debug_assert_eq!`. Free in release (the assert compiles out and the
    /// remaining-balance nonce is used); this exercises the failing direction so the contract is
    /// guarded in CI.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "shield input nonce mismatch")]
    fn test_reallocation_diverging_nonce_trips_debug_assert() {
        let addr = PlatformAddress::P2pkh([0xAA; 20]);
        let mut transition_inputs = BTreeMap::new();
        transition_inputs.insert(addr, (1u32, 1_000)); // transition-side nonce = 1
        let transition = shield_transition_with_inputs(transition_inputs, 500);

        let mut remaining = BTreeMap::new();
        remaining.insert(addr, (2u32, 0)); // remaining-balance nonce = 2 (diverges)

        let _ = reallocate_inputs_for_shield_amount(&transition, remaining, 500);
    }
}
