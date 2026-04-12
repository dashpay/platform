use crate::wallet::platform_addresses::provider::PlatformPaymentAddressAccountProvider;
use crate::wallet::PlatformAddressWallet;
use crate::{Merge, PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::address_sync::AddressSyncResult;

impl PlatformAddressWallet {
    /// Sync platform address balances across all active platform payment accounts.
    ///
    /// Iterates every platform payment account in the wallet and calls
    /// [`sync_balances_on_account_index`](Self::sync_balances_on_account_index)
    /// for each one. The returned changeset is the merged result of all accounts.
    pub async fn sync_balances(
        &self,
    ) -> Result<(Vec<AddressSyncResult>, PlatformAddressChangeSet), PlatformWalletError> {
        // Collect account indices under a short-lived read lock.
        let account_indices: Vec<u32> = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "Wallet {:?} not found in wallet manager",
                    hex::encode(self.wallet_id)
                ))
            })?;
            info.core_wallet
                .accounts
                .platform_payment_accounts
                .keys()
                .map(|k| k.account)
                .collect()
        };

        let mut all_results = Vec::with_capacity(account_indices.len());
        let mut merged_cs = PlatformAddressChangeSet::default();

        for account_index in account_indices {
            let (result, cs) = self.sync_balances_on_account_index(account_index).await?;
            all_results.push(result);
            merged_cs.merge(cs);
        }

        Ok((all_results, merged_cs))
    }

    /// Sync platform address balances for a single account.
    ///
    /// Uses the SDK's privacy-preserving trunk/branch address synchronization
    /// with DIP-17 address discovery via gap limit scanning.
    ///
    /// Returns both the raw [`AddressSyncResult`] and a
    /// [`PlatformAddressChangeSet`] describing every address update /
    /// tombstone caused by the sync.
    pub async fn sync_balances_on_account_index(
        &self,
        account_index: u32,
    ) -> Result<(AddressSyncResult, PlatformAddressChangeSet), PlatformWalletError> {
        // Take the existing provider or create a new one.
        let mut provider = {
            let mut providers = self.providers.lock().await;
            match providers.remove(&account_index) {
                Some(mut existing) => {
                    existing.prepare_for_sync()?;
                    existing
                }
                None => PlatformPaymentAddressAccountProvider::from_wallet(
                    self.wallet_manager.clone(),
                    self.wallet_id,
                    account_index,
                )?,
            }
        };

        let result = self
            .sdk
            .sync_address_balances(&mut provider, None, None)
            .await?;

        // Build the changeset from the sync results.
        // Note: balances are already written to the ManagedPlatformAccount
        // by the provider's on_address_found callback during sync.
        let mut cs = PlatformAddressChangeSet::default();
        for ((_, address), funds) in &result.found {
            cs.addresses.insert(*address, funds.balance);
        }

        // Update the provider's incremental state from the result.
        provider.update_sync_state(&result);

        // Store the provider back so incremental state is retained for next sync.
        {
            let mut providers = self.providers.lock().await;
            providers.insert(account_index, provider);
        }

        Ok((result, cs))
    }
}
