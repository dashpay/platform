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
                let inputs = self
                    .auto_select_inputs_for_withdrawal(account_index, &fee_strategy, version)
                    .await?;
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
    /// We therefore select every funded address at its full balance, then,
    /// for a `DeductFromInput`-based fee strategy, reduce the withdraw
    /// amount on the fee-source input (the BTreeMap index-0 / lex-smallest
    /// entry that `DeductFromInput(0)` resolves to) by the estimated fee so
    /// that input keeps `≥ estimated_fee` of remaining balance for the chain
    /// to deduct. The withdrawn total is the account balance minus the fee.
    async fn auto_select_inputs_for_withdrawal(
        &self,
        account_index: u32,
        fee_strategy: &[AddressFundsFeeStrategyStep],
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
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

        reserve_withdrawal_fee_on_fee_source(selected, fee_strategy, platform_version)
    }
}

/// Convert a full-balance input map into a withdraw-amount map that leaves the
/// chain enough fee headroom on the fee-source input.
///
/// `selected` maps each chosen input address to its **full on-chain balance**.
/// The chain deducts the transition fee from each input's *remaining* balance
/// (`on_chain_balance − withdraw_amount`); since the auto path has no change
/// output, withdrawing the full balance everywhere leaves zero remaining and
/// the chain rejects the transition with `fee_fully_covered = false`. We reduce
/// the withdraw amount on the fee-source input — the BTreeMap entry the first
/// `DeductFromInput(index)` step resolves to — by the estimated fee, so that
/// input retains exactly `estimated_fee` of remaining balance for the chain to
/// deduct. This mirrors the transfer path's `select_inputs_deduct_from_input`
/// invariant: the `DeductFromInput` target must keep `balance − consumed ≥
/// estimated_fee`.
///
/// Returns the adjusted withdraw-amount map, or a typed
/// [`PlatformWalletError::AddressOperation`] when no input can absorb the fee
/// while respecting the per-input minimum / minimum withdrawal amount.
fn reserve_withdrawal_fee_on_fee_source(
    mut selected: BTreeMap<PlatformAddress, Credits>,
    fee_strategy: &[AddressFundsFeeStrategyStep],
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
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

    // The fee-source input is the first `DeductFromInput` step's target index
    // (production always sends `[DeductFromInput(0)]`). If the fee strategy
    // never deducts from an input (e.g. `ReduceOutput`-only, which the auto
    // path doesn't build today since there is no output), no input headroom is
    // required and we withdraw every balance in full.
    let fee_source_index = fee_strategy.iter().find_map(|s| match s {
        AddressFundsFeeStrategyStep::DeductFromInput(index) => Some(*index as usize),
        AddressFundsFeeStrategyStep::ReduceOutput(_) => None,
    });

    let Some(fee_source_index) = fee_source_index else {
        return Ok(selected);
    };

    // Resolve the fee-source address by BTreeMap iteration order, matching how
    // the chain's `deduct_fee_from_outputs_or_remaining_balance_of_inputs`
    // resolves `DeductFromInput(index)` against the input map.
    let Some((&fee_source_addr, &fee_source_balance)) = selected.iter().nth(fee_source_index)
    else {
        // Out-of-range index would be rejected by structure validation; surface
        // a typed wallet-side error instead of shipping a doomed transition.
        return Err(PlatformWalletError::AddressOperation(format!(
            "Fee strategy DeductFromInput({}) is out of range for {} selected input(s)",
            fee_source_index,
            selected.len()
        )));
    };

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
        return Err(PlatformWalletError::AddressOperation(format!(
            "Cannot reserve withdrawal fee on the fee-source input: balance {} \
             minus estimated fee {} leaves {}, below the minimum input amount {}. \
             Consolidate funds onto fewer addresses or fund the smallest address \
             more before withdrawing.",
            fee_source_balance, estimated_fee, fee_source_amount, min_input_amount
        )));
    }

    // Same key → BTreeMap ordering (and thus the index-0 resolution above) is
    // preserved; only the withdraw amount on the fee-source input shrinks.
    selected.insert(fee_source_addr, fee_source_amount);

    Ok(selected)
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
    /// false` on-chain).
    #[test]
    fn reserves_fee_headroom_on_single_input() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(1, pv);
        let balance = fee + dpp::dash_to_credits!(1.0);

        let mut input = BTreeMap::new();
        input.insert(addr(1), balance);

        let result = reserve_withdrawal_fee_on_fee_source(
            input,
            &[AddressFundsFeeStrategyStep::DeductFromInput(0)],
            pv,
        )
        .expect("single funded input above the fee should select");

        assert_eq!(result.get(&addr(1)).copied(), Some(balance - fee));
    }

    /// The reviewer's scenario: input[0] (lex-smallest, the `DeductFromInput(0)`
    /// target) is much smaller than the fee while a larger input exists. The fee
    /// must be reserved on input[0] itself (the index the chain deducts from), so
    /// input[0]'s withdraw amount drops by the fee and the larger input is
    /// withdrawn in full. Both must stay ≥ `min_input_amount`.
    #[test]
    fn reserves_fee_on_lex_smallest_input_even_when_a_larger_input_exists() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(2, pv);
        // Small input[0] still large enough to absorb the fee + keep min_input.
        let small = fee + dpp::dash_to_credits!(0.5);
        let large = dpp::dash_to_credits!(10.0);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), small); // lex-smallest → index 0
        inputs.insert(addr(9), large);

        let result = reserve_withdrawal_fee_on_fee_source(
            inputs,
            &[AddressFundsFeeStrategyStep::DeductFromInput(0)],
            pv,
        )
        .expect("fee-source input can absorb the fee");

        assert_eq!(
            result.get(&addr(1)).copied(),
            Some(small - fee),
            "fee is reserved on the lex-smallest (index-0) input"
        );
        assert_eq!(
            result.get(&addr(9)).copied(),
            Some(large),
            "the larger non-fee-source input is withdrawn in full"
        );
    }

    /// When the fee-source input cannot retain `estimated_fee` while keeping its
    /// withdraw amount ≥ `min_input_amount`, we error rather than ship a
    /// guaranteed-rejected transition (mirrors the transfer path's headroom error).
    #[test]
    fn errors_when_fee_source_input_too_small_to_absorb_fee() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(2, pv);
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        // Fee-source balance leaves < min_input after the fee is reserved.
        let small = fee + min_input - 1;
        let large = dpp::dash_to_credits!(10.0);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), small);
        inputs.insert(addr(9), large);

        let err = reserve_withdrawal_fee_on_fee_source(
            inputs,
            &[AddressFundsFeeStrategyStep::DeductFromInput(0)],
            pv,
        )
        .expect_err("fee-source input below fee + min_input must error");
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

        let err = reserve_withdrawal_fee_on_fee_source(
            inputs,
            &[AddressFundsFeeStrategyStep::DeductFromInput(0)],
            pv,
        )
        .expect_err("balance below the fee must error");
        assert!(matches!(err, PlatformWalletError::AddressOperation(_)));
    }
}
