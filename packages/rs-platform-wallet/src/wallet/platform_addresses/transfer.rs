use std::collections::BTreeMap;

use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
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
    pub async fn transfer(
        &self,
        account_index: u32,
        input_selection: InputSelection,
        outputs: BTreeMap<PlatformAddress, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        platform_version: Option<&PlatformVersion>,
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
                    .transfer_address_funds(inputs, outputs, fee_strategy, self, None)
                    .await?
            }
            InputSelection::ExplicitWithNonces(inputs) => {
                if inputs.is_empty() {
                    return Err(PlatformWalletError::AddressOperation(
                        "Transfer requires at least one input address".to_string(),
                    ));
                }
                self.sdk
                    .transfer_address_funds_with_nonce(inputs, outputs, fee_strategy, self, None)
                    .await?
            }
            InputSelection::Auto => {
                let inputs = self
                    .auto_select_inputs(account_index, &outputs, &fee_strategy, version)
                    .await?;
                self.sdk
                    .transfer_address_funds(inputs, outputs, fee_strategy, self, None)
                    .await?
            }
        };

        // Get the cached key source from the provider for gap limit maintenance.
        let providers = self.providers.load();
        let key_source = if let Some(provider_lock) = providers.get(&account_index) {
            let provider = provider_lock.read().await;
            Some(provider.key_source().clone())
        } else {
            None
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
                    match maybe_info {
                        Some(ai) => {
                            if let PlatformAddress::P2pkh(hash) = addr {
                                let p2pkh = PlatformP2PKHAddress::new(*hash);
                                account.set_address_credit_balance(
                                    p2pkh,
                                    ai.balance,
                                    key_source.as_ref(),
                                );
                            }
                            cs.addresses.insert(*addr, ai.balance);
                        }
                        None => {
                            if let PlatformAddress::P2pkh(hash) = addr {
                                let p2pkh = PlatformP2PKHAddress::new(*hash);
                                account.set_address_credit_balance(p2pkh, 0, key_source.as_ref());
                            }
                            cs.addresses.insert(*addr, 0);
                        }
                    }
                }
            }
        }

        Ok(cs)
    }

    /// Automatically select input addresses from the account, consuming
    /// addresses from lowest derivation index to highest until the total
    /// output amount plus estimated fees is covered.
    async fn auto_select_inputs(
        &self,
        account_index: u32,
        outputs: &BTreeMap<PlatformAddress, Credits>,
        fee_strategy: &[AddressFundsFeeStrategyStep],
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
        let total_output: Credits = outputs.values().sum();
        let output_count = outputs.len();

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

        // BTreeMap<u32, _> iteration is already in ascending index order.
        let mut selected = BTreeMap::new();
        let mut accumulated: Credits = 0;

        for (_, addr_info) in &account.addresses.addresses {
            if let Ok(p2pkh) = PlatformP2PKHAddress::from_address(&addr_info.address) {
                let balance = account.address_credit_balance(&p2pkh);
                if balance == 0 {
                    continue;
                }

                let address = PlatformAddress::P2pkh(p2pkh.to_bytes());
                selected.insert(address, balance);
                accumulated = accumulated.saturating_add(balance);

                // Re-estimate fee with the current input count.
                let estimated_fee = Self::estimate_fee_for_inputs(
                    selected.len(),
                    output_count,
                    fee_strategy,
                    outputs,
                    platform_version,
                );
                let required = total_output.saturating_add(estimated_fee);

                if accumulated >= required {
                    return Ok(selected);
                }
            }
        }

        // Not enough funds.
        let estimated_fee = Self::estimate_fee_for_inputs(
            selected.len().max(1),
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

    /// Simulate the fee strategy to determine how much additional balance
    /// the inputs need beyond the output amounts.
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
