use std::collections::BTreeMap;

use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::version::PlatformVersion;
use dpp::version::LATEST_PLATFORM_VERSION;
use key_wallet::PlatformP2PKHAddress;

use crate::changeset::Merge;
use crate::wallet::PlatformAddressWallet;
use crate::{PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::transition::transfer_address_funds::TransferAddressFunds;

use super::saturating_sum_credits;
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
    ///
    /// # Local-ledger ownership invariant (V27-007 / QA-010)
    ///
    /// The SDK returns post-transition states for **all** addresses touched by
    /// the transition, including foreign output addresses the caller does not
    /// own. Only addresses that belong to this wallet's account are written into
    /// the local ledger; foreign addresses are silently skipped. The recipient
    /// wallet's syncer is responsible for observing inbound credits on its own
    /// addresses. Violating this invariant causes the source wallet's
    /// `total_credits()` to absorb the recipient's balance, which corrupts
    /// dust-gate checks and sweep address selection in teardown.
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
                    // V27-007 / QA-010: skip foreign output addresses.
                    // The SDK returns post-transition state for every address in
                    // the transition (inputs + outputs). Output addresses may
                    // belong to a different wallet; writing their balances here
                    // would pollute this wallet's local ledger and corrupt
                    // `total_credits()`. See the method-level doc comment.
                    if !account.contains_platform_address(&p2pkh) {
                        continue;
                    }
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
        drop(wm);

        // Mirror `sync.rs`: persist post-broadcast balances so a restart
        // doesn't reseed `auto_select_inputs` from stale rows. Log-on-error
        // because the on-chain transition already succeeded.
        if !cs.is_empty() {
            if let Err(e) = self.persister.store(cs.clone().into()) {
                tracing::error!("Failed to persist transfer changeset: {}", e);
            }
        }

        Ok(cs)
    }

    /// Transfer credits with an explicit "change address" override.
    ///
    /// When `output_change_address` is `Some(change_addr)`, the wrapper adds
    /// a change output absorbing `Σ consumed − Σ user_outputs` so the
    /// `Σ inputs == Σ outputs` invariant holds. When `None`, this is a
    /// passthrough to [`Self::transfer`].
    ///
    /// The change branch requires [`InputSelection::Explicit`] or
    /// [`InputSelection::ExplicitWithNonces`] — auto-selection trims inputs
    /// to a covering prefix and has no concept of a residual.
    ///
    /// Under `[DeductFromInput(*)]` the caller MUST reserve fee headroom on
    /// the targeted input (i.e. its map value must be strictly below its
    /// on-chain balance by at least the estimated fee); otherwise the chain
    /// rejects the transition with `fee_fully_covered = false`. In that
    /// case `change_amount = (Σ consumed) − (Σ user_outputs)` is smaller
    /// than `(Σ balances) − (Σ user_outputs)`. Under `[ReduceOutput(0)]`
    /// callers may pass the full balances; output 0 absorbs the fee.
    ///
    /// # Errors
    ///
    /// [`PlatformWalletError::AddressOperation`] when the change branch is
    /// requested with [`InputSelection::Auto`], when `change_addr` collides
    /// with `user_outputs` or `inputs`, or when `Σ inputs ≤ Σ user_outputs`.
    #[allow(clippy::too_many_arguments)] // mirrors `transfer` plus the change-address override.
    pub async fn transfer_with_change_address<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        account_index: u32,
        input_selection: InputSelection,
        user_outputs: BTreeMap<PlatformAddress, Credits>,
        output_change_address: Option<PlatformAddress>,
        fee_strategy: AddressFundsFeeStrategy,
        platform_version: Option<&PlatformVersion>,
        address_signer: &S,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError> {
        let Some(change_addr) = output_change_address else {
            return self
                .transfer(
                    account_index,
                    input_selection,
                    user_outputs,
                    fee_strategy,
                    platform_version,
                    address_signer,
                )
                .await;
        };

        let (input_sum, augmented_selection) = match input_selection {
            InputSelection::Explicit(ref inputs) => {
                validate_change_address(&change_addr, &user_outputs, inputs.keys())?;
                (
                    saturating_sum_credits(inputs.values().copied()),
                    InputSelection::Explicit(inputs.clone()),
                )
            }
            InputSelection::ExplicitWithNonces(ref inputs) => {
                validate_change_address(&change_addr, &user_outputs, inputs.keys())?;
                (
                    saturating_sum_credits(inputs.values().map(|(_n, c)| *c)),
                    InputSelection::ExplicitWithNonces(inputs.clone()),
                )
            }
            InputSelection::Auto => {
                return Err(PlatformWalletError::AddressOperation(
                    "output_change_address: Some(_) requires InputSelection::Explicit \
                     or ExplicitWithNonces — the auto-selector trims inputs to a covering \
                     prefix and has no concept of a residual to route to a change address"
                        .to_string(),
                ));
            }
        };

        let outputs_with_change =
            augment_outputs_with_change(user_outputs, change_addr, input_sum)?;

        self.transfer(
            account_index,
            augmented_selection,
            outputs_with_change,
            fee_strategy,
            platform_version,
            address_signer,
        )
        .await
    }

    /// Dispatch to the strategy-specific selector. Returned map values are the
    /// **consumed amount per address**; protocol enforces `Σ inputs == Σ outputs`.
    /// Supported strategies: `[DeductFromInput(0)]`, `[ReduceOutput(0)]`.
    async fn auto_select_inputs(
        &self,
        account_index: u32,
        outputs: &BTreeMap<PlatformAddress, Credits>,
        fee_strategy: &[AddressFundsFeeStrategyStep],
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
        let total_output: Credits = saturating_sum_credits(outputs.values().copied());

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

        // Classify empty-candidates failure into a typed diagnostic.
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
/// funded address exists at all (caller falls through to generic
/// insufficient-balance); otherwise returns the dominant failure shape.
/// When both apply, `OnlyOutputAddressesFunded` wins — rotating the receive
/// address is the typically more actionable fix.
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
    if !funded_outputs.is_empty() {
        return Some(PlatformWalletError::OnlyOutputAddressesFunded {
            funded_outputs,
            min_input_amount,
        });
    }
    if sub_min_count > 0 {
        return Some(PlatformWalletError::OnlyDustInputs {
            sub_min_count,
            sub_min_aggregate,
            min_input_amount,
        });
    }
    None
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

    // Unsatisfiable: every input must be ≥ min_input_amount.
    if total_output < min_input_amount {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Transfer amount {} is below the protocol minimum input amount {}; \
             a transfer cannot be split across inputs in a way that satisfies \
             the per-input minimum",
            total_output, min_input_amount,
        )));
    }

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
        // `accumulated` is the prefix Σ; subtracting the fee target gives Σ of peers.
        let other_total: Credits = accumulated.saturating_sub(fee_target_balance);
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

    // Sub-minimum tail consumptions fold back into the fee target;
    // `validate_structure` would otherwise reject `InputBelowMinimumError`.
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
        // Unreachable given Phase 3's headroom check.
        debug_assert!(
            new_consumed <= fee_target_max,
            "fee target consumption {} exceeds max {} after residue fold",
            new_consumed,
            fee_target_max,
        );
        if new_consumed > fee_target_max {
            return Err(PlatformWalletError::AddressOperation(format!(
                "Cannot satisfy fee headroom after redistributing sub-minimum tail \
                 inputs: fee-target {fee_target_addr} would consume {new_consumed} \
                 (balance {fee_target_balance}, max {fee_target_max}), leaving \
                 less than estimated fee {estimated_fee} of remaining balance",
            )));
        }
        fee_target_consumed = new_consumed;
    }

    selected.insert(fee_target_addr, fee_target_consumed);

    // Defensive post-checks: a malformed Σ or misaligned fee target ships
    // a guaranteed-rejected transition.
    debug_assert_eq!(
        selected.values().copied().sum::<Credits>(),
        total_output,
        "Σ inputs must equal Σ outputs"
    );
    debug_assert_eq!(
        selected.keys().next().copied(),
        Some(fee_target_addr),
        "fee target must be the BTreeMap index-0 (lex-smallest) entry",
    );
    if selected.keys().next().copied() != Some(fee_target_addr) {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Internal selection error: fee target {fee_target_addr} is not the BTreeMap \
             index-0 (lex-smallest) entry; first entry is {:?}",
            selected.keys().next().map(|a| a.to_string()),
        )));
    }
    debug_assert!(
        fee_target_balance.saturating_sub(fee_target_consumed) >= estimated_fee,
        "fee target must retain ≥ estimated_fee for DeductFromInput(0)",
    );
    if fee_target_balance.saturating_sub(fee_target_consumed) < estimated_fee {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Internal selection error: fee target {fee_target_addr} retains {} after \
             consumption, below estimated fee {estimated_fee}",
            fee_target_balance.saturating_sub(fee_target_consumed),
        )));
    }

    Ok(selected)
}

/// `[ReduceOutput(0)]` selector. Output 0 absorbs the fee at chain time, so
/// inputs only need to sum to `total_output` — no fee headroom on inputs.
///
/// Production callers feed candidates from `build_auto_select_candidates`,
/// which already drops sub-`min_input_amount` balances; the helper also
/// guards against direct test/future-caller invocations that skip the
/// pre-filter and would otherwise produce a sub-minimum prefix entry.
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

    // Module-internal guard for direct test/future-caller invocations;
    // production callers pre-filter via `build_auto_select_candidates`.
    if let Some((bad_addr, bad_balance)) = prefix
        .iter()
        .find(|(_, balance)| *balance < min_input_amount)
    {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Candidate {bad_addr} has balance {bad_balance} below \
             min_input_amount {min_input_amount}; callers must pre-filter via \
             build_auto_select_candidates before invoking the selector",
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

    // Donor must keep ≥ `min_input_amount` itself, so its balance must reach
    // `min_input_amount + shift`. Pick the largest peer first — that is the
    // peer most likely to retain enough headroom after donating. We re-sort
    // here rather than rely on caller order to keep the donor invariant
    // local to this block.
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
        selected.insert(donor_addr, donor_consumed.saturating_sub(shift));
        selected.insert(last_addr, last_consumed.saturating_add(shift));
    }

    // TODO(platform#3040): replace with chain-time fee API. Static estimate
    // can be ~2.3x below chain-time, leaving small `output[0]` at risk.
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

    // TODO(platform#3040): drop the heuristic 3x safety band once chain-time
    // fee API lands; current ~2.3x observed gap is not a proven boundary.
    const REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE: Credits = 3;
    let safe_threshold = estimated_fee.saturating_mul(REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE);
    if output_0 < safe_threshold {
        tracing::warn!(
            output_0,
            estimated_fee,
            safety_multiple = REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE,
            tracking_issue = "platform#3040",
            "[ReduceOutput(0)] output 0 ({} credits) is within {}x of the static estimated \
             fee ({} credits); chain-time fee may exceed the static estimate (platform#3040), \
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

/// Reject `change_addr` collisions before the chain does: the protocol
/// errors deterministically when a transition has the same address as
/// both input and output, and silently merging into a caller-declared
/// output would mask a destination amount. Called at every
/// `transfer_with_change_address` entry that has the inputs map in scope.
fn validate_change_address<'a, I>(
    change_addr: &PlatformAddress,
    user_outputs: &BTreeMap<PlatformAddress, Credits>,
    inputs: I,
) -> Result<(), PlatformWalletError>
where
    I: IntoIterator<Item = &'a PlatformAddress>,
{
    if user_outputs.contains_key(change_addr) {
        return Err(PlatformWalletError::AddressOperation(format!(
            "output_change_address {change_addr:?} already appears in user_outputs; \
             refusing to silently merge a change-output amount into a caller-declared \
             output. Pick a fresh change_addr.",
        )));
    }
    if inputs.into_iter().any(|addr| addr == change_addr) {
        return Err(PlatformWalletError::AddressOperation(format!(
            "output_change_address {change_addr:?} also appears in the input map; \
             the protocol rejects transitions where the same address is both input \
             and output. Pick a fresh change_addr.",
        )));
    }
    Ok(())
}

/// Augment `user_outputs` with an explicit change output absorbing the
/// surplus `Σ inputs − Σ user_outputs`. Caller MUST invoke
/// [`validate_change_address`] first to rule out collisions; this fn
/// re-checks the user_outputs side defensively and rejects the
/// no-surplus case.
fn augment_outputs_with_change(
    mut user_outputs: BTreeMap<PlatformAddress, Credits>,
    change_addr: PlatformAddress,
    input_sum: Credits,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    if user_outputs.contains_key(&change_addr) {
        return Err(PlatformWalletError::AddressOperation(format!(
            "output_change_address {change_addr:?} already appears in user_outputs; \
             refusing to silently merge a change-output amount into a caller-declared \
             output. Pick a fresh change_addr.",
        )));
    }
    let user_output_sum: Credits = saturating_sum_credits(user_outputs.values().copied());
    if input_sum <= user_output_sum {
        return Err(PlatformWalletError::AddressOperation(format!(
            "output_change_address: Some(_) requires Σ inputs ({input_sum}) > \
             Σ user_outputs ({user_output_sum}); no surplus to route as change. \
             Drop output_change_address or grow the input map.",
        )));
    }
    let change_amount = input_sum.saturating_sub(user_output_sum);
    user_outputs.insert(change_addr, change_amount);
    Ok(user_outputs)
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
            assert!(*amount >= min_input, "{addr} consumes {amount}");
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

    /// Detector returns `OnlyOutputAddressesFunded` when every funded address
    /// is also a destination, and `OnlyDustInputs` when every funded balance
    /// is below `min_input_amount`. When both shapes apply simultaneously,
    /// the address-collision signal wins (more actionable fix).
    #[test]
    fn detect_no_selectable_inputs_classifies_failure_shape() {
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let addr_out = p2pkh(0xC3);
        let addr_dust = p2pkh(0xD4);
        let outputs = outputs_for(addr_out, min_input);

        // Output-collision only.
        let collision_only = [(addr_out, min_input * 5)];
        match detect_no_selectable_inputs(collision_only.iter().copied(), &outputs, min_input)
            .expect("collision case")
        {
            PlatformWalletError::OnlyOutputAddressesFunded {
                funded_outputs,
                min_input_amount,
            } => {
                assert_eq!(funded_outputs, vec![addr_out]);
                assert_eq!(min_input_amount, min_input);
            }
            other => panic!("expected OnlyOutputAddressesFunded, got {other:?}"),
        }

        // Dust only.
        let no_outputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        let dust_only = [(addr_dust, min_input / 3)];
        match detect_no_selectable_inputs(dust_only.iter().copied(), &no_outputs, min_input)
            .expect("dust case")
        {
            PlatformWalletError::OnlyDustInputs {
                sub_min_count,
                sub_min_aggregate,
                min_input_amount,
            } => {
                assert_eq!(sub_min_count, 1);
                assert_eq!(sub_min_aggregate, min_input / 3);
                assert_eq!(min_input_amount, min_input);
            }
            other => panic!("expected OnlyDustInputs, got {other:?}"),
        }

        // Both: collision wins.
        let both = [(addr_out, min_input * 5), (addr_dust, min_input / 3)];
        match detect_no_selectable_inputs(both.iter().copied(), &outputs, min_input)
            .expect("combined case")
        {
            PlatformWalletError::OnlyOutputAddressesFunded { funded_outputs, .. } => {
                assert_eq!(funded_outputs, vec![addr_out]);
            }
            other => panic!("expected OnlyOutputAddressesFunded, got {other:?}"),
        }

        // No funded address → None (caller falls through to insufficient-balance).
        let no_funds = [(addr_out, 0u64), (addr_dust, 0u64)];
        assert!(
            detect_no_selectable_inputs(no_funds.iter().copied(), &outputs, min_input).is_none()
        );
    }

    /// PA-001b: the change-address override must add exactly one extra output
    /// absorbing `Σ inputs − Σ user_outputs`, leaving `Σ inputs == Σ outputs`
    /// so the protocol's structural invariant still holds.
    #[test]
    fn augment_outputs_with_change_adds_residual_output() {
        let user_target = p2pkh(0x22);
        let change_addr = p2pkh(0x33);
        let user_outputs = outputs_for(user_target, 5_000_000);
        let outputs =
            augment_outputs_with_change(user_outputs, change_addr, 60_000_000).expect("augment");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs.get(&user_target), Some(&5_000_000));
        assert_eq!(
            outputs.get(&change_addr),
            Some(&55_000_000),
            "change output must absorb exactly the surplus"
        );
        let output_sum: Credits = outputs.values().sum();
        assert_eq!(
            output_sum, 60_000_000,
            "Σ outputs must equal input sum (Σ inputs == Σ outputs invariant)"
        );
    }

    /// PA-001b: the override must reject a `change_addr` that already appears
    /// in the caller's user outputs to prevent a silent merge.
    #[test]
    fn augment_outputs_with_change_rejects_duplicate_address() {
        let target = p2pkh(0x44);
        let user_outputs = outputs_for(target, 5_000_000);
        let err = augment_outputs_with_change(user_outputs, target, 60_000_000)
            .expect_err("change_addr equal to user output must be rejected");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("already appears in user_outputs"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// PA-001b: when `Σ user_outputs ≥ Σ inputs` there is no surplus to route.
    /// The wrapper must reject rather than emit a zero-credit (or underflowing)
    /// change output.
    #[test]
    fn augment_outputs_with_change_rejects_no_surplus() {
        let target = p2pkh(0x55);
        let change_addr = p2pkh(0x66);
        let user_outputs = outputs_for(target, 60_000_000);
        let err = augment_outputs_with_change(user_outputs, change_addr, 60_000_000)
            .expect_err("equal sums must be rejected: nothing to route as change");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(msg.contains("no surplus"), "unexpected message: {msg}");
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// `validate_change_address` rejects collisions with user_outputs OR
    /// the input map; otherwise it accepts.
    #[test]
    fn validate_change_address_rejects_both_collision_shapes() {
        let target = p2pkh(0xAA);
        let input = p2pkh(0xBB);
        let other = p2pkh(0xCC);
        let user_outputs = outputs_for(target, 5_000_000);
        let input_keys = std::iter::once(&input);

        // Collision with user_outputs.
        let err = validate_change_address(&target, &user_outputs, input_keys.clone())
            .expect_err("user_outputs collision");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(msg.contains("already appears in user_outputs"), "{msg}");
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }

        // Collision with inputs.
        let err = validate_change_address(&input, &user_outputs, input_keys.clone())
            .expect_err("input collision");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(msg.contains("also appears in the input map"), "{msg}");
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }

        // Fresh address — accepted.
        validate_change_address(&other, &user_outputs, input_keys)
            .expect("fresh change_addr must validate");
    }

    /// `[ReduceOutput(0)]` Phase 3 success: two-input prefix where the trim
    /// drops the last entry below `min_input_amount` and the donor has the
    /// headroom to lift it back. Verifies the shift lands both entries
    /// exactly at min_input and the donor's remaining consumption stays
    /// ≥ min_input.
    #[test]
    fn reduce_output_phase3_donor_lift_success() {
        let addr_a = p2pkh(0xA1); // larger balance → donor
        let addr_b = p2pkh(0xB2); // smaller balance → last (gets trimmed)
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // candidates are caller-supplied in balance-descending order.
        // total_output leaves only 10_000 on the last entry after trim,
        // forcing Phase 3 donor lift up to `min_input`.
        let addr_a_balance = 109_000_000u64;
        let addr_b_balance = min_input; // exactly min_input
        let total_output = addr_a_balance + min_input - 90_000;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let selected =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("Phase 3 donor lift must succeed");

        // Both entries end exactly at expected post-shift values.
        let consumed_a = selected[&addr_a];
        let consumed_b = selected[&addr_b];
        assert_eq!(consumed_b, min_input, "last lifted back to min_input");
        assert_eq!(
            consumed_a,
            addr_a_balance - 90_000,
            "donor consumes its balance minus the shift"
        );
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output, "Σ inputs == total_output");

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// `[ReduceOutput(0)]` Phase 3 failure: trim drops the last entry below
    /// `min_input_amount` and no peer has the headroom to donate. The
    /// selector must error out rather than ship a sub-minimum input.
    #[test]
    fn reduce_output_phase3_redistribution_fails_when_no_donor() {
        let addr_a = p2pkh(0x01); // donor candidate (only peer); exactly min_input → no headroom
        let addr_b = p2pkh(0x02); // last entry, gets trimmed below min_input
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // Both candidates have exactly min_input balance. Prefix covers
        // total_output by including both; the trim drops addr_b below
        // min_input and addr_a (min_input == 100_000) cannot give any
        // positive amount without dropping below min_input itself.
        let addr_a_balance = min_input;
        let addr_b_balance = min_input;
        let total_output = min_input + (min_input / 10); // 110_000: requires both inputs
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let err =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect_err("no donor with headroom — must error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("Cannot satisfy per-input minimum"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// `[ReduceOutput(0)]` Phase 1 failure: aggregate candidate balance is
    /// below `total_output`. The selector must reject with the
    /// insufficient-balance diagnostic.
    #[test]
    fn reduce_output_phase1_insufficient_aggregate_balance() {
        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let candidates = vec![(addr_a, min_input), (addr_b, min_input)];
        let total_output = min_input * 3; // > Σ balances (= 2 * min_input)
        let outputs = outputs_for(target, total_output);
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let err =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect_err("aggregate balance below total_output — must error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("Insufficient balance"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// CMT-007: `transfer_with_change_address` must reject
    /// `InputSelection::Auto` before doing any I/O. Mock SDK + an empty
    /// wallet manager are enough — the rejection happens in the wrapper's
    /// own match arm, well before `wallet_manager.read().await`.
    #[tokio::test]
    async fn transfer_with_change_address_rejects_auto_selection() {
        use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
        use crate::wallet::platform_addresses::PlatformAddressWallet;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let wallet_manager = Arc::new(RwLock::new(key_wallet_manager::WalletManager::new(
            sdk.network,
        )));
        let persister = WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence));
        let wallet = PlatformAddressWallet::new(sdk, wallet_manager, [0u8; 32], persister);

        let signer = NullSigner;
        let outputs = outputs_for(p2pkh(0x77), 10_000_000);
        let change_addr = p2pkh(0x88);
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let err = wallet
            .transfer_with_change_address(
                0,
                InputSelection::Auto,
                outputs,
                Some(change_addr),
                fee_strategy,
                None,
                &signer,
            )
            .await
            .expect_err("Auto + Some(change_addr) must error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("InputSelection::Explicit"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// Signer used only by tests that exercise paths which short-circuit
    /// before any signing happens. Never produces a signature.
    #[derive(Debug)]
    pub(super) struct NullSigner;

    #[async_trait::async_trait]
    impl dpp::identity::signer::Signer<PlatformAddress> for NullSigner {
        async fn sign(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            unreachable!("NullSigner used by a test path that should short-circuit before signing")
        }

        async fn sign_create_witness(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("NullSigner used by a test path that should short-circuit before signing")
        }

        fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
            false
        }
    }
}
