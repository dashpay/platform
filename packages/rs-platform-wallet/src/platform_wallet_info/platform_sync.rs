//! Platform address synchronization
//!
//! This module provides methods for synchronizing platform payment account
//! balances using the privacy-preserving sync protocol.

use super::address_provider::{AccountAddressProvider, PlatformSyncResult};
use super::sync_state::PlatformSyncState;
use super::PlatformWalletInfo;
use crate::error::PlatformWalletError;
use dash_sdk::platform::address_sync::{sync_address_balances, AddressProvider, AddressSyncConfig};
use dash_sdk::platform::query::RecentAddressBalanceChangesQuery;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::RecentAddressBalanceChanges;
use dash_sdk::Sdk;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use key_wallet::managed_account::platform_address::PlatformP2PKHAddress;
use std::sync::Arc;

impl PlatformWalletInfo {
    /// Synchronize platform address balances using pre-generated addresses
    ///
    /// This performs the complete platform sync process using addresses already
    /// generated in the account's address pool. No key material is required.
    ///
    /// 1. Full sync (if needed): trunk + branch queries with privacy protection
    /// 2. Terminal sync: fetch balance changes after checkpoint
    ///
    /// The sync is performed in "auto" mode, meaning a full sync is only done if:
    /// - No previous sync has been performed
    /// - More than 6 days 20 hours have passed since the last full sync
    ///
    /// # Arguments
    /// - `sdk`: The Dash SDK instance for network queries
    /// - `sync_state`: Mutable sync state to track progress
    ///
    /// # Returns
    /// - `Ok(PlatformSyncResult)` with sync statistics
    /// - `Err(PlatformWalletError)` if sync fails
    pub async fn sync_platform_addresses(
        &mut self,
        sdk: &Arc<Sdk>,
        sync_state: &mut PlatformSyncState,
    ) -> Result<PlatformSyncResult, PlatformWalletError> {
        let current_time = Self::current_timestamp();
        let needs_full_sync = sync_state.needs_full_sync(current_time);

        let (checkpoint_height, full_sync_performed, highest_found_index, total_balance_from_full) =
            if needs_full_sync {
                // Get the platform payment account
                let account = self
                    .wallet_info
                    .first_platform_payment_managed_account_mut()
                    .ok_or(PlatformWalletError::NoPlatformPaymentAccount)?;

                // Create address provider from the account (uses pre-existing addresses)
                let mut provider = AccountAddressProvider::new(account)?;

                // Perform full privacy-preserving sync
                let result = Self::execute_full_sync(sdk, &mut provider).await?;

                // Apply results to account
                provider.apply_to_account();

                // Update sync state
                sync_state.update_after_full_sync(current_time, result.checkpoint_height);
                sync_state.highest_found_index = result.highest_found_index;

                (
                    result.checkpoint_height,
                    true,
                    result.highest_found_index,
                    Some(result.total_balance),
                )
            } else {
                (
                    sync_state.checkpoint_height,
                    false,
                    sync_state.highest_found_index,
                    None,
                )
            };

        // Terminal sync: catch balance changes after checkpoint
        let terminal_start = sync_state.terminal_sync_start_height();
        let highest_terminal_block = self.execute_terminal_sync(sdk, terminal_start).await?;

        // Update terminal block in state
        sync_state.update_last_terminal_block(highest_terminal_block);

        // Calculate total balance
        let total_balance = total_balance_from_full.unwrap_or_else(|| {
            self.wallet_info
                .first_platform_payment_managed_account()
                .map(|a| a.total_credit_balance())
                .unwrap_or(0)
        });

        let funded_count = self
            .wallet_info
            .first_platform_payment_managed_account()
            .map(|a| a.funded_address_count())
            .unwrap_or(0);

        Ok(PlatformSyncResult {
            full_sync_performed,
            checkpoint_height,
            highest_terminal_block,
            total_balance,
            funded_address_count: funded_count,
            highest_found_index,
        })
    }

    /// Execute full privacy-preserving sync (trunk + branch queries)
    async fn execute_full_sync(
        sdk: &Arc<Sdk>,
        provider: &mut AccountAddressProvider<'_>,
    ) -> Result<FullSyncResult, PlatformWalletError> {
        // Execute sync with default config
        let config = AddressSyncConfig::default();
        let result = sync_address_balances(sdk.as_ref(), provider, Some(config))
            .await
            .map_err(|e| PlatformWalletError::SyncError(e.to_string()))?;

        Ok(FullSyncResult {
            checkpoint_height: result.checkpoint_height,
            highest_found_index: provider.highest_found_index(),
            total_balance: provider.total_found_balance(),
            funded_count: provider.funded_address_count(),
        })
    }

    /// Execute terminal sync to catch balance changes after checkpoint
    async fn execute_terminal_sync(
        &mut self,
        sdk: &Arc<Sdk>,
        start_height: u64,
    ) -> Result<u64, PlatformWalletError> {
        // Fetch recent balance changes
        let query = RecentAddressBalanceChangesQuery::new(start_height);
        let changes: RecentAddressBalanceChanges =
            RecentAddressBalanceChanges::fetch(sdk.as_ref(), query)
                .await
                .map_err(|e| PlatformWalletError::SyncError(e.to_string()))?
                .unwrap_or_default();

        let mut highest_block = start_height;

        // Apply changes to wallet
        for block_changes in changes.into_inner() {
            highest_block = highest_block.max(block_changes.block_height);

            for (platform_addr, operation) in block_changes.changes {
                self.apply_credit_operation(&platform_addr, &operation)?;
            }
        }

        Ok(highest_block)
    }

    /// Apply a credit operation to an address
    fn apply_credit_operation(
        &mut self,
        platform_addr: &PlatformAddress,
        operation: &CreditOperation,
    ) -> Result<(), PlatformWalletError> {
        // Convert PlatformAddress to PlatformP2PKHAddress
        let p2pkh = match platform_addr {
            PlatformAddress::P2pkh(hash) => PlatformP2PKHAddress::new(*hash),
            PlatformAddress::P2sh(_) => {
                // P2SH addresses are not supported for platform payment accounts
                return Ok(());
            }
        };

        // Apply to account if it exists
        if let Some(account) = self
            .wallet_info
            .first_platform_payment_managed_account_mut()
        {
            // Only apply if this address belongs to our account
            if account.contains_platform_address(&p2pkh) {
                match operation {
                    CreditOperation::SetCredits(credits) => {
                        account.set_address_credit_balance(p2pkh, *credits, None);
                    }
                    CreditOperation::AddToCredits(credits) => {
                        account.add_address_credit_balance(p2pkh, *credits, None);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get current timestamp in seconds
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

/// Internal result from full sync operation
struct FullSyncResult {
    checkpoint_height: u64,
    highest_found_index: Option<u32>,
    total_balance: u64,
    #[allow(dead_code)]
    funded_count: usize,
}

#[cfg(test)]
mod tests {
    // Integration tests would require mocking the SDK
    // Unit tests for individual components are in their respective modules
}
