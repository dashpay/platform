use crate::wallet::PlatformAddressWallet;
use crate::{Merge, PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::address_sync::{AddressSyncConfig, AddressSyncResult};

impl PlatformAddressWallet {
    /// Sync platform address balances across all active platform payment accounts.
    ///
    /// Iterates every platform payment account in the wallet and calls
    /// [`sync_balances_on_account_index`](Self::sync_balances_on_account_index)
    /// for each one. The returned changeset is the merged result of all accounts.
    ///
    /// The same `config` is shared across all accounts. Pass `None` for defaults.
    pub async fn sync_balances(
        &self,
        config: Option<AddressSyncConfig>,
    ) -> Result<(Vec<AddressSyncResult>, PlatformAddressChangeSet), PlatformWalletError> {
        let providers = self.providers.load();
        let account_indices: Vec<u32> = providers.keys().copied().collect();

        let mut all_results = Vec::with_capacity(account_indices.len());
        let mut merged_cs = PlatformAddressChangeSet::default();

        for account_index in account_indices {
            let (result, cs) = self
                .sync_balances_on_account_index(account_index, config.clone())
                .await?;
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
    ///
    /// Pass `None` for `config` to use defaults.
    pub async fn sync_balances_on_account_index(
        &self,
        account_index: u32,
        config: Option<AddressSyncConfig>,
    ) -> Result<(AddressSyncResult, PlatformAddressChangeSet), PlatformWalletError> {
        let providers = self.providers.load();
        let provider_lock = providers.get(&account_index).ok_or_else(|| {
            PlatformWalletError::AddressSync(format!(
                "No provider for account index {}",
                account_index
            ))
        })?;

        let mut provider = provider_lock.write().await;
        let last_sync_timestamp = provider.last_sync_timestamp();
        provider.prepare_for_sync().await?;

        let result = self
            .sdk
            .sync_address_balances(&mut *provider, config, last_sync_timestamp)
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

        Ok((result, cs))
    }
}
