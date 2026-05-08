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
    /// from the account via [`InputSelection::Auto`]. When `platform_version`
    /// is `None`, [`LATEST_PLATFORM_VERSION`] drives fee estimation.
    ///
    /// `address_signer` produces ECDSA signatures for the input
    /// [`PlatformAddress`]es; the wallet itself holds no key material —
    /// callers supply a seed-backed, hardware, or FFI-trampoline signer.
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
                // Auto-select supports `[DeductFromInput(0)]` and `[ReduceOutput(0)]`;
                // any other shape must use `Explicit`.
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

        let key_source = {
            let guard = self.provider.read().await;
            guard
                .as_ref()
                .and_then(|p| p.key_source(&self.wallet_id, account_index))
        };

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
    /// fee-strategy-specific helper. The returned map's values are the
    /// **consumed amount per address** — the protocol enforces
    /// `Σ inputs == Σ outputs`.
    ///
    /// Supported strategies:
    /// - `[DeductFromInput(0)]` — fee deducted from input 0's remaining
    ///   balance at chain time; selector reserves headroom.
    /// - `[ReduceOutput(0)]` — fee taken from output 0's amount at chain
    ///   time; selector skips input-side headroom.
    async fn auto_select_inputs(
        &self,
        account_index: u32,
        outputs: &BTreeMap<PlatformAddress, Credits>,
        fee_strategy: &[AddressFundsFeeStrategyStep],
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
        // Saturating fold matches the file-wide policy. Total credit supply
        // is far below `u64::MAX`, so saturation is unreachable in practice.
        let total_output: Credits = outputs
            .values()
            .copied()
            .fold(0u64, Credits::saturating_add);

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

        // When the candidate set is empty, classify why (funded-but-also-output
        // addresses, sub-minimum aggregate, or both) and raise the typed
        // `NoSelectableInputs` variant so callers get a precise diagnostic
        // without parsing downstream message strings.
        if candidates.is_empty() {
            if let Some(err) = detect_no_selectable_inputs(
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
    /// the inputs need beyond the output amounts. Walks the strategy steps
    /// in order and returns the residual fee inputs must cover.
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
                    if let Some(&amount) = output_amounts.get(*index as usize) {
                        let reduction = remaining_fee.min(amount);
                        remaining_fee -= reduction;
                    }
                }
                AddressFundsFeeStrategyStep::DeductFromInput(_) => {
                    break;
                }
            }
        }

        remaining_fee
    }
}

/// Build the auto-selection candidate list: keep only addresses whose balance
/// reaches `min_input_amount`, drop any address that is also a destination
/// output (the protocol forbids the same address being both input and output),
/// then sort balance-descending so the selector picks the smallest covering
/// prefix.
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

/// Classify why no candidate survived the filter. Returns `None` when no
/// funded address exists at all, letting the caller fall through to the
/// generic insufficient-balance path. Otherwise reports both failure shapes
/// (funded-but-also-output, sub-minimum aggregate) in one variant; the
/// `Display` rendering interpolates zero-valued fields naturally.
fn detect_no_selectable_inputs<I>(
    address_balances: I,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    min_input_amount: Credits,
) -> Option<PlatformWalletError>
where
    I: IntoIterator<Item = (PlatformAddress, Credits)>,
{
    let mut funded_outputs: Vec<PlatformAddress> = Vec::new();
    let mut sub_min_count: usize = 0;
    let mut sub_min_aggregate: Credits = 0;
    for (addr, balance) in address_balances {
        if balance >= min_input_amount {
            if outputs.contains_key(&addr) {
                funded_outputs.push(addr);
            }
        } else if balance > 0 {
            sub_min_count = sub_min_count.saturating_add(1);
            sub_min_aggregate = sub_min_aggregate.saturating_add(balance);
        }
    }
    if funded_outputs.is_empty() && sub_min_count == 0 {
        return None;
    }
    Some(PlatformWalletError::NoSelectableInputs {
        funded_outputs,
        sub_min_count,
        sub_min_aggregate,
        min_input_amount,
    })
}

/// `[DeductFromInput(0)]` selector. Order-agnostic: walks `candidates` as-is
/// and picks the smallest covering prefix.
///
/// Produces an inputs map satisfying:
/// 1. `Σ selected.values() == total_output`.
/// 2. The `DeductFromInput(0)` fee target — the lex-smallest entry, which is
///    the `BTreeMap` index-0 — must keep `balance − consumed ≥ estimated_fee`
///    so drive can deduct the fee from its remaining balance (otherwise
///    `fee_fully_covered = false` and the transition is rejected).
///
/// Algorithm:
/// 1. Grow the prefix until `Σ balances ≥ total_output + estimated_fee`.
/// 2. Within that prefix, the lex-smallest entry is the fee target.
/// 3. Solve for `fee_target_consumed` in
///    `[max(min_input_amount, total_output − other_total),
///       fee_target_balance − estimated_fee]`. If the range is empty, extend
///    the prefix and retry; error out only when candidates are exhausted.
/// 4. Insert the fee target at its minimum consumption, then distribute the
///    remainder of `total_output` across the other prefix entries. Tail
///    consumptions below `min_input_amount` get folded back into the fee
///    target rather than producing a sub-minimum input.
/// 5. Defensive invariant checks.
fn select_inputs_deduct_from_input(
    candidates: Vec<(PlatformAddress, Credits)>,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    total_output: Credits,
    fee_strategy: &[AddressFundsFeeStrategyStep],
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
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
    // `total_output` if `total_output < min_input_amount`. Reject upfront.
    if total_output < min_input_amount {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Transfer amount {} is below the protocol minimum input amount {}; \
             a transfer cannot be split across inputs in a way that satisfies \
             the per-input minimum",
            total_output, min_input_amount,
        )));
    }

    // Saturating arithmetic on `Credits` (== u64): total Dash credit supply
    // is far below `u64::MAX`, so saturation is unreachable in practice.
    let mut prefix: Vec<(PlatformAddress, Credits)> = Vec::new();
    let mut accumulated: Credits = 0;
    let mut last_estimated_fee: Credits = 0;
    let mut feasible: Option<(PlatformAddress, Credits, Credits, Credits)> = None;

    for (address, balance) in candidates {
        prefix.push((address, balance));
        accumulated = accumulated.saturating_add(balance);

        let estimated_fee = PlatformAddressWallet::estimate_fee_for_inputs(
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
    }

    let Some((fee_target_addr, fee_target_balance, fee_target_min, estimated_fee)) = feasible
    else {
        let required_total = total_output.saturating_add(last_estimated_fee);
        if accumulated < required_total {
            return Err(PlatformWalletError::AddressOperation(format!(
                "Insufficient balance: available {} credits, required {} \
                 (outputs {} + estimated fee {}; [DeductFromInput(0)])",
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

    // Phase 4: consume `fee_target_min` from the fee target, distribute the
    // rest of `total_output` over the remaining prefix in caller order. Tail
    // consumptions below `min_input_amount` get folded into the fee target —
    // `validate_structure` would otherwise reject the transition with
    // `InputBelowMinimumError`.
    //
    // Single-target fold-back is the simplest correct behaviour. Multi-peer
    // redistribution is a defensible optimisation but adds combinatorial
    // complexity for a borderline case; ship the simpler form first.
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
            // Should be unreachable given Phase 3's headroom check, but
            // guarded explicitly: silently shipping an invalid transition
            // would be worse than a loud error here.
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

    // Defensive post-check: production trusts the protocol-side
    // `validate_structure` for the full audit, but a malformed Σ here would
    // ship a guaranteed-rejected transition. Cheap enough to verify.
    debug_assert_eq!(
        selected.values().copied().sum::<Credits>(),
        total_output,
        "Σ inputs must equal Σ outputs"
    );
    if selected.keys().next().copied() != Some(fee_target_addr) {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Internal selection error: fee target {} is not the BTreeMap index-0 \
             (lex-smallest) entry; first entry is {:?}",
            format_address(&fee_target_addr),
            selected.keys().next().map(format_address),
        )));
    }
    if fee_target_balance.saturating_sub(fee_target_consumed) < estimated_fee {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Internal selection error: fee target {} retains {} after consumption, \
             below estimated fee {}",
            format_address(&fee_target_addr),
            fee_target_balance.saturating_sub(fee_target_consumed),
            estimated_fee,
        )));
    }

    Ok(selected)
}

/// `[ReduceOutput(0)]` selector. Output 0 absorbs the fee at chain time, so
/// inputs only need to sum to `total_output` — no fee headroom on inputs.
///
/// Algorithm:
/// 1. Grow the prefix until `Σ balances ≥ total_output`.
/// 2. Trim the last prefix entry by `surplus = Σ − total_output` so
///    `Σ inputs == Σ outputs`. Earlier entries stay at full balance.
/// 3. If the trim drops the last entry below `min_input_amount`, shift
///    consumption from a peer in **balance-descending donor order** (largest
///    peer first) to lift it back up while keeping the donor ≥
///    `min_input_amount`. Error out if no peer has the headroom.
/// 4. Estimate the fee for the chosen input count and verify
///    `output[0] ≥ estimated_fee`; otherwise the chain-time deduction would
///    leave the fee uncovered.
/// 5. Defensive invariant checks.
fn select_inputs_reduce_output(
    candidates: Vec<(PlatformAddress, Credits)>,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    total_output: Credits,
    fee_strategy: &[AddressFundsFeeStrategyStep],
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    if !matches!(fee_strategy, [AddressFundsFeeStrategyStep::ReduceOutput(0)]) {
        return Err(PlatformWalletError::AddressOperation(
            "select_inputs_reduce_output only supports fee_strategy = \
             [ReduceOutput(0)]; other shapes must route through the dispatcher"
                .to_string(),
        ));
    }

    let output_count = outputs.len();
    let min_input_amount = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_input_amount;

    if total_output < min_input_amount {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Transfer amount {} is below the protocol minimum input amount {}; \
             a transfer cannot be split across inputs in a way that satisfies \
             the per-input minimum",
            total_output, min_input_amount,
        )));
    }

    // Saturating arithmetic everywhere: total credit supply is far below
    // `u64::MAX`, so saturation is unreachable in practice.
    let mut prefix: Vec<(PlatformAddress, Credits)> = Vec::new();
    let mut accumulated: Credits = 0;
    for (address, balance) in candidates {
        prefix.push((address, balance));
        accumulated = accumulated.saturating_add(balance);
        if accumulated >= total_output {
            break;
        }
    }

    if accumulated < total_output {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Insufficient balance: available {} credits, required {} \
             (outputs sum; [ReduceOutput(0)] absorbs the fee from output 0)",
            accumulated, total_output,
        )));
    }

    // Phase 1.5: every prefix entry must clear `min_input_amount`. Phase 2
    // sets `consumed = balance` for every non-last entry, so a sub-minimum
    // candidate would silently produce an invalid transition. Production
    // callers filter via `build_auto_select_candidates`; this is the
    // module-internal guard for direct test/future-caller invocations.
    if let Some((bad_addr, bad_balance)) = prefix
        .iter()
        .find(|(_, balance)| *balance < min_input_amount)
    {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Candidate {} has balance {} below min_input_amount {}; \
             callers must pre-filter via build_auto_select_candidates \
             before invoking the selector",
            format_address(bad_addr),
            bad_balance,
            min_input_amount,
        )));
    }

    // Phase 2: every prefix entry consumes its full balance except the last,
    // which absorbs the surplus.
    let mut selected: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let surplus = accumulated - total_output;
    let last_index = prefix.len() - 1;
    for (i, (addr, balance)) in prefix.iter().enumerate() {
        let consumed = if i == last_index {
            balance.saturating_sub(surplus)
        } else {
            *balance
        };
        selected.insert(*addr, consumed);
    }

    // Phase 3: if the trim dropped the last entry below `min_input_amount`,
    // lift it from a peer in balance-descending donor order. The donor must
    // keep ≥ `min_input_amount` itself, so its balance must reach
    // `min_input_amount + shift`. Largest peer first maximises the chance of
    // meeting that threshold.
    let last_addr = prefix[last_index].0;
    let last_consumed = selected[&last_addr];
    if last_consumed < min_input_amount && prefix.len() > 1 {
        let shift = min_input_amount - last_consumed;
        let donor_threshold = min_input_amount.saturating_add(shift);
        let mut donor_candidates: Vec<&(PlatformAddress, Credits)> = prefix
            .iter()
            .filter(|(addr, _)| *addr != last_addr)
            .collect();
        donor_candidates.sort_by(|a, b| b.1.cmp(&a.1));
        let donor_addr = donor_candidates
            .into_iter()
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

    // Phase 4: ReduceOutput(0) takes the fee from output 0 at chain time;
    // verify output 0 has enough to absorb it.
    //
    // KNOWN BUG — platform #3040 (https://github.com/dashpay/platform/issues/3040):
    // `estimate_fee_for_inputs` returns only the static
    // `state_transition_min_fees` floor. Chain-time fee includes storage +
    // processing costs that scale with the actual operation set; for 1in/1out
    // we've seen ~6.5M static vs ~14.94M real. Until #3040 is fixed, callers
    // with small `output[0]` should prefer `[DeductFromInput(0)]`.
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

    // Borderline warning for platform #3040: chain-time fees can exceed the
    // static estimate by ~2.3x in practice. The 3x multiple is a heuristic
    // safety band, not a proven boundary; revisit when #3040 is fixed.
    const REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE: Credits = 3;
    let safe_threshold = estimated_fee.saturating_mul(REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE);
    if output_0 < safe_threshold {
        tracing::warn!(
            output_0,
            estimated_fee,
            safety_multiple = REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE,
            "[ReduceOutput(0)] output 0 ({} credits) is within {}x of the static estimated \
             fee ({} credits); chain-time fee may exceed the static estimate (platform #3040), \
             risking on-chain rejection. Consider raising output 0 or switching to \
             [DeductFromInput(0)].",
            output_0,
            REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE,
            estimated_fee,
        );
    }

    debug_assert_eq!(
        selected.values().copied().sum::<Credits>(),
        total_output,
        "Σ inputs must equal Σ outputs"
    );

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

    /// Feed a selector result into dpp's `validate_structure` to confirm the
    /// transition is shape-valid. Uses zero nonces and dummy P2PKH witnesses.
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

    /// One address with a large balance, output amount well below it →
    /// `selected[addr] == total_output` (NOT full balance, NOT `total_output + fee`).
    /// Fee comes from the address's remaining balance via `DeductFromInput(0)`.
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

        assert_eq!(selected.get(&addr), Some(&10_000_000));
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, outputs.values().copied().sum::<Credits>());

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Balance-descending input — the order `auto_select_inputs` supplies —
    /// with a single largest balance covering `total_output + fee` produces a
    /// 1-input map.
    #[test]
    fn descending_order_picks_single_largest_when_sufficient() {
        let addr_small = p2pkh(0x01);
        let addr_large = p2pkh(0xFE);
        let target = p2pkh(0xCC);
        let total_output = 30_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_large, 100_000_000), (addr_small, 5_000_000)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("selection");

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[&addr_large], total_output);

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Protocol-level proof: the inputs map a naive selector would produce
    /// for `(20M, 50M)` / `total_output = 30M` / `[DeductFromInput(0)]`
    /// (`{addr_a: 20M, addr_b: 10M}`), when fed to
    /// `deduct_fee_from_outputs_or_remaining_balance_of_inputs`, returns
    /// `fee_fully_covered = false` — drive's `validate_fees_of_event` would
    /// reject the transition. The fixed selector retains `min_input_amount`
    /// at addr_a so the fee deduction has headroom.
    #[test]
    fn pre_fix_buggy_selector_output_is_rejected_by_protocol_fee_deduction() {
        use dpp::address_funds::fee_strategy::deduct_fee_from_inputs_and_outputs::deduct_fee_from_outputs_or_remaining_balance_of_inputs;
        use dpp::prelude::AddressNonce;

        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0xFF);
        let total_output = 30_000_000u64;
        let addr_a_balance = 20_000_000u64;
        let addr_b_balance = 50_000_000u64;
        let outputs = outputs_for(target, total_output);
        let fee_strategy: AddressFundsFeeStrategy =
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let mut buggy_inputs_consumed: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        buggy_inputs_consumed.insert(addr_a, 20_000_000);
        buggy_inputs_consumed.insert(addr_b, 10_000_000);

        let mut input_current_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            BTreeMap::new();
        input_current_balances.insert(addr_a, (0, addr_a_balance - 20_000_000));
        input_current_balances.insert(addr_b, (0, addr_b_balance - 10_000_000));

        let fee: Credits = 1_000_000;
        let added_to_outputs: BTreeMap<PlatformAddress, Credits> = outputs.clone();

        let result = deduct_fee_from_outputs_or_remaining_balance_of_inputs(
            input_current_balances.clone(),
            added_to_outputs,
            &fee_strategy,
            fee,
            pv,
        )
        .expect("deduction call must succeed (rejection is via fee_fully_covered)");

        assert!(
            !result.fee_fully_covered,
            "Pre-fix selector's output must be rejected by the protocol's fee deduction"
        );
        assert!(addr_b_balance - 10_000_000 >= fee);

        // Cross-check: the fixed selector at the same fixture produces a
        // map that DOES leave headroom on addr_a.
        let fixed = select_inputs_deduct_from_input(
            vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)],
            &outputs,
            total_output,
            &fee_strategy,
            pv,
        )
        .expect("fixed selector");
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        assert_eq!(fixed.get(&addr_a), Some(&min_input));
        assert_eq!(fixed.keys().next(), Some(&addr_a));
        assert_selection_validates(&fixed, &outputs, fee_strategy, pv);
    }

    /// Phase 1 covers `total_output + fee` but the lex-smallest entry has no
    /// headroom for the fee. Selection must error out rather than ship a
    /// transition the validator will reject.
    #[test]
    fn fee_headroom_violation_errors() {
        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // addr_a holds exactly `min_input_amount` — no remaining balance for
        // the fee. addr_b lets Phase 1 succeed, so the headroom violation
        // must be caught in Phase 3.
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
                assert!(msg.contains("Cannot satisfy fee headroom"), "got {msg:?}");
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// Tail entry's tentative consumption falls below `min_input_amount`. The
    /// selector folds the residue back into the fee target so every shipped
    /// input ≥ `min_input_amount`.
    #[test]
    fn non_fee_target_below_min_input_redistributes() {
        let addr_x = p2pkh(0x01); // lex-smallest → fee target
        let addr_y = p2pkh(0x02); // sub-min peer; folds into fee target
        let addr_z = p2pkh(0x03); // large peer; absorbs the bulk
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // Fixture (numbers chosen against fee schedule `500_000*N + 6_000_000`):
        // - prefix [x] (acc 10M) doesn't cover 10.5M (=4M+fee_1in).
        // - prefix [x,y] (acc 10.08M) doesn't cover 11M (=4M+fee_2in).
        // - prefix [x,y,z] (acc 12.08M) covers 11.5M.
        // - Phase 4: y's tentative=80k folds into fee target; z absorbs 2M.
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
                .expect("redistribute path must reach Ok");

        for (addr, amount) in selected.iter() {
            assert!(
                *amount >= min_input,
                "{} consumes {amount}",
                format_address(addr)
            );
        }
        assert!(
            !selected.contains_key(&addr_y),
            "sub-min y must be folded out"
        );
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output);

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// QA-001: an address that is also a destination output must be excluded
    /// from auto-selection candidates, even when it is the only address with
    /// sufficient balance. Otherwise the protocol would reject the transition
    /// with `Output address cannot also be an input address`.
    #[test]
    fn auto_select_inputs_excludes_output_addresses() {
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let addr_a = p2pkh(0xA1);
        let addr_b = p2pkh(0xB2);
        let outputs = outputs_for(addr_a, min_input);

        let address_balances = vec![(addr_a, min_input * 3), (addr_b, min_input / 2)];
        let candidates =
            build_auto_select_candidates(address_balances.clone(), &outputs, min_input);
        assert!(candidates.is_empty(), "got {candidates:?}");

        let no_outputs = BTreeMap::new();
        let with_self_spend =
            build_auto_select_candidates(address_balances, &no_outputs, min_input);
        assert_eq!(with_self_spend, vec![(addr_a, min_input * 3)]);
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

    /// `total_output < min_input_amount` is unsatisfiable. The selector must
    /// reject upfront with a descriptive error.
    #[test]
    fn total_output_below_min_input_amount_errors() {
        let addr = p2pkh(0x10);
        let target = p2pkh(0x90);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        let total_output = min_input - 1;
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
                    "{msg:?}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// Single input fully covers `total_output`; the input is trimmed to
    /// `total_output` (no fee headroom on inputs — output 0 absorbs the fee
    /// at chain time).
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

        assert_eq!(selected.get(&addr), Some(&total_output));
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output);

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Both failure modes coexist: one funded-but-also-output address AND
    /// one sub-min address. Detector reports both via the unified
    /// `NoSelectableInputs` variant.
    #[test]
    fn detect_no_selectable_inputs_combines_both_cases() {
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let addr_out = p2pkh(0xC3);
        let addr_dust = p2pkh(0xD4);
        let outputs = outputs_for(addr_out, min_input);
        let address_balances = [(addr_out, min_input * 5), (addr_dust, min_input / 3)];

        let err =
            detect_no_selectable_inputs(address_balances.iter().copied(), &outputs, min_input)
                .expect("expected NoSelectableInputs");
        match &err {
            PlatformWalletError::NoSelectableInputs {
                funded_outputs,
                sub_min_count,
                sub_min_aggregate,
                min_input_amount,
            } => {
                assert_eq!(funded_outputs, &vec![addr_out]);
                assert_eq!(*sub_min_count, 1);
                assert_eq!(*sub_min_aggregate, min_input / 3);
                assert_eq!(*min_input_amount, min_input);
            }
            other => panic!("expected NoSelectableInputs, got {other:?}"),
        }
        let rendered = err.to_string();
        assert!(rendered.contains("funded_outputs"), "{rendered}");
        assert!(rendered.contains("sub_min_count"), "{rendered}");

        // No funded address at all → detector returns None (caller falls
        // through to generic insufficient-balance error).
        let no_funds = [(addr_out, 0u64), (addr_dust, 0u64)];
        assert!(
            detect_no_selectable_inputs(no_funds.iter().copied(), &outputs, min_input).is_none()
        );
    }
}
