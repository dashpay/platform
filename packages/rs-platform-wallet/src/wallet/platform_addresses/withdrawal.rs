use std::collections::BTreeMap;

use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use dpp::version::PlatformVersion;
use dpp::version::LATEST_PLATFORM_VERSION;
use dpp::withdrawal::Pooling;
use key_wallet::PlatformP2PKHAddress;

use super::InputSelection;
use crate::wallet::PlatformAddressWallet;
use crate::{PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::transition::address_credit_withdrawal::WithdrawAddressFunds;

impl PlatformAddressWallet {
    /// Withdraw platform credits to a Core L1 address.
    ///
    /// Input addresses can be specified explicitly or selected automatically
    /// from the account via [`InputSelection::Auto`].
    ///
    /// If `platform_version` is `None`, the latest platform version's fee
    /// schedule is used for fee estimation during auto-selection.
    ///
    /// `address_signer` produces ECDSA signatures for the input
    /// [`PlatformAddress`]es; the wallet struct carries no key material
    /// itself (see the type-level docs on
    /// [`PlatformAddressWallet`]).
    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        account_index: u32,
        input_selection: InputSelection,
        output_script: CoreScript,
        core_fee_per_byte: u32,
        fee_strategy: AddressFundsFeeStrategy,
        platform_version: Option<&PlatformVersion>,
        address_signer: &S,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError> {
        // Validate that the output script is a supported type (P2PKH or P2SH).
        if !output_script.is_p2pkh() && !output_script.is_p2sh() {
            return Err(PlatformWalletError::AddressOperation(
                "Output script must be P2PKH or P2SH".to_string(),
            ));
        }

        let version = platform_version.unwrap_or(LATEST_PLATFORM_VERSION);

        let address_infos = match input_selection {
            InputSelection::Explicit(inputs) => {
                if inputs.is_empty() {
                    return Err(PlatformWalletError::AddressOperation(
                        "Withdrawal requires at least one input address".to_string(),
                    ));
                }
                self.sdk
                    .withdraw_address_funds(
                        inputs,
                        None,
                        fee_strategy,
                        core_fee_per_byte,
                        Pooling::Never,
                        output_script,
                        address_signer,
                        None,
                    )
                    .await?
            }
            InputSelection::ExplicitWithNonces(inputs) => {
                if inputs.is_empty() {
                    return Err(PlatformWalletError::AddressOperation(
                        "Withdrawal requires at least one input address".to_string(),
                    ));
                }
                self.sdk
                    .withdraw_address_funds_with_nonce(
                        inputs,
                        None,
                        fee_strategy,
                        core_fee_per_byte,
                        Pooling::Never,
                        output_script,
                        address_signer,
                        None,
                    )
                    .await?
            }
            InputSelection::Auto => {
                // The AUTO path owns its own fee strategy: it picks the
                // fee-source input by balance (largest selected input) and
                // emits the matching `DeductFromInput(index)`, ignoring the
                // caller's `fee_strategy`. The caller cannot know the final
                // BTreeMap ordering of auto-selected inputs, so trusting a
                // hardcoded index (e.g. the wrapper's `DeductFromInput(0)`,
                // which resolves to the lex-smallest address regardless of
                // balance) would reserve the fee on an arbitrarily small
                // input and reject otherwise-fundable withdrawals.
                let (inputs, auto_fee_strategy) = self
                    .auto_select_inputs_for_withdrawal(account_index, version)
                    .await?;
                self.sdk
                    .withdraw_address_funds(
                        inputs,
                        None,
                        auto_fee_strategy,
                        core_fee_per_byte,
                        Pooling::Never,
                        output_script,
                        address_signer,
                        None,
                    )
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

    /// Auto-select all funded addresses for withdrawal.
    ///
    /// The per-input `Credits` value in the returned map is the amount to
    /// *withdraw* from that address, not its on-chain balance. The chain
    /// deducts the transition fee from each input's **remaining** balance
    /// (`on_chain_balance − withdraw_amount`), so a withdraw amount equal to
    /// the full balance leaves zero remaining and is rejected with
    /// `fee_fully_covered = false` — see
    /// `test_exact_balance_withdrawal_fails_insufficient_remaining_for_fees`
    /// in the drive-abci address-credit-withdrawal tests, and the transfer
    /// path's `select_inputs_deduct_from_input` for the same invariant.
    ///
    /// We therefore select every funded address at its full balance, then
    /// reduce the withdraw amount on the **largest-balance** selected input
    /// by the estimated fee so that input keeps `≥ estimated_fee` of
    /// remaining balance for the chain to deduct. The largest input is the
    /// most likely to absorb the fee while staying above `min_input_amount`,
    /// so picking it (rather than the lexicographically-smallest index-0
    /// entry) avoids rejecting an otherwise-fundable withdrawal when the
    /// lex-smallest input happens to be tiny.
    ///
    /// Returns the adjusted withdraw-amount map together with the fee
    /// strategy that targets the fee-source input. The AUTO path owns this
    /// strategy because only it knows the final BTreeMap ordering of the
    /// auto-selected inputs (and therefore which `DeductFromInput(index)`
    /// resolves to the largest input).
    async fn auto_select_inputs_for_withdrawal(
        &self,
        account_index: u32,
        platform_version: &PlatformVersion,
    ) -> Result<(BTreeMap<PlatformAddress, Credits>, AddressFundsFeeStrategy), PlatformWalletError>
    {
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

        // Select all funded addresses.
        let mut selected = BTreeMap::new();
        let mut accumulated: Credits = 0;

        for addr_info in account.addresses.addresses.values() {
            if let Ok(p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address) {
                let balance = account.address_credit_balance(&p2pkh);
                if balance > 0 {
                    let address = PlatformAddress::P2pkh(p2pkh.to_bytes());
                    selected.insert(address, balance);
                    accumulated = accumulated.saturating_add(balance);
                }
            }
        }

        if selected.is_empty() {
            return Err(PlatformWalletError::AddressOperation(
                "No funded addresses available for withdrawal".to_string(),
            ));
        }

        reserve_withdrawal_fee_on_largest_input(selected, platform_version)
    }
}

/// Convert a full-balance input map into a withdraw-amount map that leaves the
/// chain enough fee headroom on the fee-source input, and compute the fee
/// strategy that targets that input.
///
/// `selected` maps each chosen input address to its **full on-chain balance**.
/// The chain deducts the transition fee from each input's *remaining* balance
/// (`on_chain_balance − withdraw_amount`); since the auto path has no change
/// output, withdrawing the full balance everywhere leaves zero remaining and
/// the chain rejects the transition with `fee_fully_covered = false`. We reduce
/// the withdraw amount on the **largest-balance** selected input by the
/// estimated fee, so that input retains exactly `estimated_fee` of remaining
/// balance for the chain to deduct. This mirrors the transfer path's
/// `select_inputs_deduct_from_input` invariant: the `DeductFromInput` target
/// must keep `balance − consumed ≥ estimated_fee`.
///
/// Picking the largest input as the fee source (rather than the
/// lexicographically-smallest index-0 entry) is what makes an otherwise-
/// fundable withdrawal succeed: the on-chain `DeductFromInput(index)` resolves
/// against BTreeMap iteration order, which is address-hash ordering — unrelated
/// to balance. A tiny lex-smallest input could fail to absorb the fee even
/// when a much larger peer trivially could. We therefore locate the largest
/// input, then emit `DeductFromInput(<its position in BTreeMap order>)`.
///
/// Returns the adjusted withdraw-amount map and the fee strategy targeting the
/// fee-source input, or a typed [`PlatformWalletError::AddressOperation`] when
/// no input can absorb the fee while respecting the per-input minimum /
/// minimum withdrawal amount.
fn reserve_withdrawal_fee_on_largest_input(
    mut selected: BTreeMap<PlatformAddress, Credits>,
    platform_version: &PlatformVersion,
) -> Result<(BTreeMap<PlatformAddress, Credits>, AddressFundsFeeStrategy), PlatformWalletError> {
    let accumulated: Credits = selected
        .values()
        .copied()
        .fold(0, |acc, b| acc.saturating_add(b));

    // Estimate the transition fee for the selected input count (no change
    // output on the auto path).
    let estimated_fee = AddressCreditWithdrawalTransition::estimate_min_fee(
        selected.len(),
        false, // no change output
        platform_version,
    );

    // Locate the fee-source input: the largest balance, ties broken by the
    // first in BTreeMap (address-hash) order so the choice is deterministic.
    // `max_by_key` returns the *last* maximal element on ties, so iterate and
    // keep the first occurrence of the maximum explicitly.
    let (fee_source_index, fee_source_addr, fee_source_balance) = selected
        .iter()
        .enumerate()
        .fold(None, |best, (idx, (&addr, &balance))| match best {
            Some((_, _, best_balance)) if best_balance >= balance => best,
            _ => Some((idx, addr, balance)),
        })
        .expect("selected is non-empty: callers reject empty input maps");

    // The reduced fee-source amount must still be ≥ `min_input_amount`, and the
    // overall withdrawal (accumulated − estimated_fee) must clear the minimum
    // withdrawal amount, otherwise the transition is rejected on-chain.
    let min_input_amount = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_input_amount;
    let min_withdrawal_amount = platform_version.system_limits.min_withdrawal_amount;

    let withdraw_total = accumulated.saturating_sub(estimated_fee);
    if accumulated <= estimated_fee || withdraw_total < min_withdrawal_amount {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Insufficient balance for withdrawal fee: available {} credits, \
             estimated fee {}, leaving {} below the minimum withdrawal amount {}",
            accumulated, estimated_fee, withdraw_total, min_withdrawal_amount
        )));
    }

    let fee_source_amount = fee_source_balance.saturating_sub(estimated_fee);
    if fee_source_amount < min_input_amount {
        // The largest input cannot absorb the fee while staying above the
        // per-input minimum, so no input can: a genuine insufficiency.
        return Err(PlatformWalletError::AddressOperation(format!(
            "Cannot reserve withdrawal fee on the fee-source input: largest input \
             balance {} minus estimated fee {} leaves {}, below the minimum input \
             amount {}. Consolidate funds onto fewer addresses or fund the largest \
             address more before withdrawing.",
            fee_source_balance, estimated_fee, fee_source_amount, min_input_amount
        )));
    }

    // Same key → BTreeMap ordering (and thus the index resolution below) is
    // preserved; only the withdraw amount on the fee-source input shrinks.
    selected.insert(fee_source_addr, fee_source_amount);

    let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(
        fee_source_index as u16,
    )];

    Ok((selected, fee_strategy))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `PlatformAddress::P2pkh` is `Ord`-derived, so a smaller leading byte sorts
    /// first in the BTreeMap and becomes the `DeductFromInput(0)` target.
    fn addr(first_byte: u8) -> PlatformAddress {
        let mut bytes = [0u8; 20];
        bytes[0] = first_byte;
        PlatformAddress::P2pkh(bytes)
    }

    fn estimated_fee(input_count: usize, pv: &PlatformVersion) -> Credits {
        AddressCreditWithdrawalTransition::estimate_min_fee(input_count, false, pv)
    }

    /// A single funded input must keep `estimated_fee` of headroom: the withdraw
    /// amount on the fee-source input is its balance minus the estimated fee, NOT
    /// the full balance (which would leave zero remaining → `fee_fully_covered =
    /// false` on-chain). With one input it is trivially the largest, so the
    /// emitted strategy targets index 0.
    #[test]
    fn reserves_fee_headroom_on_single_input() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(1, pv);
        let balance = fee + dpp::dash_to_credits!(1.0);

        let mut input = BTreeMap::new();
        input.insert(addr(1), balance);

        let (result, strategy) = reserve_withdrawal_fee_on_largest_input(input, pv)
            .expect("single funded input above the fee should select");

        assert_eq!(result.get(&addr(1)).copied(), Some(balance - fee));
        assert_eq!(
            strategy,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            "the single input is the fee source at index 0"
        );
    }

    /// The reviewer's scenario, corrected: input[0] (lex-smallest, BTreeMap
    /// index 0) is much smaller than the fee while a larger peer exists. The fee
    /// must now be reserved on the LARGER peer (the fee source picked by
    /// balance), so the small lex-smallest input is withdrawn in full and the
    /// larger input's withdraw amount drops by the fee. The emitted strategy
    /// must target the larger input's BTreeMap index, NOT index 0.
    #[test]
    fn reserves_fee_on_largest_input_even_when_lex_smallest_is_tiny() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(2, pv);
        // Small lex-smallest input: too small to absorb the fee on its own
        // (would have failed the old index-0 path), but withdrawn in full here.
        let small = dpp::dash_to_credits!(0.001);
        let large = dpp::dash_to_credits!(10.0);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), small); // lex-smallest → BTreeMap index 0
        inputs.insert(addr(9), large); // larger → BTreeMap index 1

        let (result, strategy) = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect("the larger peer can absorb the fee");

        assert_eq!(
            result.get(&addr(1)).copied(),
            Some(small),
            "the small lex-smallest input is withdrawn in full"
        );
        assert_eq!(
            result.get(&addr(9)).copied(),
            Some(large - fee),
            "the fee is reserved on the largest input"
        );
        assert_eq!(
            strategy,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(1)],
            "the emitted DeductFromInput index points at the largest input (BTreeMap index 1)"
        );
    }

    /// The emitted `DeductFromInput` index points at the largest input even when
    /// that input is NOT the last in BTreeMap (address-hash) order — i.e. the
    /// balance ranking and the address-hash ranking disagree.
    #[test]
    fn emitted_index_points_at_largest_input_not_last() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(3, pv);
        let large = dpp::dash_to_credits!(10.0);
        let small_a = dpp::dash_to_credits!(0.01);
        let small_b = dpp::dash_to_credits!(0.02);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), large); // lex-smallest → BTreeMap index 0, largest balance
        inputs.insert(addr(5), small_a); // index 1
        inputs.insert(addr(9), small_b); // index 2

        let (result, strategy) = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect("the largest input can absorb the fee");

        assert_eq!(
            strategy,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            "the largest input is at BTreeMap index 0, so the fee deducts from index 0"
        );
        assert_eq!(result.get(&addr(1)).copied(), Some(large - fee));
        assert_eq!(result.get(&addr(5)).copied(), Some(small_a));
        assert_eq!(result.get(&addr(9)).copied(), Some(small_b));
    }

    /// Genuine insufficiency: even the LARGEST input cannot retain
    /// `estimated_fee` while keeping its withdraw amount ≥ `min_input_amount`,
    /// so no input can. We error rather than ship a guaranteed-rejected
    /// transition (mirrors the transfer path's headroom error). The aggregate
    /// here clears `min_withdrawal_amount`, so the error is specifically the
    /// per-input headroom failure, not the aggregate-too-small gate.
    #[test]
    fn errors_when_largest_input_too_small_to_absorb_fee() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(3, pv);
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        let min_withdrawal = pv.system_limits.min_withdrawal_amount;

        // Largest input leaves < min_input after the fee is reserved.
        let large = fee + min_input - 1;
        // Two equal peers, each smaller than `large` so it stays the maximum,
        // sized so the aggregate clears `min_withdrawal_amount + fee`.
        let peer = large / 2;

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), peer);
        inputs.insert(addr(5), peer);
        inputs.insert(addr(9), large);

        // Sanity: the aggregate clears the withdrawal minimum, so the only
        // remaining failure path is the largest-input headroom check.
        let accumulated = peer + peer + large;
        assert!(
            accumulated.saturating_sub(fee) >= min_withdrawal,
            "test setup: aggregate must clear the withdrawal minimum"
        );

        let err = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect_err("largest input below fee + min_input must error");
        assert!(matches!(err, PlatformWalletError::AddressOperation(_)));
    }

    /// Aggregate balance below the fee (or leaving less than the minimum
    /// withdrawal amount) is rejected up front.
    #[test]
    fn errors_when_total_below_fee() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(1, pv);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), fee - 1);

        let err = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect_err("balance below the fee must error");
        assert!(matches!(err, PlatformWalletError::AddressOperation(_)));
    }
}
