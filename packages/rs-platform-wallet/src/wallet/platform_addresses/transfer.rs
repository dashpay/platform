use std::collections::BTreeMap;

use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::version::PlatformVersion;
use dpp::version::LATEST_PLATFORM_VERSION;
use key_wallet::PlatformP2PKHAddress;

use crate::wallet::PlatformAddressWallet;
use crate::{PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::transition::transfer_address_funds::TransferAddressFunds;

pub use super::InputSelection;

impl PlatformAddressWallet {
    /// Transfer credits between platform addresses.
    ///
    /// Input addresses can be specified explicitly or selected automatically
    /// from the account via [`InputSelection::Auto`].
    ///
    /// If `platform_version` is `None`, the latest platform version's fee
    /// schedule is used for fee estimation during auto-selection.
    ///
    /// `address_signer` produces ECDSA signatures for the input
    /// [`PlatformAddress`]es. The wallet struct itself carries no key
    /// material — callers supply a seed-backed, hardware, or
    /// FFI-trampoline signer per their environment (iOS routes through
    /// `KeychainSigner` via `VTableSigner`).
    pub async fn transfer<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        account_index: u32,
        input_selection: InputSelection,
        outputs: BTreeMap<PlatformAddress, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        platform_version: Option<&PlatformVersion>,
        address_signer: &S,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError> {
        if outputs.is_empty() {
            return Err(PlatformWalletError::AddressOperation(
                "Transfer requires at least one output address".to_string(),
            ));
        }

        let version = platform_version.unwrap_or(LATEST_PLATFORM_VERSION);

        let address_infos = match input_selection {
            InputSelection::Explicit(inputs) => {
                if inputs.is_empty() {
                    return Err(PlatformWalletError::AddressOperation(
                        "Transfer requires at least one input address".to_string(),
                    ));
                }
                self.sdk
                    .transfer_address_funds(inputs, outputs, fee_strategy, address_signer, None)
                    .await?
            }
            InputSelection::ExplicitWithNonces(inputs) => {
                if inputs.is_empty() {
                    return Err(PlatformWalletError::AddressOperation(
                        "Transfer requires at least one input address".to_string(),
                    ));
                }
                self.sdk
                    .transfer_address_funds_with_nonce(
                        inputs,
                        outputs,
                        fee_strategy,
                        address_signer,
                        None,
                    )
                    .await?
            }
            InputSelection::Auto => {
                // Auto-select currently only implements the protocol-correct
                // distribution (Phase 1-4 in `select_inputs`) for a single
                // `[DeductFromInput(0)]` step. Other shapes — `DeductFromInput(N>0)`,
                // `ReduceOutput`, multi-step — are not yet wired through that
                // path; reject early and steer the caller toward `Explicit`.
                if !matches!(
                    fee_strategy.as_slice(),
                    [AddressFundsFeeStrategyStep::DeductFromInput(0)]
                ) {
                    return Err(PlatformWalletError::AddressOperation(
                        "InputSelection::Auto currently only supports fee_strategy = \
                         [DeductFromInput(0)]; for other strategies use InputSelection::Explicit"
                            .to_string(),
                    ));
                }
                let inputs = self
                    .auto_select_inputs(account_index, &outputs, &fee_strategy, version)
                    .await?;
                self.sdk
                    .transfer_address_funds(inputs, outputs, fee_strategy, address_signer, None)
                    .await?
            }
        };

        // Get the cached key source from the unified provider for gap
        // limit maintenance.
        let key_source = {
            let guard = self.provider.read().await;
            guard
                .as_ref()
                .and_then(|p| p.key_source(&self.wallet_id, account_index))
        };

        // Update balances in the ManagedPlatformAccount.
        let mut wm = self.wallet_manager.write().await;
        let mut cs = PlatformAddressChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            if let Some(account) = info
                .core_wallet
                .platform_payment_managed_account_at_index_mut(account_index)
            {
                for (addr, maybe_info) in address_infos.iter() {
                    let PlatformAddress::P2pkh(hash) = addr else {
                        continue;
                    };
                    let p2pkh = PlatformP2PKHAddress::new(*hash);
                    let funds = match maybe_info {
                        Some(ai) => dash_sdk::platform::address_sync::AddressFunds {
                            balance: ai.balance,
                            nonce: ai.nonce,
                        },
                        None => dash_sdk::platform::address_sync::AddressFunds {
                            balance: 0,
                            nonce: 0,
                        },
                    };
                    account.set_address_credit_balance(p2pkh, funds.balance, key_source.as_ref());
                    let address_index = account
                        .addresses
                        .addresses
                        .iter()
                        .find_map(|(&idx, info)| {
                            PlatformP2PKHAddress::from_address(&info.address)
                                .ok()
                                .filter(|found| *found == p2pkh)
                                .map(|_| idx)
                        })
                        .unwrap_or(0);
                    cs.addresses.push(crate::PlatformAddressBalanceEntry {
                        wallet_id: self.wallet_id,
                        account_index,
                        address_index,
                        address: p2pkh,
                        funds,
                    });
                }
            }
        }

        Ok(cs)
    }

    /// Automatically select input addresses from the account,
    /// consuming candidates in **balance-descending order** until
    /// the total output amount plus the estimated input-side fee
    /// margin is covered.
    ///
    /// Sorting candidates largest-balance-first mirrors the
    /// dash-evo-tool allocator
    /// (`src/ui/wallets/send_screen.rs:155-157`) and minimises the
    /// number of inputs picked: when the largest single balance
    /// already covers `total_output + estimated_fee`, the result
    /// is a 1-input map and the multi-input fee-headroom logic in
    /// [`select_inputs`] never fires. For the multi-input case
    /// (largest balance alone insufficient), `select_inputs` still
    /// applies the headroom-respecting distribution introduced in
    /// 9ea9e7033c — this sort change only narrows the set of
    /// scenarios that reach that branch.
    ///
    /// The selected map's values are the **consumed amount per
    /// address** (what gets moved into outputs) — not the address
    /// balance. The protocol validates `Σ inputs.credits ==
    /// Σ outputs.credits`; the fee is then deducted from one input
    /// address's REMAINING balance per [`AddressFundsFeeStrategy`]
    /// (e.g. `DeductFromInput(0)` reduces the balance left at
    /// input #0 by the fee, rather than reducing input #0's
    /// `Credits` value). For the wallet, this means we only need
    /// each input address to hold `consumed + fee_share`; the
    /// `Credits` we hand to the SDK is just the consumed amount.
    async fn auto_select_inputs(
        &self,
        account_index: u32,
        outputs: &BTreeMap<PlatformAddress, Credits>,
        fee_strategy: &[AddressFundsFeeStrategyStep],
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
        let total_output: Credits = outputs.values().sum();

        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found in wallet manager",
                hex::encode(self.wallet_id)
            ))
        })?;

        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index(account_index)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account at index {}",
                    account_index
                ))
            })?;

        let min_input_amount = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount;

        // Snapshot addresses with balance ≥ `min_input_amount`, then sort
        // them by balance descending so [`select_inputs`] sees the
        // largest candidates first. Mirrors the dash-evo-tool allocator
        // (`src/ui/wallets/send_screen.rs:155-157`) and means the
        // common case — one address holds enough to cover
        // `total_output + estimated_fee` — bypasses the multi-input
        // fee-headroom branch entirely. Addresses with balance below
        // `min_input_amount` are filtered out: the protocol's
        // structural validator (`AddressFundsTransferTransitionV0::
        // validate_structure`, see `state_transition_validation.rs:146`)
        // rejects any input with `amount < min_input_amount`, so such
        // an address cannot legally appear in the inputs map and is
        // useless as a standalone candidate. Materialising a `Vec`
        // here also lets the selection loop run as a pure helper that's
        // amenable to direct unit testing.
        let mut candidates: Vec<(PlatformAddress, Credits)> = account
            .addresses
            .addresses
            .values()
            .filter_map(|addr_info| {
                let p2pkh = PlatformP2PKHAddress::from_address(&addr_info.address).ok()?;
                let balance = account.address_credit_balance(&p2pkh);
                if balance < min_input_amount {
                    None
                } else {
                    Some((PlatformAddress::P2pkh(p2pkh.to_bytes()), balance))
                }
            })
            .collect();
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        select_inputs(
            candidates,
            outputs,
            total_output,
            fee_strategy,
            platform_version,
        )
    }

    /// Simulate the fee strategy to determine how much additional balance
    /// the inputs need beyond the output amounts.
    ///
    /// Re-exposed at module scope via [`estimate_fee_for_inputs_pub`]
    /// so [`select_inputs`] (the pure helper) can drive the same
    /// estimator without going through `Self`.
    ///
    /// Walks through the fee strategy steps in order, deducting from the
    /// available sources (outputs or inputs) until the fee is covered.
    /// Returns the portion of the fee that must come from inputs.
    fn estimate_fee_for_inputs(
        input_count: usize,
        output_count: usize,
        fee_strategy: &[AddressFundsFeeStrategyStep],
        outputs: &BTreeMap<PlatformAddress, Credits>,
        platform_version: &PlatformVersion,
    ) -> Credits {
        let total_fee = AddressFundsTransferTransition::estimate_min_fee(
            input_count,
            output_count,
            platform_version,
        );

        let mut remaining_fee = total_fee;
        let output_amounts: Vec<Credits> = outputs.values().copied().collect();

        for step in fee_strategy {
            if remaining_fee == 0 {
                break;
            }
            match step {
                AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                    // This output absorbs part of the fee — inputs don't need to cover it.
                    if let Some(&amount) = output_amounts.get(*index as usize) {
                        let reduction = remaining_fee.min(amount);
                        remaining_fee -= reduction;
                    }
                }
                AddressFundsFeeStrategyStep::DeductFromInput(_) => {
                    // Inputs will cover whatever fee remains at this step.
                    // We don't reduce remaining_fee here because we're
                    // computing the total that inputs must cover — this
                    // step confirms inputs pay, but the actual deduction
                    // happens on-chain from whichever input is specified.
                    break;
                }
            }
        }

        // Whatever fee wasn't covered by reducing outputs must come from inputs.
        remaining_fee
    }
}

/// Module-scope re-export of the per-input fee estimator so the
/// pure [`select_inputs`] helper can be unit-tested without an
/// instance of [`PlatformAddressWallet`].
fn estimate_fee_for_inputs_pub(
    input_count: usize,
    output_count: usize,
    fee_strategy: &[AddressFundsFeeStrategyStep],
    outputs: &BTreeMap<PlatformAddress, Credits>,
    platform_version: &PlatformVersion,
) -> Credits {
    PlatformAddressWallet::estimate_fee_for_inputs(
        input_count,
        output_count,
        fee_strategy,
        outputs,
        platform_version,
    )
}

/// Pure input-selection helper.
///
/// Given a `candidates` list of `(address, balance)` pairs in the
/// caller's preferred selection order (balance-descending in
/// practice — see [`PlatformAddressWallet::auto_select_inputs`] —
/// but `select_inputs` itself is order-agnostic: it walks
/// `candidates` as-is and picks the smallest covering prefix),
/// produce an inputs map satisfying TWO invariants demanded by the
/// validator:
///
/// 1. `Σ selected.values() == total_output` — the protocol's
///    structural balance invariant for transfers.
/// 2. The address selected for fee deduction (currently the
///    lex-smallest address in `selected`, which is the
///    `BTreeMap` index-0 entry that
///    [`AddressFundsFeeStrategyStep::DeductFromInput(0)`] targets)
///    must have **post-consumption remaining balance ≥ estimated
///    fee**. Otherwise drive's
///    `deduct_fee_from_outputs_or_remaining_balance_of_inputs`
///    cannot fully cover the fee, the transition fails with
///    `fee_fully_covered = false`, and validation rejects the
///    state transition (see
///    `rs-drive-abci/.../validate_fees_of_event/v0/mod.rs:209-224`).
///
/// CodeRabbit caught the bug where the previous implementation
/// satisfied invariant (1) but not (2): if candidates were
/// `[(addr_a, 20M), (addr_b, 50M)]`, `total_output` was 30M, and the
/// strategy was `[DeductFromInput(0)]`, the previous build returned
/// `{addr_a: 20M, addr_b: 10M}`. `addr_a` was fully drained, so its
/// post-consumption remaining was 0 — the fee couldn't be deducted,
/// and the transition was rejected. This rewrite ensures the fee
/// target keeps enough headroom by consuming the **minimum
/// allowable** amount (`min_input_amount` from the platform version)
/// from it, and shifting the rest of the consumption onto the other
/// selected inputs.
///
/// # Algorithm (single `DeductFromInput(0)` strategy — the only supported case)
///
/// 1. Pick the smallest prefix of `candidates` (in the order the
///    caller supplied — balance-descending in practice) such that
///    `Σ balances ≥ total_output + estimated_fee_for(prefix.len())`.
///    Error out if no prefix covers it.
/// 2. Identify the prospective fee target = lex-smallest address in
///    that prefix (this is the address at `BTreeMap` index 0 of the
///    eventual selected map, which is what `DeductFromInput(0)`
///    targets).
/// 3. Pick the consumption distribution:
///    - `fee_target_max  = max(0, fee_target_balance − estimated_fee)`
///      — the largest amount we can consume from the fee target
///      while still leaving ≥ `estimated_fee` of remaining balance.
///    - `other_total     = Σ balances of non-fee-target prefix entries`
///    - `fee_target_min  = max(min_input_amount, total_output − other_total)`
///      — the smallest amount we can consume from the fee target
///      while still keeping it in the inputs map (`min_input_amount`,
///      so the protocol's per-input minimum is respected) AND
///      reaching the `Σ inputs == total_output` invariant.
///    - If `fee_target_min > fee_target_max`, **extend the prefix
///      with the next candidate and retry steps 1-3**. A larger
///      prefix can lower `fee_target_min` (more `other_total` to
///      absorb consumption) and may also pull in a smaller
///      lex-key candidate that becomes the new fee target. Only
///      after candidates are exhausted do we error out.
/// 4. Build the result:
///    - Insert `(fee_target_addr, fee_target_min)` first
///      (always ≥ `min_input_amount`, so always present in the map
///      and lex-smallest of the result).
///    - Distribute `total_output − fee_target_min` across the other
///      prefix entries in caller-supplied order
///      (`min(balance, remaining)`). If a tail entry's tentative
///      consumption falls below `min_input_amount` (the protocol's
///      per-input minimum), the residue is rolled back into the
///      fee target's consumption rather than inserted as a
///      sub-minimum input. After roll-back the fee target's
///      consumption must still be ≤ `fee_target_max`; otherwise
///      we error out (this should not happen given that Phase 3
///      already proved the prefix has slack, but the check is
///      kept as a defensive guard).
/// 5. Final defensive invariant check.
///
/// `select_inputs` only supports `fee_strategy == [DeductFromInput(0)]`.
/// The public `transfer()` rejects other shapes for the
/// `InputSelection::Auto` path before they reach this helper.
fn select_inputs(
    candidates: Vec<(PlatformAddress, Credits)>,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    total_output: Credits,
    fee_strategy: &[AddressFundsFeeStrategyStep],
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    debug_assert!(
        matches!(
            fee_strategy,
            [AddressFundsFeeStrategyStep::DeductFromInput(0)]
        ),
        "select_inputs only supports [DeductFromInput(0)]; \
         the public `transfer()` should have validated this already"
    );
    if !matches!(
        fee_strategy,
        [AddressFundsFeeStrategyStep::DeductFromInput(0)]
    ) {
        return Err(PlatformWalletError::AddressOperation(
            "select_inputs only supports fee_strategy = [DeductFromInput(0)]; \
             other shapes must use InputSelection::Explicit"
                .to_string(),
        ));
    }

    let output_count = outputs.len();
    let min_input_amount = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_input_amount;

    // Finding #4: the protocol rejects any input below `min_input_amount`,
    // and an input always covers (a portion of) `total_output`. So if
    // `total_output < min_input_amount`, no input can be sized within
    // both bounds simultaneously — error out cleanly here rather than
    // tripping the per-input minimum check downstream.
    if total_output < min_input_amount {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Transfer amount {} is below the protocol minimum input amount {}; \
             a transfer cannot be split across inputs in a way that satisfies \
             the per-input minimum",
            total_output, min_input_amount,
        )));
    }

    // Phase 1+2+3: walk candidates in caller-supplied order, growing
    // the prefix one candidate at a time. After each push, re-run
    // Phase 1 (does the prefix cover `total_output + estimated_fee`?)
    // and, if so, Phase 2/3 (does the lex-smallest prefix entry have
    // enough headroom to absorb the fee?). Either accept the prefix
    // or extend further. Errors out only when candidates are
    // exhausted with no feasible prefix.
    let mut prefix: Vec<(PlatformAddress, Credits)> = Vec::new();
    let mut accumulated: Credits = 0;
    let mut last_estimated_fee: Credits = 0;
    let mut feasible: Option<(PlatformAddress, Credits, Credits, Credits)> = None;

    for (address, balance) in candidates {
        prefix.push((address, balance));
        accumulated = accumulated.saturating_add(balance);

        let estimated_fee = estimate_fee_for_inputs_pub(
            prefix.len(),
            output_count,
            fee_strategy,
            outputs,
            platform_version,
        );
        last_estimated_fee = estimated_fee;
        let required = total_output.saturating_add(estimated_fee);

        if accumulated < required {
            continue;
        }

        // Phase 2: lex-smallest of the current prefix is the fee target.
        let (fee_target_addr, fee_target_balance) = prefix
            .iter()
            .min_by_key(|(addr, _)| *addr)
            .copied()
            .expect("prefix is non-empty: we just pushed");

        let fee_target_max = fee_target_balance.saturating_sub(estimated_fee);
        let other_total: Credits = prefix
            .iter()
            .filter(|(addr, _)| addr != &fee_target_addr)
            .map(|(_, bal)| *bal)
            .sum();
        let fee_target_min =
            std::cmp::max(min_input_amount, total_output.saturating_sub(other_total));

        if fee_target_min <= fee_target_max {
            feasible = Some((
                fee_target_addr,
                fee_target_balance,
                fee_target_min,
                estimated_fee,
            ));
            break;
        }
        // Phase 3 failed for this prefix size: keep growing.
    }

    let Some((fee_target_addr, fee_target_balance, fee_target_min, estimated_fee)) = feasible
    else {
        // Distinguish "couldn't cover total_output + fee" from
        // "covered but no headroom-feasible fee target".
        if accumulated < total_output.saturating_add(last_estimated_fee) {
            return Err(PlatformWalletError::AddressOperation(format!(
                "Insufficient balance: available {} credits, required {} \
                 (outputs {} + estimated fee {})",
                accumulated,
                total_output.saturating_add(last_estimated_fee),
                total_output,
                last_estimated_fee,
            )));
        }
        return Err(PlatformWalletError::AddressOperation(format!(
            "Cannot satisfy fee headroom: no covering prefix of the available inputs \
             leaves the lex-smallest entry with ≥ estimated fee {} of remaining balance \
             after consumption. Consider providing more inputs or using a different \
             fee strategy.",
            last_estimated_fee,
        )));
    };

    // Phase 4: build the result map.
    //
    // Start by consuming the minimum from the fee target so it
    // retains maximum remaining balance for the on-chain fee
    // deduction. Then walk the remaining prefix entries (in
    // caller-supplied order) and distribute what's left of
    // `total_output`. If a tail entry's tentative consumption is
    // below `min_input_amount`, roll the residue back onto the
    // fee target instead of producing a sub-minimum input —
    // the protocol's `validate_structure` would reject the
    // transition otherwise (`InputBelowMinimumError`).
    let mut fee_target_consumed = fee_target_min;
    let fee_target_max = fee_target_balance.saturating_sub(estimated_fee);
    let mut selected: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();

    let mut remaining = total_output.saturating_sub(fee_target_consumed);
    let mut residue_to_fee_target: Credits = 0;
    for (addr, bal) in prefix.iter() {
        if *addr == fee_target_addr {
            continue;
        }
        if remaining == 0 {
            break;
        }
        let tentative = (*bal).min(remaining);
        if tentative == 0 {
            continue;
        }
        if tentative < min_input_amount {
            // Sub-minimum input — fold into the fee target.
            residue_to_fee_target = residue_to_fee_target.saturating_add(tentative);
            remaining = remaining.saturating_sub(tentative);
            continue;
        }
        selected.insert(*addr, tentative);
        remaining = remaining.saturating_sub(tentative);
    }

    if residue_to_fee_target > 0 {
        let new_consumed = fee_target_consumed.saturating_add(residue_to_fee_target);
        if new_consumed > fee_target_max {
            // Should be unreachable: Phase 3 only accepts a prefix
            // when fee_target_min ≤ fee_target_max, and the residue
            // we're folding here represents amounts that *would*
            // have been consumed by other entries — the prefix
            // covers `total_output + estimated_fee`, so the fee
            // target's headroom up to `fee_target_max` should
            // accommodate any residue from the tail. We still
            // guard against it because silently producing an
            // invalid transition is worse than a loud error.
            return Err(PlatformWalletError::AddressOperation(format!(
                "Cannot satisfy fee headroom after redistributing sub-minimum tail \
                 inputs: fee-target {} would consume {} (balance {}, max {}), leaving \
                 less than estimated fee {} of remaining balance",
                format_address(&fee_target_addr),
                new_consumed,
                fee_target_balance,
                fee_target_max,
                estimated_fee,
            )));
        }
        fee_target_consumed = new_consumed;
    }

    selected.insert(fee_target_addr, fee_target_consumed);

    // Phase 5: defensive invariant checks. These should never trip
    // if Phase 1+3+4 are correct, but we'd much rather fail loudly
    // here than ship a transition the validator silently rejects.
    let input_sum: Credits = selected.values().sum();
    debug_assert_eq!(input_sum, total_output, "Σ inputs == Σ outputs invariant");
    debug_assert_eq!(
        selected.keys().next().copied(),
        Some(fee_target_addr),
        "fee target must be the BTreeMap index-0 (lex-smallest) entry"
    );
    debug_assert!(
        fee_target_balance.saturating_sub(fee_target_consumed) >= estimated_fee,
        "fee target must retain ≥ estimated_fee remaining balance for DeductFromInput(0)"
    );
    debug_assert!(
        selected.values().all(|amount| *amount >= min_input_amount),
        "every selected input must satisfy the protocol's per-input minimum"
    );

    if input_sum != total_output {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Internal selection error: Σ inputs ({}) != total_output ({})",
            input_sum, total_output
        )));
    }

    Ok(selected)
}

fn format_address(addr: &PlatformAddress) -> String {
    match addr {
        PlatformAddress::P2pkh(hash) => format!("p2pkh({})", hex::encode(hash)),
        PlatformAddress::P2sh(hash) => format!("p2sh({})", hex::encode(hash)),
    }
}

#[cfg(test)]
mod auto_select_tests {
    use super::*;
    use dpp::address_funds::AddressWitness;
    use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use dpp::state_transition::StateTransitionStructureValidation;

    fn p2pkh(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    fn outputs_for(target: PlatformAddress, amount: Credits) -> BTreeMap<PlatformAddress, Credits> {
        std::iter::once((target, amount)).collect()
    }

    /// Build a minimal valid `AddressFundsTransferTransitionV0` from a
    /// selector result and feed it to the protocol's pure
    /// `validate_structure` validator. Mirrors the shape used by
    /// `valid_transfer_transition()` in
    /// `state_transition_validation.rs:237`. Uses zero nonces and
    /// dummy P2PKH witnesses — the structural validator doesn't
    /// inspect signature material, only counts.
    fn assert_selection_validates(
        selected: &BTreeMap<PlatformAddress, Credits>,
        outputs: &BTreeMap<PlatformAddress, Credits>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
        platform_version: &PlatformVersion,
    ) {
        let inputs = selected
            .iter()
            .map(|(addr, amount)| (*addr, (0u32, *amount)))
            .collect();
        let input_witnesses = (0..selected.len())
            .map(|_| AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            })
            .collect();
        let transition = AddressFundsTransferTransitionV0 {
            inputs,
            outputs: outputs.clone(),
            fee_strategy,
            user_fee_increase: 0,
            input_witnesses,
        };
        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "validate_structure rejected the selection: {:?}",
            result.errors,
        );
    }

    /// Regression test for the bug surfaced by Wave 8's live
    /// testnet run: a wallet with one address holding 100M credits,
    /// asked for an output of 10M, must produce
    /// `selected[addr] == 10M` (the consumed amount) — NOT
    /// `100M` (the full balance) and NOT `10M + fee`. The fee
    /// comes from the address's REMAINING balance via the
    /// `DeductFromInput(0)` strategy; it's never part of the
    /// inputs map's `Credits` value.
    ///
    /// The validator asserts `Σ inputs == Σ outputs` (verified
    /// at `rs-dpp/.../address_funds_transfer_transition/v0/state_transition_validation.rs`)
    /// and the on-chain test
    /// (`rs-drive-abci/.../address_funds_transfer/tests.rs:test_input_balance_decreased_correctly`)
    /// confirms `new_balance == initial_balance - transfer_amount - fee`,
    /// i.e. the fee is deducted from the address balance separately
    /// from the input.credits value.
    #[test]
    fn single_input_oversized_balance_trims_to_output_amount() {
        let addr = p2pkh(0x11);
        let target = p2pkh(0x22);
        let outputs = outputs_for(target, 10_000_000);
        let total_output = 10_000_000u64;
        let candidates = vec![(addr, 100_000_000u64)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv)
            .expect("selection");

        assert_eq!(
            selected.get(&addr),
            Some(&10_000_000),
            "consumed amount must equal total_output (NOT full balance, NOT total_output + fee)"
        );
        let input_sum: Credits = selected.values().sum();
        let output_sum: Credits = outputs.values().sum();
        assert_eq!(
            input_sum, output_sum,
            "Σ inputs must equal Σ outputs (protocol's structural invariant)"
        );

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// When the first selected address can't cover `output + fee`
    /// alone but two inputs together can, the **fee target** (the
    /// lex-smallest address, which `DeductFromInput(0)` will hit)
    /// must keep enough remaining balance to cover the fee. So the
    /// fee target consumes only `min_input_amount`, and the rest of
    /// `total_output` is drawn from the other selected input(s).
    ///
    /// CodeRabbit caught the previous, broken behaviour where
    /// `addr_a` was drained in full (`{addr_a: 20M, addr_b: 10M}`),
    /// leaving zero remaining balance for fee deduction at index 0.
    #[test]
    fn two_input_selection_keeps_fee_headroom_at_index_zero() {
        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0x99);
        let total_output = 30_000_000u64;
        let outputs = outputs_for(target, total_output);
        let addr_a_balance = 20_000_000u64;
        let addr_b_balance = 50_000_000u64;
        let candidates = vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv)
            .expect("selection");

        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // Fee target consumes the minimum; the remainder is shifted
        // onto addr_b.
        assert_eq!(selected.get(&addr_a), Some(&min_input));
        assert_eq!(selected.get(&addr_b), Some(&(total_output - min_input)));

        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output);

        // addr_a is the BTreeMap index-0 entry (lex-smallest), so
        // `DeductFromInput(0)` will deduct from its remaining
        // balance.
        assert_eq!(selected.keys().next(), Some(&addr_a));

        // Headroom invariant: addr_a's post-consumption remaining
        // (= balance − consumed) must be ≥ estimated fee.
        let estimated_fee =
            estimate_fee_for_inputs_pub(selected.len(), outputs.len(), &fee_strategy, &outputs, pv);
        let remaining = addr_a_balance - selected[&addr_a];
        assert!(
            remaining >= estimated_fee,
            "fee target remaining {} must be ≥ estimated fee {}",
            remaining,
            estimated_fee,
        );

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Inputs are insufficient → error path returns a descriptive
    /// `AddressOperation` error with the required-vs-available
    /// numbers.
    #[test]
    fn insufficient_balance_errors() {
        let addr = p2pkh(0x33);
        let target = p2pkh(0x44);
        let total_output = 100_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr, 5_000_000)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let err = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv)
            .expect_err("expected insufficient-balance error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("Insufficient balance"),
                    "expected 'Insufficient balance' in error, got {msg:?}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// Two-input scenario where the first candidate alone is
    /// nearly enough to cover `total_output`, but cannot cover
    /// `total_output + fee` (so a second input is added). The new
    /// algorithm always shifts consumption to the non-fee-target
    /// inputs to keep the fee-target's remaining balance for the
    /// fee. The map's `Σ values` must still equal `total_output`.
    #[test]
    fn fee_only_tail_input_does_not_inflate_input_sum() {
        let addr_a = p2pkh(0xA0);
        let addr_b = p2pkh(0xB0);
        let target = p2pkh(0xCC);
        let total_output = 1_000_000_000u64;
        let outputs = outputs_for(target, total_output);
        let addr_a_balance = total_output + 1;
        let addr_b_balance = total_output;
        let candidates = vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv)
            .expect("selection");

        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let input_sum: Credits = selected.values().sum();
        assert_eq!(
            input_sum, total_output,
            "Σ inputs must equal Σ outputs (protocol's structural invariant)"
        );

        // addr_a (lex-smallest) is the fee target. With the new
        // algorithm it consumes min_input_amount; addr_b absorbs
        // the rest of `total_output`.
        assert_eq!(selected.get(&addr_a), Some(&min_input));
        assert_eq!(selected.get(&addr_b), Some(&(total_output - min_input)));
        // addr_a stays at BTreeMap index 0.
        assert_eq!(selected.keys().next(), Some(&addr_a));

        // Headroom invariant.
        let estimated_fee =
            estimate_fee_for_inputs_pub(selected.len(), outputs.len(), &fee_strategy, &outputs, pv);
        assert!(
            addr_a_balance - selected[&addr_a] >= estimated_fee,
            "fee target must retain ≥ estimated_fee for DeductFromInput(0)"
        );

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Direct regression test for the bug CodeRabbit flagged on
    /// PR #3554: the old `select_inputs` returned
    /// `{addr_a: 20M, addr_b: 10M}` for this exact scenario. That
    /// satisfied `Σ inputs == Σ outputs` but drained `addr_a`
    /// completely, so when drive applied `DeductFromInput(0)` it
    /// found `min(fee, remaining=0) = 0` and rejected the
    /// transition with `AddressesNotEnoughFundsError`.
    ///
    /// The new algorithm must keep `addr_a` in the map at
    /// `min_input_amount` and shift the remaining consumption
    /// onto `addr_b`, leaving `addr_a` with enough balance left
    /// over to absorb the fee at deduction time.
    #[test]
    fn fee_target_keeps_remaining_for_fee_deduction() {
        // Address bytes are chosen so addr_a < addr_b
        // lexicographically (matching the BTreeMap ordering used
        // by `DeductFromInput(0)`).
        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0xFF);
        let total_output = 30_000_000u64;
        let outputs = outputs_for(target, total_output);
        let addr_a_balance = 20_000_000u64;
        let addr_b_balance = 50_000_000u64;
        let candidates = vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv)
            .expect("selection");

        // (1) Σ inputs == Σ outputs.
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output);

        // (2) Fee target stays in the map and is index-0.
        assert_eq!(
            selected.keys().next(),
            Some(&addr_a),
            "fee target (lex-smallest) must be the BTreeMap index-0 entry"
        );

        // (3) Fee target's post-consumption remaining ≥ estimated
        //     fee — THE invariant the bug violated.
        let estimated_fee =
            estimate_fee_for_inputs_pub(selected.len(), outputs.len(), &fee_strategy, &outputs, pv);
        let remaining = addr_a_balance - selected[&addr_a];
        assert!(
            remaining >= estimated_fee,
            "fee target remaining {} must be ≥ estimated fee {} (CodeRabbit regression)",
            remaining,
            estimated_fee,
        );

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Protocol-level reproduction of the CodeRabbit bug. Constructs the
    /// exact `inputs` map the pre-fix `select_inputs` would have returned
    /// for the original example (candidates (20M, 50M), total_output 30M,
    /// `DeductFromInput(0)`), feeds it through the live dpp fee-deduction
    /// code path, and asserts `fee_fully_covered == false` — i.e. the
    /// transition would have been rejected with `AddressesNotEnoughFundsError`.
    ///
    /// This is the smoking gun: not just a unit test of our selector, but
    /// proof that the unfixed selector's output is structurally invalid
    /// at the protocol layer (not merely "we agreed it should look
    /// different"). The fixed selector is verified independently by
    /// `fee_target_keeps_remaining_for_fee_deduction`.
    ///
    /// Reference:
    /// - dpp deduction:
    ///   `packages/rs-dpp/src/address_funds/fee_strategy/deduct_fee_from_inputs_and_outputs/v0/mod.rs`
    /// - drive enforcement:
    ///   `packages/rs-drive-abci/src/execution/platform_events/state_transition_processing/validate_fees_of_event/v0/mod.rs:209`
    ///   (rejects when `!fee_fully_covered`).
    #[test]
    fn pre_fix_buggy_selector_output_is_rejected_by_protocol_fee_deduction() {
        use dpp::address_funds::fee_strategy::deduct_fee_from_inputs_and_outputs::deduct_fee_from_outputs_or_remaining_balance_of_inputs;
        use dpp::prelude::AddressNonce;

        // CodeRabbit's example.
        let addr_a = p2pkh(0x01); // lex-smallest → DeductFromInput(0) target
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0xFF);
        let total_output = 30_000_000u64;
        let addr_a_balance = 20_000_000u64;
        let addr_b_balance = 50_000_000u64;
        let outputs = outputs_for(target, total_output);
        let fee_strategy: AddressFundsFeeStrategy =
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        // The OLD selector would produce: addr_a fully consumed (20M),
        // addr_b trimmed to 10M. Σ = 30M = total_output ✓ aggregate, but
        // addr_a is fully drained.
        let mut buggy_inputs_consumed: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        buggy_inputs_consumed.insert(addr_a, 20_000_000);
        buggy_inputs_consumed.insert(addr_b, 10_000_000);

        // Drive computes `input_current_balances[addr] = original_balance - consumed`
        // and feeds *that* (with the address nonce) into the fee-deduction code.
        // Reproducing that step here.
        let mut input_current_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            BTreeMap::new();
        input_current_balances.insert(addr_a, (0, addr_a_balance - 20_000_000)); // 0 remaining
        input_current_balances.insert(addr_b, (0, addr_b_balance - 10_000_000)); // 40M remaining

        // Use a representative fee that's small enough to be plausible
        // but large enough that any non-zero remaining balance on an
        // input could absorb it (so we know the failure isn't "fee too
        // large" but specifically "fee target has zero remaining").
        let fee: Credits = 1_000_000;

        let added_to_outputs: BTreeMap<PlatformAddress, Credits> = outputs.clone();

        let result = deduct_fee_from_outputs_or_remaining_balance_of_inputs(
            input_current_balances.clone(),
            added_to_outputs,
            &fee_strategy,
            fee,
            pv,
        )
        .expect("deduction call must succeed (the rejection is expressed via fee_fully_covered)");

        assert!(
            !result.fee_fully_covered,
            "Pre-fix selector's output was supposed to be rejected by the protocol's \
             fee deduction (DeductFromInput(0) targets addr_a which has 0 remaining \
             after full consumption), but `fee_fully_covered` came back true. The \
             reproduction is broken or the protocol semantics changed; investigate."
        );

        // Cross-check: addr_b alone would have been able to absorb the
        // fee (40M remaining ≫ 1M fee). The bug is specifically that the
        // strategy targets the WRONG input — the one with no headroom.
        assert!(
            addr_b_balance - 10_000_000 >= fee,
            "sanity: addr_b's remaining ({}) covers the fee ({}); the bug is not \
             a global shortage but a misdirected fee strategy",
            addr_b_balance - 10_000_000,
            fee,
        );
    }

    /// When the lex-smallest candidate is too small to retain fee
    /// headroom AND the remaining inputs cannot absorb enough of
    /// `total_output` to keep its consumption ≥ `min_input_amount`
    /// at the same time, selection must error out rather than
    /// produce a transition the validator will reject.
    ///
    /// Construction: candidates have just barely enough combined
    /// balance to cover `total_output + fee` (so Phase 1 succeeds),
    /// but the lex-smallest entry is so heavily consumed that
    /// `fee_target_min > fee_target_max`.
    #[test]
    fn fee_headroom_violation_errors() {
        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // addr_a (fee target, lex-smallest) holds exactly the
        // minimum input amount, so it cannot retain *any*
        // remaining balance for fee deduction without dropping
        // below `min_input_amount`. addr_b is large enough that
        // Phase 1 (prefix covers `total_output + fee`) succeeds —
        // the algorithm must catch the headroom violation in
        // Phase 3 and error out instead of producing a transition
        // the validator will reject.
        let addr_a_balance = min_input;
        let total_output = 10_000_000u64;
        let addr_b_balance = 20_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let err = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv)
            .expect_err("expected fee-headroom error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("Cannot satisfy fee headroom"),
                    "expected 'Cannot satisfy fee headroom' phrasing in error, got {msg:?}",
                );
                // The exhaustion-path message references the
                // estimated fee that the lex-smallest entry of every
                // tried prefix could not cover.
                assert!(
                    msg.contains("estimated fee"),
                    "expected estimated-fee callout in error, got {msg:?}",
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// `select_inputs` is order-agnostic: it walks `candidates` as-is and
    /// picks the smallest covering prefix. The caller (`auto_select_inputs`)
    /// is responsible for sorting candidates in the desired preference order.
    ///
    /// This test asserts that when candidates arrive in balance-descending
    /// order — the convention `auto_select_inputs` adopts — the largest
    /// single balance covering `total_output + fee` results in a 1-input
    /// map. This is the common path that sidesteps the multi-input fee
    /// headroom logic entirely.
    #[test]
    fn descending_order_picks_single_largest_when_sufficient() {
        let addr_small = p2pkh(0x01);
        let addr_large = p2pkh(0xFE);
        let target = p2pkh(0xCC);
        let total_output = 30_000_000u64;
        let outputs = outputs_for(target, total_output);
        // Caller pre-sorts: largest first.
        let candidates = vec![(addr_large, 100_000_000), (addr_small, 5_000_000)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv)
            .expect("selection");

        assert_eq!(
            selected.len(),
            1,
            "single largest covers, no multi-input case"
        );
        assert!(
            selected.contains_key(&addr_large),
            "the large input is the only one selected"
        );
        assert_eq!(selected[&addr_large], total_output);

        // The fee target (lex-smallest of selected = addr_large here, since it's the only entry)
        // has remaining = 100M - 30M = 70M, far above any plausible fee.
        let estimated_fee =
            estimate_fee_for_inputs_pub(selected.len(), outputs.len(), &fee_strategy, &outputs, pv);
        let remaining = 100_000_000u64 - selected[&addr_large];
        assert!(remaining >= estimated_fee);

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Empty candidate list → error rather than panic / silent zero-input transition.
    #[test]
    fn no_candidates_errors() {
        let target = p2pkh(0x55);
        let outputs = outputs_for(target, 1_000_000);
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let err = select_inputs(Vec::new(), &outputs, 1_000_000, &fee_strategy, pv)
            .expect_err("expected error for empty candidates");
        assert!(matches!(err, PlatformWalletError::AddressOperation(_)));
    }

    /// Finding #4 regression: when `total_output` is below the
    /// protocol's `min_input_amount`, no single-input transfer can
    /// be sized within both the per-input minimum and the structural
    /// `Σ inputs == total_output` invariant. `select_inputs` must
    /// reject upfront with a descriptive error rather than tripping
    /// the internal "should never trip" branch downstream.
    #[test]
    fn total_output_below_min_input_amount_errors() {
        let addr = p2pkh(0x10);
        let target = p2pkh(0x90);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        let total_output = min_input - 1;
        // Output-side minimum applies separately at validate_structure;
        // this test is purely about `select_inputs`'s upfront guard.
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr, 100_000_000)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let err = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv)
            .expect_err("expected below-min-input error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("below the protocol minimum input amount"),
                    "expected below-min-input phrasing in error, got {msg:?}",
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// Finding #1 regression (GMHz scenario): candidates after the
    /// balance-descending sort are `[(addr_X=0x01, 1_000_000),
    /// (addr_Y=0x02, 30_000)]` with `total_output = 950_000`. The
    /// pre-fix algorithm would build a 2-input map `{addr_X: 920_000,
    /// addr_Y: 30_000}` (after Phase 4 distribution), and `addr_Y`'s
    /// 30_000 amount is below `min_input_amount = 100_000`.
    /// `validate_structure` would reject the transition with
    /// `InputBelowMinimumError`. The new selector must either
    /// produce a result whose every input ≥ `min_input_amount`, or
    /// error out — never silently ship a sub-minimum input.
    ///
    /// Note: `auto_select_inputs` filters candidates with balance
    /// below `min_input_amount` upstream, so addr_Y wouldn't even
    /// reach this helper in production. We feed it directly to
    /// `select_inputs` to exercise the in-helper redistribution
    /// path: tail entries whose tentative consumption falls below
    /// `min_input_amount` get folded back into the fee target's
    /// consumption.
    #[test]
    fn non_fee_target_below_min_input_redistributes() {
        let addr_x = p2pkh(0x01); // lex-smallest → fee target
        let addr_y = p2pkh(0x02);
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // GMHz numbers, scaled so total_output is comfortably above
        // min_output_amount (500_000) — the protocol's per-output
        // minimum is checked by validate_structure separately and is
        // unrelated to the input-side redistribution we're exercising.
        let total_output = 950_000u64;
        let addr_x_balance = 1_000_000u64; // covers total_output + fee on its own
        let addr_y_balance = 30_000u64; // below min_input_amount
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_x, addr_x_balance), (addr_y, addr_y_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let result = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv);

        match result {
            Ok(selected) => {
                // Every selected input must satisfy the per-input minimum.
                for (addr, amount) in selected.iter() {
                    assert!(
                        *amount >= min_input,
                        "input {} consumes {} which is below min_input_amount {}",
                        format_address(addr),
                        amount,
                        min_input,
                    );
                }
                let input_sum: Credits = selected.values().sum();
                assert_eq!(input_sum, total_output);
                assert_selection_validates(&selected, &outputs, fee_strategy, pv);
            }
            Err(PlatformWalletError::AddressOperation(_)) => {
                // Acceptable: the helper opted to error out rather
                // than redistribute. Either outcome is valid; the
                // failure mode we're guarding against is a silent
                // sub-minimum input.
            }
            Err(other) => panic!("unexpected error variant: {other:?}"),
        }
    }
}
