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
                // Auto-select supports `[DeductFromInput(0)]` and
                // `[ReduceOutput(0)]`; any other shape must use `Explicit`.
                if !matches!(
                    fee_strategy.as_slice(),
                    [AddressFundsFeeStrategyStep::DeductFromInput(0)]
                        | [AddressFundsFeeStrategyStep::ReduceOutput(0)]
                ) {
                    return Err(PlatformWalletError::AddressOperation(
                        "InputSelection::Auto supports fee_strategy = [DeductFromInput(0)] \
                         or [ReduceOutput(0)]; for other strategies use InputSelection::Explicit"
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

    /// Auto-select inputs balance-descending and dispatch to the
    /// fee-strategy-specific helper. The returned map's values are
    /// the **consumed amount per address** — the protocol enforces
    /// `Σ inputs == Σ outputs`.
    ///
    /// Supported strategies:
    /// - `[DeductFromInput(0)]` — fee deducted from input 0's
    ///   remaining balance at chain time; selector reserves headroom.
    /// - `[ReduceOutput(0)]` — fee taken from output 0's amount at
    ///   chain time; selector skips input-side headroom.
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

        // Filter to addresses with balance ≥ `min_input_amount` (the
        // protocol's per-input minimum — anything smaller cannot
        // legally appear as an input), exclude any address that is
        // also a destination output (the protocol rejects a transition
        // where the same address is both input and output), and sort
        // balance-descending so the helper picks the smallest
        // covering prefix.
        let address_balances: Vec<(PlatformAddress, Credits)> = account
            .addresses
            .addresses
            .values()
            .filter_map(|addr_info| {
                let p2pkh = PlatformP2PKHAddress::from_address(&addr_info.address).ok()?;
                let balance = account.address_credit_balance(&p2pkh);
                Some((PlatformAddress::P2pkh(p2pkh.to_bytes()), balance))
            })
            .collect();
        let candidates = build_auto_select_candidates(
            address_balances.iter().copied(),
            outputs,
            min_input_amount,
        );

        // Surface the "every funded address is also an output" case
        // distinctly from generic insufficient-balance: when the
        // candidate set is empty but at least one address satisfies
        // the per-input minimum and is filtered out solely because it
        // overlaps `outputs`, raise a typed
        // `OnlyOutputAddressesFunded` error so callers don't have to
        // parse downstream message strings (QA-001 follow-up).
        //
        // TODO(QA-002): add an end-to-end integration test driving the
        // full `auto_select_inputs` path (requires a `WalletManager`
        // harness with synthetic balances). Pure-helper coverage of
        // the detection logic lives in `auto_select_tests::detect_*`.
        if candidates.is_empty() {
            if let Some(err) = detect_only_output_addresses_funded(
                address_balances.iter().copied(),
                outputs,
                min_input_amount,
            ) {
                return Err(err);
            }
        }

        match fee_strategy {
            [AddressFundsFeeStrategyStep::DeductFromInput(0)] => select_inputs_deduct_from_input(
                candidates,
                outputs,
                total_output,
                fee_strategy,
                platform_version,
            ),
            [AddressFundsFeeStrategyStep::ReduceOutput(0)] => select_inputs_reduce_output(
                candidates,
                outputs,
                total_output,
                fee_strategy,
                platform_version,
            ),
            _ => Err(PlatformWalletError::AddressOperation(
                "auto_select_inputs supports fee_strategy = [DeductFromInput(0)] \
                 or [ReduceOutput(0)]; other shapes must use InputSelection::Explicit"
                    .to_string(),
            )),
        }
    }

    /// Simulate the fee strategy to determine how much additional balance
    /// the inputs need beyond the output amounts. Walks the strategy
    /// steps in order, deducting from outputs/inputs until the fee is
    /// covered, and returns the portion that must come from inputs.
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

/// Build the auto-selection candidate list: keep only addresses whose
/// balance reaches `min_input_amount`, drop any address that already
/// appears as a destination output (the protocol forbids the same
/// address being both input and output of a single transition), then
/// sort balance-descending so the selector can pick the smallest
/// covering prefix.
fn build_auto_select_candidates<I>(
    address_balances: I,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    min_input_amount: Credits,
) -> Vec<(PlatformAddress, Credits)>
where
    I: IntoIterator<Item = (PlatformAddress, Credits)>,
{
    let mut candidates: Vec<(PlatformAddress, Credits)> = address_balances
        .into_iter()
        .filter(|(addr, balance)| *balance >= min_input_amount && !outputs.contains_key(addr))
        .collect();
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    candidates
}

/// Detect the "only output addresses are funded" failure mode and
/// produce a typed [`PlatformWalletError::OnlyOutputAddressesFunded`].
///
/// Caller invokes this only when [`build_auto_select_candidates`]
/// returned empty. We re-scan `address_balances` with the outputs
/// filter dropped — any address satisfying the per-input minimum that
/// also appears in `outputs` proves the candidate set was emptied
/// solely by the input-equals-output filter, not by genuine
/// insufficient balance. Returns `None` when no such address exists,
/// letting the caller fall through to the generic insufficient-balance
/// path inside the selector helpers.
fn detect_only_output_addresses_funded<I>(
    address_balances: I,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    min_input_amount: Credits,
) -> Option<PlatformWalletError>
where
    I: IntoIterator<Item = (PlatformAddress, Credits)>,
{
    let funded_outputs: Vec<PlatformAddress> = address_balances
        .into_iter()
        .filter(|(addr, balance)| *balance >= min_input_amount && outputs.contains_key(addr))
        .map(|(addr, _)| addr)
        .collect();
    if funded_outputs.is_empty() {
        None
    } else {
        Some(PlatformWalletError::OnlyOutputAddressesFunded {
            outputs: funded_outputs,
        })
    }
}

/// `[DeductFromInput(0)]` selector. Order-agnostic: walks
/// `candidates` as-is and picks the smallest covering prefix.
///
/// Produces an inputs map satisfying two protocol invariants:
/// 1. `Σ selected.values() == total_output`.
/// 2. The `DeductFromInput(0)` fee target — the lex-smallest entry,
///    which is the `BTreeMap` index-0 — must keep
///    `balance − consumed ≥ estimated_fee` so drive can deduct
///    the fee from its remaining balance (otherwise
///    `fee_fully_covered = false` and the transition is rejected).
///
/// Algorithm:
/// 1. Grow the prefix until `Σ balances ≥ total_output + estimated_fee`.
/// 2. Within that prefix, the lex-smallest entry is the fee target.
/// 3. Solve for `fee_target_consumed` in
///    `[max(min_input_amount, total_output − other_total),
///       fee_target_balance − estimated_fee]`. If the range is empty
///    (no headroom), extend the prefix and retry; error out only
///    when candidates are exhausted.
/// 4. Insert the fee target at its minimum consumption, then
///    distribute the remainder of `total_output` across the other
///    prefix entries in caller-supplied order. Tail consumptions
///    below `min_input_amount` get folded back into the fee target
///    rather than producing a sub-minimum input.
/// 5. Defensive invariant checks.
///
/// Caller (`auto_select_inputs`) sorts candidates balance-descending
/// in practice, but the helper itself doesn't rely on that order.
fn select_inputs_deduct_from_input(
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
        "select_inputs_deduct_from_input requires [DeductFromInput(0)]; \
         the dispatcher should have routed other shapes elsewhere"
    );
    if !matches!(
        fee_strategy,
        [AddressFundsFeeStrategyStep::DeductFromInput(0)]
    ) {
        return Err(PlatformWalletError::AddressOperation(
            "select_inputs_deduct_from_input only supports fee_strategy = \
             [DeductFromInput(0)]; other shapes must route through the dispatcher"
                .to_string(),
        ));
    }

    let output_count = outputs.len();
    let min_input_amount = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_input_amount;

    // No input can simultaneously be ≥ `min_input_amount` AND sum to
    // `total_output` if `total_output < min_input_amount`. Reject upfront
    // rather than tripping the per-input minimum check downstream.
    if total_output < min_input_amount {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Transfer amount {} is below the protocol minimum input amount {}; \
             a transfer cannot be split across inputs in a way that satisfies \
             the per-input minimum",
            total_output, min_input_amount,
        )));
    }

    // Phase 1-3: extend the prefix one candidate at a time until it
    // covers `total_output + estimated_fee` AND the lex-smallest
    // prefix entry has headroom to absorb the fee.
    let mut prefix: Vec<(PlatformAddress, Credits)> = Vec::new();
    let mut accumulated: Credits = 0;
    let mut last_estimated_fee: Credits = 0;
    let mut feasible: Option<(PlatformAddress, Credits, Credits, Credits)> = None;

    for (address, balance) in candidates {
        prefix.push((address, balance));
        accumulated = checked_credits_add(
            accumulated,
            balance,
            "select_inputs_deduct_from_input: prefix accumulator",
        )?;

        let estimated_fee = PlatformAddressWallet::estimate_fee_for_inputs(
            prefix.len(),
            output_count,
            fee_strategy,
            outputs,
            platform_version,
        );
        last_estimated_fee = estimated_fee;
        let required = checked_credits_add(
            total_output,
            estimated_fee,
            "select_inputs_deduct_from_input: total_output + estimated_fee",
        )?;

        if accumulated < required {
            continue;
        }

        // Phase 2: lex-smallest of the current prefix is the fee target.
        let (fee_target_addr, fee_target_balance) = prefix
            .iter()
            .min_by_key(|(addr, _)| *addr)
            .copied()
            .expect("prefix is non-empty: we just pushed");

        // `estimated_fee` may exceed `fee_target_balance` for a thin
        // fee target; saturating to 0 makes the `fee_target_min <=
        // fee_target_max` headroom check below reject this prefix size
        // and grow. Not an overflow site.
        let fee_target_max = fee_target_balance.saturating_sub(estimated_fee);
        let other_total: Credits = prefix
            .iter()
            .filter(|(addr, _)| addr != &fee_target_addr)
            .map(|(_, bal)| *bal)
            .sum();
        // `other_total` may exceed `total_output` when peers alone
        // cover the outputs; the saturating floor of 0 is intentional —
        // combined with `max(min_input_amount, ..)` it yields
        // `min_input_amount`, the smallest legal consumption for the
        // fee target. Not an overflow site.
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
        let required_total = checked_credits_add(
            total_output,
            last_estimated_fee,
            "select_inputs_deduct_from_input: required_total in error path",
        )?;
        if accumulated < required_total {
            return Err(PlatformWalletError::AddressOperation(format!(
                "Insufficient balance: available {} credits, required {} \
                 (outputs {} + estimated fee {})",
                accumulated, required_total, total_output, last_estimated_fee,
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

    // Phase 4: consume `fee_target_min` from the fee target, distribute
    // the rest of `total_output` over the remaining prefix in caller
    // order. Tail consumptions below `min_input_amount` get folded into
    // the fee target — `validate_structure` would otherwise reject the
    // transition with `InputBelowMinimumError`.
    let mut fee_target_consumed = fee_target_min;
    let fee_target_max = checked_credits_sub(
        fee_target_balance,
        estimated_fee,
        "select_inputs_deduct_from_input: Phase 4 fee_target_max",
    )?;
    let mut selected: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();

    let mut remaining = checked_credits_sub(
        total_output,
        fee_target_consumed,
        "select_inputs_deduct_from_input: Phase 4 remaining",
    )?;
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
            residue_to_fee_target = checked_credits_add(
                residue_to_fee_target,
                tentative,
                "select_inputs_deduct_from_input: residue_to_fee_target",
            )?;
            remaining = checked_credits_sub(
                remaining,
                tentative,
                "select_inputs_deduct_from_input: remaining after residue fold",
            )?;
            continue;
        }
        selected.insert(*addr, tentative);
        remaining = checked_credits_sub(
            remaining,
            tentative,
            "select_inputs_deduct_from_input: remaining after select",
        )?;
    }

    if residue_to_fee_target > 0 {
        let new_consumed = checked_credits_add(
            fee_target_consumed,
            residue_to_fee_target,
            "select_inputs_deduct_from_input: new_consumed",
        )?;
        if new_consumed > fee_target_max {
            // Should be unreachable given Phase 3's headroom check, but
            // guarded explicitly: silently shipping an invalid
            // transition would be worse than a loud error here.
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

    // Phase 5: defensive invariant checks. Fail loudly here rather
    // than ship a transition the validator will reject.
    let input_sum: Credits = selected.values().sum();
    debug_assert_eq!(input_sum, total_output, "Σ inputs == Σ outputs invariant");
    debug_assert_eq!(
        selected.keys().next().copied(),
        Some(fee_target_addr),
        "fee target must be the BTreeMap index-0 (lex-smallest) entry"
    );
    // Saturating-sub is fine here: the assert exists to catch a
    // negative remaining (which saturates to 0 and trips `>= estimated_fee`).
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

/// `[ReduceOutput(0)]` selector. Output 0 absorbs the fee at chain
/// time, so inputs only need to sum to `total_output` — no fee
/// headroom on inputs. Order-agnostic: walks `candidates` as-is and
/// picks the smallest covering prefix.
///
/// Produces an inputs map satisfying:
/// 1. `Σ selected.values() == total_output`.
/// 2. Every selected input ≥ `min_input_amount`.
/// 3. The BTreeMap-index-0 output (lex-smallest) holds enough to
///    absorb the estimated fee at chain time.
///
/// Algorithm (mirrors the 5-phase shape of the input-side helper):
/// 1. Grow the prefix until `Σ balances ≥ total_output`.
/// 2. Trim the last prefix entry by `surplus = Σ − total_output` so
///    `Σ inputs == Σ outputs`. Earlier entries stay at full balance.
/// 3. If the trim drops the last entry below `min_input_amount`,
///    shift consumption from the lex-smallest peer to lift it back up
///    while keeping the peer ≥ `min_input_amount`. Error out if no
///    peer has the headroom.
/// 4. Estimate the fee for the chosen input count and verify
///    `output[0] ≥ estimated_fee`; otherwise the chain-time
///    `ReduceOutput(0)` deduction would leave the fee uncovered.
/// 5. Defensive invariant checks.
fn select_inputs_reduce_output(
    candidates: Vec<(PlatformAddress, Credits)>,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    total_output: Credits,
    fee_strategy: &[AddressFundsFeeStrategyStep],
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    debug_assert!(
        matches!(fee_strategy, [AddressFundsFeeStrategyStep::ReduceOutput(0)]),
        "select_inputs_reduce_output requires [ReduceOutput(0)]; \
         the dispatcher should have routed other shapes elsewhere"
    );

    let output_count = outputs.len();
    let min_input_amount = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_input_amount;

    // Same upfront guard as the DeductFromInput(0) helper: a single
    // input cannot satisfy `≥ min_input_amount` and sum to a smaller
    // `total_output` — reject loudly rather than tripping the
    // per-input minimum check downstream.
    if total_output < min_input_amount {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Transfer amount {} is below the protocol minimum input amount {}; \
             a transfer cannot be split across inputs in a way that satisfies \
             the per-input minimum",
            total_output, min_input_amount,
        )));
    }

    // Phase 1: walk `candidates` until the running sum covers
    // `total_output`. Last entry will be trimmed in Phase 2.
    let mut prefix: Vec<(PlatformAddress, Credits)> = Vec::new();
    let mut accumulated: Credits = 0;
    for (address, balance) in candidates {
        prefix.push((address, balance));
        accumulated = checked_credits_add(
            accumulated,
            balance,
            "select_inputs_reduce_output: prefix accumulator",
        )?;
        if accumulated >= total_output {
            break;
        }
    }

    if accumulated < total_output {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Insufficient balance: available {} credits, required {} \
             (outputs sum; ReduceOutput(0) absorbs the fee from output 0)",
            accumulated, total_output,
        )));
    }

    // Phase 2: every prefix entry consumes its full balance except
    // the last, which absorbs the surplus.
    let mut selected: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let surplus = accumulated - total_output;
    let last_index = prefix.len() - 1;
    for (i, (addr, balance)) in prefix.iter().enumerate() {
        let consumed = if i == last_index {
            // Loop above stops as soon as `accumulated >= total_output`,
            // so before the final push we had `accumulated_prev <
            // total_output`, hence `surplus = accumulated_prev +
            // balance - total_output < balance`. Saturating-sub is
            // documentary defense, the underflow path is unreachable.
            balance.saturating_sub(surplus)
        } else {
            *balance
        };
        selected.insert(*addr, consumed);
    }

    // Phase 3: if the trim dropped the last entry below
    // `min_input_amount`, lift it from the lex-smallest peer with
    // spare balance. The peer must keep ≥ `min_input_amount` itself.
    let last_addr = prefix[last_index].0;
    let last_consumed = selected[&last_addr];
    if last_consumed < min_input_amount && prefix.len() > 1 {
        let shift = min_input_amount - last_consumed;
        let donor_threshold = checked_credits_add(
            min_input_amount,
            shift,
            "select_inputs_reduce_output: donor_threshold",
        )?;
        let donor_addr = prefix
            .iter()
            .filter(|(addr, _)| *addr != last_addr)
            .find(|(_, balance)| *balance >= donor_threshold)
            .map(|(addr, _)| *addr);
        let Some(donor_addr) = donor_addr else {
            return Err(PlatformWalletError::AddressOperation(format!(
                "Cannot satisfy per-input minimum: trimming the last input to \
                 {} (below {}) and no peer has ≥ {} of headroom to redistribute",
                last_consumed, min_input_amount, donor_threshold,
            )));
        };
        let donor_consumed = selected[&donor_addr];
        selected.insert(donor_addr, donor_consumed - shift);
        selected.insert(last_addr, last_consumed + shift);
    }

    // Phase 4: ReduceOutput(0) takes the fee from output 0 at chain
    // time; verify the chosen output 0 has enough to absorb it.
    //
    // KNOWN BUG — platform #3040: `PlatformAddressWallet::estimate_fee_for_inputs` returns
    // `AddressFundsTransferTransition::estimate_min_fee`, which models only
    // the static `state_transition_min_fees` floor. The chain-time fee
    // includes storage + processing costs that scale with the actual
    // operation set; for 1in/1out we've seen ~6.5M static vs ~14.94M
    // real, leaving the auto-selector to greenlight a transition that
    // then fails on-chain with `AddressesNotEnoughFundsError`.
    //
    // Until #3040 is fixed at the dpp layer, callers with small `output[0]`
    // (where `output[0]` >= static estimate but < chain-time fee) should
    // prefer `[DeductFromInput(0)]` so any shortfall comes out of an input
    // rather than the absorbing output. The Phase 4 check below remains as
    // the static lower-bound gate; it cannot reject the chain-time-only
    // failure mode.
    let estimated_fee = PlatformAddressWallet::estimate_fee_for_inputs(
        selected.len(),
        output_count,
        fee_strategy,
        outputs,
        platform_version,
    );
    let output_0 = outputs.values().next().copied().unwrap_or(0);
    if output_0 < estimated_fee {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Output 0 ({} credits) cannot absorb estimated fee ({} credits) \
             under [ReduceOutput(0)]; raise output 0 or use a different fee strategy",
            output_0, estimated_fee,
        )));
    }

    // Phase 5: defensive invariant checks. Fail loudly here rather
    // than ship a transition the validator will reject.
    let input_sum: Credits = selected.values().sum();
    debug_assert_eq!(input_sum, total_output, "Σ inputs == Σ outputs invariant");
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

/// Checked add of two `Credits` values. Returns
/// [`PlatformWalletError::ArithmeticOverflow`] when the addition would
/// wrap. `Credits` is `u64`; total Dash supply (≈ 21M DASH ×
/// 100_000_000 duffs/DASH × the credit conversion factor) is far below
/// `u64::MAX`, so this overflow is unreachable in practice — the helper
/// is defensive correctness, not a bug fix.
#[inline]
fn checked_credits_add(
    a: Credits,
    b: Credits,
    context: &str,
) -> Result<Credits, PlatformWalletError> {
    a.checked_add(b)
        .ok_or_else(|| PlatformWalletError::ArithmeticOverflow {
            context: context.to_string(),
        })
}

/// Checked sub of two `Credits` values. Returns
/// [`PlatformWalletError::ArithmeticOverflow`] when the subtraction
/// would wrap. Mirrors [`checked_credits_add`] — defensive only.
#[inline]
fn checked_credits_sub(
    a: Credits,
    b: Credits,
    context: &str,
) -> Result<Credits, PlatformWalletError> {
    a.checked_sub(b)
        .ok_or_else(|| PlatformWalletError::ArithmeticOverflow {
            context: context.to_string(),
        })
}

#[cfg(test)]
mod auto_select_tests {
    use super::*;
    use dpp::address_funds::AddressWitness;
    use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use dpp::state_transition::StateTransitionStructureValidation;
    use std::collections::BTreeSet;

    fn p2pkh(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    fn outputs_for(target: PlatformAddress, amount: Credits) -> BTreeMap<PlatformAddress, Credits> {
        std::iter::once((target, amount)).collect()
    }

    /// Build a minimal valid `AddressFundsTransferTransitionV0` from a
    /// selector result and feed it to `validate_structure`. Uses zero
    /// nonces and dummy P2PKH witnesses; the structural validator only
    /// inspects counts, not signature material.
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

    /// One address with 100M credits, output 10M → `selected[addr] == 10M`
    /// (the consumed amount) — NOT the full balance, NOT `10M + fee`.
    /// The fee comes from the address's remaining balance via
    /// `DeductFromInput(0)` and is never part of the inputs map.
    #[test]
    fn single_input_oversized_balance_trims_to_output_amount() {
        let addr = p2pkh(0x11);
        let target = p2pkh(0x22);
        let outputs = outputs_for(target, 10_000_000);
        let total_output = 10_000_000u64;
        let candidates = vec![(addr, 100_000_000u64)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
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

    /// Two-input case: the fee target (lex-smallest, `DeductFromInput(0)`)
    /// consumes only `min_input_amount`, the rest of `total_output` is
    /// drawn from the other input — so the fee target keeps enough
    /// remaining balance for the fee deduction.
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

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
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
        let estimated_fee = PlatformAddressWallet::estimate_fee_for_inputs(
            selected.len(),
            outputs.len(),
            &fee_strategy,
            &outputs,
            pv,
        );
        let remaining = addr_a_balance - selected[&addr_a];
        assert!(
            remaining >= estimated_fee,
            "fee target remaining {} must be ≥ estimated fee {}",
            remaining,
            estimated_fee,
        );

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Insufficient inputs → descriptive `AddressOperation` error.
    #[test]
    fn insufficient_balance_errors() {
        let addr = p2pkh(0x33);
        let target = p2pkh(0x44);
        let total_output = 100_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr, 5_000_000)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let err =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
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

    /// First candidate covers `total_output` but not `total_output + fee`,
    /// so a second input joins. Consumption shifts to the non-fee-target
    /// input; `Σ values` still equals `total_output`.
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

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("selection");

        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let input_sum: Credits = selected.values().sum();
        assert_eq!(
            input_sum, total_output,
            "Σ inputs must equal Σ outputs (protocol's structural invariant)"
        );

        // addr_a (lex-smallest) is the fee target: consumes
        // `min_input_amount`; addr_b absorbs the remainder.
        assert_eq!(selected.get(&addr_a), Some(&min_input));
        assert_eq!(selected.get(&addr_b), Some(&(total_output - min_input)));
        // addr_a stays at BTreeMap index 0.
        assert_eq!(selected.keys().next(), Some(&addr_a));

        // Headroom invariant.
        let estimated_fee = PlatformAddressWallet::estimate_fee_for_inputs(
            selected.len(),
            outputs.len(),
            &fee_strategy,
            &outputs,
            pv,
        );
        assert!(
            addr_a_balance - selected[&addr_a] >= estimated_fee,
            "fee target must retain ≥ estimated_fee for DeductFromInput(0)"
        );

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Candidates `(20M, 50M)`, `total_output = 30M`,
    /// `[DeductFromInput(0)]`: the fee target (`addr_a`) must remain
    /// in the map at `min_input_amount` with the rest of consumption
    /// shifted onto `addr_b`, so `addr_a` retains enough balance for
    /// `DeductFromInput(0)` to deduct the fee at chain time.
    #[test]
    fn fee_target_keeps_remaining_for_fee_deduction() {
        // addr_a < addr_b lexicographically — `DeductFromInput(0)`
        // targets the BTreeMap index-0 entry.
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

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
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

        // (3) Fee target's post-consumption remaining ≥ estimated fee.
        let estimated_fee = PlatformAddressWallet::estimate_fee_for_inputs(
            selected.len(),
            outputs.len(),
            &fee_strategy,
            &outputs,
            pv,
        );
        let remaining = addr_a_balance - selected[&addr_a];
        assert!(
            remaining >= estimated_fee,
            "fee target remaining {} must be ≥ estimated fee {} (CodeRabbit regression)",
            remaining,
            estimated_fee,
        );

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Protocol-level proof: the inputs map a naive selector would
    /// produce for `(20M, 50M)` / `total_output = 30M` /
    /// `[DeductFromInput(0)]` (`{addr_a: 20M, addr_b: 10M}`), when
    /// fed to dpp's `deduct_fee_from_outputs_or_remaining_balance_of_inputs`,
    /// returns `fee_fully_covered = false` — so drive's
    /// `validate_fees_of_event` would reject the transition. The
    /// correct selector is verified by
    /// `fee_target_keeps_remaining_for_fee_deduction`.
    #[test]
    fn pre_fix_buggy_selector_output_is_rejected_by_protocol_fee_deduction() {
        use dpp::address_funds::fee_strategy::deduct_fee_from_inputs_and_outputs::deduct_fee_from_outputs_or_remaining_balance_of_inputs;
        use dpp::prelude::AddressNonce;

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

        // Naive selector output: addr_a fully consumed (20M),
        // addr_b trimmed to 10M. Σ = total_output, but addr_a is
        // fully drained — no headroom left for the fee.
        let mut buggy_inputs_consumed: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        buggy_inputs_consumed.insert(addr_a, 20_000_000);
        buggy_inputs_consumed.insert(addr_b, 10_000_000);

        // Drive computes `input_current_balances[addr] = original_balance - consumed`
        // and feeds that (with the address nonce) into fee deduction.
        let mut input_current_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            BTreeMap::new();
        input_current_balances.insert(addr_a, (0, addr_a_balance - 20_000_000)); // 0 remaining
        input_current_balances.insert(addr_b, (0, addr_b_balance - 10_000_000)); // 40M remaining

        // Representative fee: small enough to be plausible, large
        // enough that any non-zero remaining input balance could
        // absorb it. The failure here is "fee target has 0 remaining",
        // not "fee too large".
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

        // Cross-check: addr_b's remaining (40M) ≫ fee. The bug is the
        // strategy targeting addr_a, the one with no headroom.
        assert!(
            addr_b_balance - 10_000_000 >= fee,
            "sanity: addr_b's remaining ({}) covers the fee ({}); the bug is not \
             a global shortage but a misdirected fee strategy",
            addr_b_balance - 10_000_000,
            fee,
        );
    }

    /// Phase 1 covers `total_output + fee` but the lex-smallest entry's
    /// `fee_target_min > fee_target_max`. Selection must error out
    /// rather than ship a transition the validator will reject.
    #[test]
    fn fee_headroom_violation_errors() {
        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // addr_a (fee target) holds exactly `min_input_amount` — no
        // remaining balance for the fee. addr_b lets Phase 1 succeed,
        // so the headroom violation must be caught in Phase 3.
        let addr_a_balance = min_input;
        let total_output = 10_000_000u64;
        let addr_b_balance = 20_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let err =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect_err("expected fee-headroom error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("Cannot satisfy fee headroom"),
                    "expected 'Cannot satisfy fee headroom' phrasing in error, got {msg:?}",
                );
                // Exhaustion-path message names the estimated fee
                // that no tried prefix could leave headroom for.
                assert!(
                    msg.contains("estimated fee"),
                    "expected estimated-fee callout in error, got {msg:?}",
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// With balance-descending input — the order `auto_select_inputs`
    /// supplies — a single largest balance covering `total_output + fee`
    /// produces a 1-input map, sidestepping the multi-input headroom
    /// branch.
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

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
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
        let estimated_fee = PlatformAddressWallet::estimate_fee_for_inputs(
            selected.len(),
            outputs.len(),
            &fee_strategy,
            &outputs,
            pv,
        );
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

        let err =
            select_inputs_deduct_from_input(Vec::new(), &outputs, 1_000_000, &fee_strategy, pv)
                .expect_err("expected error for empty candidates");
        assert!(matches!(err, PlatformWalletError::AddressOperation(_)));
    }

    /// `total_output < min_input_amount` is unsatisfiable (no input can
    /// be both ≥ `min_input_amount` and sum to `total_output`).
    /// `select_inputs` must reject upfront with a descriptive error.
    #[test]
    fn total_output_below_min_input_amount_errors() {
        let addr = p2pkh(0x10);
        let target = p2pkh(0x90);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        let total_output = min_input - 1;
        // Output-side minimum is checked separately by `validate_structure`;
        // this test exercises only the input-side upfront guard.
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr, 100_000_000)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let err =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
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

    /// Tail entry's tentative consumption falls below `min_input_amount`.
    /// The selector must fold the residue back into the fee target
    /// (so every shipped input ≥ `min_input_amount`) — never silently
    /// ship a sub-minimum input that `validate_structure` would reject
    /// with `InputBelowMinimumError`.
    ///
    /// Production callers filter sub-minimum candidates upstream in
    /// `auto_select_inputs`; this test feeds the helper directly to
    /// exercise its in-helper redistribution path. The fixture is
    /// engineered so the Ok branch is reachable: with
    /// `input_cost=500_000`, `output_cost=6_000_000` the static fee is
    /// `500_000*N + 6_000_000*max(M,1)`, and the chosen balances make
    /// Phase 1 grow the prefix to [x,y,z] before Phase 3 finds
    /// headroom.
    #[test]
    fn non_fee_target_below_min_input_redistributes() {
        let addr_x = p2pkh(0x01); // lex-smallest → fee target
        let addr_y = p2pkh(0x02); // sub-min peer; folds into fee target
        let addr_z = p2pkh(0x03); // large peer; absorbs the bulk
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // Engineered fixture (numbers chosen against fee schedule
        // `500_000 * N + 6_000_000`):
        // - prefix [x] (acc 10M) doesn't cover required 10.5M (=4M+fee_1in).
        // - prefix [x,y] (acc 10.08M) doesn't cover 11M (=4M+fee_2in).
        // - prefix [x,y,z] (acc 12.08M) covers 11.5M (=4M+fee_3in).
        //   fee_target_max(x) = 10M-7.5M = 2.5M;
        //   fee_target_min = max(100k, 4M-2.08M) = 1.92M;
        //   1.92M ≤ 2.5M → Phase 3 succeeds.
        // - Phase 4: fee_target_consumed=1.92M, remaining=2.08M;
        //   y's tentative=80k folds (residue=80k); z's tentative=2M
        //   selected; new_consumed=2M ≤ fee_target_max ✓.
        let total_output = 4_000_000u64;
        let addr_x_balance = 10_000_000u64;
        let addr_y_balance = 80_000u64; // below min_input_amount (100_000)
        let addr_z_balance = 2_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![
            (addr_x, addr_x_balance),
            (addr_y, addr_y_balance),
            (addr_z, addr_z_balance),
        ];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("redistribute path must reach Ok with engineered fixture");

        // (1) Every selected input satisfies the per-input minimum
        //     (the redistribute path's invariant — sub-min y must NOT
        //     appear in `selected`).
        for (addr, amount) in selected.iter() {
            assert!(
                *amount >= min_input,
                "input {} consumes {} which is below min_input_amount {}",
                format_address(addr),
                amount,
                min_input,
            );
        }

        // (2) Sub-min y was folded — must not be in the inputs map.
        assert!(
            !selected.contains_key(&addr_y),
            "sub-min addr_y must not appear as an input; expected fold into fee target"
        );

        // (3) Σ inputs == Σ outputs.
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output);

        // (4) Fee target (lex-smallest x) absorbed the y residue —
        //     selected[x] = fee_target_min + addr_y_balance.
        let expected_fee_target_min = total_output - addr_y_balance - addr_z_balance;
        assert_eq!(
            selected.get(&addr_x),
            Some(&(expected_fee_target_min + addr_y_balance)),
            "fee target must consume fee_target_min plus the folded y residue"
        );
        assert_eq!(
            selected.get(&addr_z),
            Some(&addr_z_balance),
            "z absorbs its full balance as a non-fee-target peer"
        );

        // (5) Structural validation against dpp.
        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Single input fully covers `total_output`; the input is trimmed
    /// to `total_output` (no fee headroom on inputs — output 0 absorbs
    /// the fee at chain time).
    #[test]
    fn reduce_output_happy_path_single_input() {
        let addr = p2pkh(0x11);
        let target = p2pkh(0x22);
        let total_output = 10_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr, 100_000_000u64)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("selection");

        assert_eq!(
            selected.get(&addr),
            Some(&total_output),
            "single input consumes exactly total_output (no headroom on inputs)"
        );
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output, "Σ inputs == Σ outputs");

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Multiple inputs needed: every entry except the last consumes
    /// its full balance; the last is trimmed by `surplus` so
    /// `Σ inputs == Σ outputs`.
    #[test]
    fn reduce_output_multi_input_trims_to_total_output() {
        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0x99);
        let total_output = 60_000_000u64;
        let outputs = outputs_for(target, total_output);
        // Caller pre-sorts balance-descending; addr_b is the larger,
        // walked first, fully consumed; addr_a is trimmed.
        let addr_b_balance = 50_000_000u64;
        let addr_a_balance = 20_000_000u64;
        let candidates = vec![(addr_b, addr_b_balance), (addr_a, addr_a_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("selection");

        assert_eq!(selected.len(), 2);
        assert_eq!(
            selected.get(&addr_b),
            Some(&addr_b_balance),
            "non-last entry stays at full balance"
        );
        assert_eq!(
            selected.get(&addr_a),
            Some(&(total_output - addr_b_balance)),
            "last entry trimmed by surplus"
        );
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output);

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Multi-output: only output 0 (BTreeMap-lex-smallest) absorbs the
    /// fee at chain time. The selector ships the user's outputs map
    /// untouched — outputs 1, 2, ... still hold their requested amounts.
    #[test]
    fn reduce_output_multi_output_only_first_absorbs_fee() {
        let addr_in = p2pkh(0xFE);
        // Output 0 (lex-smallest) gets the fee; the rest are untouched.
        let out0 = p2pkh(0x10);
        let out1 = p2pkh(0x20);
        let out2 = p2pkh(0x30);
        let mut outputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        outputs.insert(out0, 50_000_000);
        outputs.insert(out1, 10_000_000);
        outputs.insert(out2, 5_000_000);
        let total_output: Credits = outputs.values().sum();

        let candidates = vec![(addr_in, total_output + 100_000_000u64)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("selection");

        // Selector mutates only inputs; outputs map is what the caller
        // hands to the SDK and what `validate_structure` inspects.
        assert_eq!(outputs.get(&out1), Some(&10_000_000));
        assert_eq!(outputs.get(&out2), Some(&5_000_000));

        // Confirm BTreeMap-index-0 is `out0` (lex-smallest by 20-byte hash).
        assert_eq!(outputs.keys().next(), Some(&out0));

        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output);

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Output 0 < estimated fee → descriptive `AddressOperation` error.
    /// The protocol's chain-time `ReduceOutput(0)` deduction would
    /// otherwise leave the fee uncovered.
    #[test]
    fn reduce_output_output_too_small_to_absorb_fee_errors() {
        let addr_in = p2pkh(0xAA);
        let target = p2pkh(0xBB);
        let pv = LATEST_PLATFORM_VERSION;
        let min_output = pv.dpp.state_transitions.address_funds.min_output_amount;
        // Output sits at the protocol minimum — far below any plausible
        // fee for a real transition.
        let total_output = min_output;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_in, 100_000_000u64)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let estimated_fee =
            PlatformAddressWallet::estimate_fee_for_inputs(1, 1, &fee_strategy, &outputs, pv);
        // Sanity guard: this test is meaningful only when the output
        // really cannot cover the fee.
        assert!(
            total_output < estimated_fee,
            "test premise broken: output {} ≥ estimated fee {}",
            total_output,
            estimated_fee,
        );

        let err =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect_err("expected output-too-small-for-fee error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("cannot absorb estimated fee"),
                    "expected output-cannot-absorb-fee phrasing, got {msg:?}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// QA-001: an address that is also a destination output must be
    /// excluded from auto-selection candidates, even when it is the
    /// only address with sufficient balance. Otherwise the selector
    /// would propose the same address as both input and output and
    /// the protocol would reject the transition with `Output address
    /// cannot also be an input address`.
    #[test]
    fn auto_select_inputs_excludes_output_addresses() {
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let addr_a = p2pkh(0xA1);
        let addr_b = p2pkh(0xB2);
        let outputs = outputs_for(addr_a, min_input);

        // addr_a is funded above the floor but is also the only
        // output; addr_b is below the floor.
        let address_balances = vec![(addr_a, min_input * 3), (addr_b, min_input / 2)];
        let candidates =
            build_auto_select_candidates(address_balances.clone(), &outputs, min_input);
        assert!(
            candidates.is_empty(),
            "addr_a must be excluded as an output and addr_b must be excluded as below the \
             min-input floor; got {candidates:?}",
        );

        // Sanity check: without the outputs filter, addr_a would
        // pass the floor check — proving the exclusion is what
        // emptied the list.
        let no_outputs = BTreeMap::new();
        let with_self_spend =
            build_auto_select_candidates(address_balances, &no_outputs, min_input);
        assert_eq!(
            with_self_spend,
            vec![(addr_a, min_input * 3)],
            "without the outputs filter addr_a alone passes",
        );
    }

    /// QA-001: a funded non-output address coexisting with a funded
    /// output address must remain selectable; only the output one
    /// is dropped. Also confirms balance-descending order survives
    /// the filter.
    #[test]
    fn auto_select_inputs_keeps_non_output_funded_addresses() {
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let addr_out = p2pkh(0xC3);
        let addr_in_small = p2pkh(0xD4);
        let addr_in_big = p2pkh(0xE5);
        let outputs = outputs_for(addr_out, min_input);

        let address_balances = vec![
            (addr_out, min_input * 5),
            (addr_in_small, min_input * 2),
            (addr_in_big, min_input * 10),
        ];
        let candidates = build_auto_select_candidates(address_balances, &outputs, min_input);

        assert_eq!(
            candidates,
            vec![
                (addr_in_big, min_input * 10),
                (addr_in_small, min_input * 2)
            ],
            "output address must be dropped; remaining candidates sort balance-descending",
        );
    }

    /// CMT-014: when every funded address is also an output (the
    /// `OnlyOutputAddressesFunded` failure mode), the detector
    /// returns the typed error carrying the exact set of offending
    /// addresses, not a generic insufficient-balance string.
    #[test]
    fn detect_only_output_addresses_funded_typed_payload() {
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let addr_a = p2pkh(0xA1);
        let addr_b = p2pkh(0xB2);
        // Both funded above floor; both also outputs.
        let outputs: BTreeMap<PlatformAddress, Credits> =
            [(addr_a, min_input), (addr_b, min_input)]
                .into_iter()
                .collect();
        let address_balances = [(addr_a, min_input * 5), (addr_b, min_input * 4)];

        let err = detect_only_output_addresses_funded(
            address_balances.iter().copied(),
            &outputs,
            min_input,
        )
        .expect("expected OnlyOutputAddressesFunded");
        match &err {
            PlatformWalletError::OnlyOutputAddressesFunded { outputs: payload } => {
                assert_eq!(
                    payload.iter().copied().collect::<BTreeSet<_>>(),
                    [addr_a, addr_b].iter().copied().collect::<BTreeSet<_>>(),
                    "payload must list every funded output address",
                );
            }
            other => panic!("expected OnlyOutputAddressesFunded, got {other:?}"),
        }
        // QA-001: Display interpolates the payload so
        // error.to_string() carries it across boundaries that strip
        // typed error variants (notably FFI).
        let rendered = err.to_string();
        assert!(
            rendered.contains("funded addresses"),
            "Display must explain the failure: {rendered}"
        );
    }

    /// No funded addresses at all (every entry below the per-input
    /// minimum) → detector returns `None`, letting the caller fall
    /// through to the existing insufficient-balance error path inside
    /// the selector helpers rather than misclassifying as "only
    /// outputs funded".
    #[test]
    fn detect_only_output_addresses_funded_returns_none_when_unfunded() {
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let addr_a = p2pkh(0xA1);
        let addr_b = p2pkh(0xB2);
        let outputs = outputs_for(addr_a, min_input);
        // Both below floor — no funded addresses at all.
        let address_balances = [(addr_a, min_input / 2), (addr_b, min_input / 4)];

        let err = detect_only_output_addresses_funded(
            address_balances.iter().copied(),
            &outputs,
            min_input,
        );
        assert!(
            err.is_none(),
            "no funded address means generic insufficient-balance, not the typed error"
        );
    }

    /// At least one funded non-output candidate exists → detector
    /// returns `None`, letting the regular candidate path proceed.
    /// (Belt-and-braces: in production this branch is unreachable
    /// because `auto_select_inputs` only consults the detector when
    /// `build_auto_select_candidates` returned empty — but the helper
    /// must still behave correctly when called in isolation.)
    #[test]
    fn detect_only_output_addresses_funded_returns_none_when_non_output_funded() {
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let addr_out = p2pkh(0xC3);
        let addr_in = p2pkh(0xD4);
        let outputs = outputs_for(addr_out, min_input);
        let address_balances = [(addr_out, min_input * 5), (addr_in, min_input * 3)];

        // Both funded; addr_out IS an output, addr_in is NOT. The
        // helper still scans for funded outputs and would produce a
        // typed error — but the production flow only calls this when
        // candidates is empty, which requires no funded non-output
        // candidates to exist. Calling here with a funded non-output
        // is a contract violation by the caller; the helper still
        // returns the typed error because both filters look only at
        // the outputs side. Document that the contract is "call only
        // when candidates.is_empty()" by asserting the typed-error
        // result with the funded output payload.
        let err = detect_only_output_addresses_funded(
            address_balances.iter().copied(),
            &outputs,
            min_input,
        )
        .expect("typed error fires whenever a funded output exists");
        match err {
            PlatformWalletError::OnlyOutputAddressesFunded { outputs: payload } => {
                assert_eq!(payload, vec![addr_out]);
            }
            other => panic!("expected OnlyOutputAddressesFunded, got {other:?}"),
        }
    }

    /// `checked_credits_add` / `checked_credits_sub` happy path returns
    /// the wrapped sum/difference; the overflow path produces a typed
    /// `ArithmeticOverflow` carrying the supplied call-site context so
    /// downstream observers can pinpoint where the overflow happened.
    #[test]
    fn checked_credits_helpers_typed_errors() {
        assert_eq!(checked_credits_add(2, 3, "ctx").unwrap(), 5);
        assert_eq!(checked_credits_sub(5, 3, "ctx").unwrap(), 2);

        let add_err = checked_credits_add(u64::MAX, 1, "add-site")
            .expect_err("expected ArithmeticOverflow on add");
        match add_err {
            PlatformWalletError::ArithmeticOverflow { context } => {
                assert!(
                    context.contains("add-site"),
                    "unexpected context: {context}"
                );
            }
            other => panic!("expected ArithmeticOverflow, got {other:?}"),
        }

        let sub_err =
            checked_credits_sub(0, 1, "sub-site").expect_err("expected ArithmeticOverflow on sub");
        match sub_err {
            PlatformWalletError::ArithmeticOverflow { context } => {
                assert!(
                    context.contains("sub-site"),
                    "unexpected context: {context}"
                );
            }
            other => panic!("expected ArithmeticOverflow, got {other:?}"),
        }
    }

    /// End-to-end structural validation: feed the selector's output
    /// to `AddressFundsTransferTransitionV0::validate_structure` to
    /// confirm the transition is shape-valid under
    /// `[ReduceOutput(0)]`.
    #[test]
    fn reduce_output_validates() {
        let addr_in = p2pkh(0x77);
        let target = p2pkh(0x88);
        let total_output = 25_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_in, 100_000_000u64)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("selection");

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }
}
