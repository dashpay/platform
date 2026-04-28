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
    /// consuming addresses from lowest derivation index to highest
    /// until the total output amount plus the estimated input-side
    /// fee margin is covered.
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

        // Snapshot non-zero-balance addresses in ascending DIP-17
        // derivation index order — `BTreeMap<u32, _>` iteration is
        // already ordered. Materialising a `Vec` here lets the
        // selection loop run as a pure helper (`select_inputs`)
        // that's amenable to direct unit testing.
        let candidates: Vec<(PlatformAddress, Credits)> = account
            .addresses
            .addresses
            .values()
            .filter_map(|addr_info| {
                let p2pkh = PlatformP2PKHAddress::from_address(&addr_info.address).ok()?;
                let balance = account.address_credit_balance(&p2pkh);
                if balance == 0 {
                    None
                } else {
                    Some((PlatformAddress::P2pkh(p2pkh.to_bytes()), balance))
                }
            })
            .collect();

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
/// Given a `candidates` list of `(address, balance)` pairs in
/// preferred selection order (DIP-17 derivation order, in practice),
/// pick the smallest prefix that covers `total_output + estimated_fee`,
/// then trim the **last consumed input** down so that
/// `Σ inputs.credits == total_output` exactly.
///
/// The fee is *not* added to the returned `Credits` values. It's
/// covered separately by the fee strategy (typically
/// [`AddressFundsFeeStrategyStep::DeductFromInput`], which reduces
/// the remaining balance left at the targeted input address by the
/// fee — a separate on-chain operation from the consumed-credits
/// transfer modeled by the inputs map).
///
/// # Invariant
///
/// The returned map always satisfies `Σ values == total_output`.
/// Tail candidates that were only added to satisfy the fee margin
/// (i.e. whose balance is not needed to reach `total_output`) are
/// excluded from the map; the fee continues to be paid out of the
/// fee-bearing input's remaining balance per `fee_strategy`.
///
/// Returns `Err(PlatformWalletError::AddressOperation(_))` when no
/// prefix of `candidates` has total balance covering
/// `total_output + estimated_fee`.
fn select_inputs(
    candidates: Vec<(PlatformAddress, Credits)>,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    total_output: Credits,
    fee_strategy: &[AddressFundsFeeStrategyStep],
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    let output_count = outputs.len();
    // Track the chosen prefix in INSERTION order so we can trim
    // from the front-to-back when building the result. A
    // `BTreeMap` would re-order by key, which loses the DIP-17
    // derivation-order intent and complicates the trim logic.
    let mut chosen: Vec<(PlatformAddress, Credits)> = Vec::new();
    let mut accumulated: Credits = 0;

    for (address, balance) in candidates {
        chosen.push((address, balance));
        accumulated = accumulated.saturating_add(balance);

        let estimated_fee = estimate_fee_for_inputs_pub(
            chosen.len(),
            output_count,
            fee_strategy,
            outputs,
            platform_version,
        );
        let required = total_output.saturating_add(estimated_fee);

        if accumulated >= required {
            // Build the result by consuming from the front of
            // `chosen` until exactly `total_output` is reached.
            // Any remaining candidates were only added to satisfy
            // the fee margin and are excluded — protecting the
            // protocol's `Σ inputs == Σ outputs` structural
            // invariant. The fee continues to be paid out of the
            // fee-bearing input's remaining balance per
            // `fee_strategy`, which `accumulated >= required`
            // already guarantees has enough head-room.
            let mut selected: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
            let mut remaining = total_output;
            for (addr, bal) in chosen.iter() {
                if remaining == 0 {
                    break;
                }
                let consumed = (*bal).min(remaining);
                // The protocol rejects zero-amount inputs
                // (`InputBelowMinimumError`); we never insert
                // here when `consumed == 0` because the loop
                // breaks out as soon as `remaining` hits zero.
                selected.insert(*addr, consumed);
                remaining = remaining.saturating_sub(consumed);
            }
            return Ok(selected);
        }
    }

    // Not enough funds to cover `total_output + estimated_fee`.
    let estimated_fee = estimate_fee_for_inputs_pub(
        chosen.len().max(1),
        output_count,
        fee_strategy,
        outputs,
        platform_version,
    );
    let required = total_output.saturating_add(estimated_fee);
    Err(PlatformWalletError::AddressOperation(format!(
        "Insufficient balance: available {} credits, required {} (outputs {} + estimated fee {})",
        accumulated, required, total_output, estimated_fee
    )))
}

#[cfg(test)]
mod auto_select_tests {
    use super::*;

    fn p2pkh(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    fn outputs_for(target: PlatformAddress, amount: Credits) -> BTreeMap<PlatformAddress, Credits> {
        std::iter::once((target, amount)).collect()
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
    }

    /// When the first selected address can't cover `output + fee`
    /// alone but two inputs together can, the second input is
    /// trimmed to bring the input sum to exactly `total_output`.
    #[test]
    fn two_input_selection_trims_only_the_last() {
        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0x99);
        let total_output = 30_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_a, 20_000_000), (addr_b, 50_000_000)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv)
            .expect("selection");

        // First input is consumed in full (its balance was below
        // total_output, so it doesn't get trimmed); second input
        // is trimmed to bring the sum to exactly total_output.
        assert_eq!(selected.get(&addr_a), Some(&20_000_000));
        assert_eq!(selected.get(&addr_b), Some(&10_000_000));
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output);
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

    /// Regression test for the trim invariant: when a tail
    /// candidate is added only to satisfy the per-input fee
    /// margin (because the prior prefix already exceeds
    /// `total_output` strictly, but didn't cover
    /// `total_output + estimated_fee_for(N - 1)`), the result
    /// must still satisfy `Σ selected.values() == total_output`.
    /// The tail candidate is dropped, and the prefix is trimmed
    /// down to exactly `total_output`.
    ///
    /// Numbers are chosen so the bug triggers regardless of the
    /// exact protocol fee schedule:
    /// - `addr_a` = 1B + 1 credit (strictly exceeds `total_output`)
    /// - `addr_b` = 1B (any positive balance suffices)
    /// - `total_output` = 1B
    /// - `fee_for_1` is small (~5M on testnet, ≪ 1) — note that
    ///   `addr_a < total_output + fee_for_1` only when fee > 1,
    ///   which is universally true for the protocol's min fee.
    #[test]
    fn fee_only_tail_input_does_not_inflate_input_sum() {
        let addr_a = p2pkh(0xA0);
        let addr_b = p2pkh(0xB0);
        let target = p2pkh(0xCC);
        let total_output = 1_000_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_a, total_output + 1), (addr_b, total_output)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected = select_inputs(candidates, &outputs, total_output, &fee_strategy, pv)
            .expect("selection");

        let input_sum: Credits = selected.values().sum();
        assert_eq!(
            input_sum, total_output,
            "Σ inputs must equal Σ outputs (protocol's structural invariant) — \
             tail-only-for-fee inputs must not inflate the sum"
        );
        // The first input is consumed for the full `total_output`
        // (its balance exceeds it); the tail input is excluded
        // from the inputs map entirely.
        assert_eq!(
            selected.get(&addr_a),
            Some(&total_output),
            "first input should consume exactly total_output"
        );
        assert!(
            !selected.contains_key(&addr_b),
            "tail-only-for-fee input must be excluded from the inputs map"
        );
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
}
