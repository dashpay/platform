use std::collections::BTreeMap;

use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
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
    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw(
        &self,
        account_index: u32,
        input_selection: InputSelection,
        output_script: CoreScript,
        core_fee_per_byte: u32,
        fee_strategy: AddressFundsFeeStrategy,
        platform_version: Option<&PlatformVersion>,
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
                        self,
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
                        self,
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
                        self,
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
                            cs.addresses.insert(
                                *addr,
                                dash_sdk::platform::address_sync::AddressFunds {
                                    balance: ai.balance,
                                    nonce: ai.nonce,
                                },
                            );
                        }
                        None => {
                            if let PlatformAddress::P2pkh(hash) = addr {
                                let p2pkh = PlatformP2PKHAddress::new(*hash);
                                account.set_address_credit_balance(p2pkh, 0, key_source.as_ref());
                            }
                            cs.addresses.insert(
                                *addr,
                                dash_sdk::platform::address_sync::AddressFunds {
                                    balance: 0,
                                    nonce: 0,
                                },
                            );
                        }
                    }
                }
            }
        }

        Ok(cs)
    }

    /// Auto-select all funded addresses for withdrawal. Withdrawals consume
    /// all input balances (minus the fee), so we select every funded address
    /// and verify there's enough to cover the fee.
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

        for (_, addr_info) in &account.addresses.addresses {
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

        // Verify the total covers the fee.
        let estimated_fee = AddressCreditWithdrawalTransition::estimate_min_fee(
            selected.len(),
            false, // no change output
            platform_version,
        );

        // Only check if fee comes from inputs.
        let fee_from_inputs = fee_strategy
            .iter()
            .any(|s| matches!(s, AddressFundsFeeStrategyStep::DeductFromInput(_)));

        if fee_from_inputs && accumulated < estimated_fee {
            return Err(PlatformWalletError::AddressOperation(format!(
                "Insufficient balance for withdrawal fee: available {} credits, fee {}",
                accumulated, estimated_fee
            )));
        }

        Ok(selected)
    }
}
