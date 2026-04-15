use std::collections::BTreeMap;

use crate::wallet::PlatformAddressWallet;
use crate::{Merge, PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::address_sync::{AddressSyncConfig, AddressSyncResult};

impl PlatformAddressWallet {
    /// Sync platform address balances across all active platform payment accounts.
    ///
    /// Iterates every platform payment account in the wallet and calls
    /// [`sync_balances_on_account_index`](Self::sync_balances_on_account_index)
    /// for each one. The changeset is persisted internally via the wallet persister.
    ///
    /// The same `config` is shared across all accounts. Pass `None` for defaults.
    ///
    /// Returns a map of account index to sync result.
    pub async fn sync_balances(
        &self,
        config: Option<AddressSyncConfig>,
    ) -> Result<BTreeMap<u32, AddressSyncResult>, PlatformWalletError> {
        let providers = self.providers.load();
        let account_indices: Vec<u32> = providers.keys().copied().collect();

        let mut all_results = BTreeMap::new();
        let mut merged_cs = PlatformAddressChangeSet::default();

        for account_index in account_indices {
            let (result, cs) = self.sync_on_account(account_index, config.clone()).await?;
            all_results.insert(account_index, result);
            merged_cs.merge(cs);
        }

        // Persist the merged changeset.
        if !merged_cs.is_empty() {
            if let Err(e) = self.persister.store(merged_cs.into()) {
                tracing::error!("Failed to persist address sync changeset: {}", e);
            }
        }

        Ok(all_results)
    }

    /// Sync platform address balances for a single account.
    ///
    /// Uses the SDK's privacy-preserving trunk/branch address synchronization
    /// with DIP-17 address discovery via gap limit scanning.
    ///
    /// Persists the changeset internally and returns the [`AddressSyncResult`].
    ///
    /// Pass `None` for `config` to use defaults.
    pub async fn sync_balances_on_account_index(
        &self,
        account_index: u32,
        config: Option<AddressSyncConfig>,
    ) -> Result<AddressSyncResult, PlatformWalletError> {
        let (result, cs) = self.sync_on_account(account_index, config).await?;

        // Persist the changeset.
        if !cs.is_empty() {
            if let Err(e) = self.persister.store(cs.into()) {
                tracing::error!("Failed to persist address sync changeset: {}", e);
            }
        }

        Ok(result)
    }

    /// Internal sync for a single account. Returns the raw result and changeset
    /// without persisting — callers handle persistence.
    async fn sync_on_account(
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
        // by the provider's on_address_found callback during sync; the
        // changeset carries the full AddressFunds snapshot (balance +
        // nonce) so persisters can record both in one write.
        let mut cs = PlatformAddressChangeSet::default();
        for ((_, address), funds) in &result.found {
            cs.addresses.insert(*address, *funds);
        }
        // Carry the incremental-sync watermark alongside the balances
        // so persisters can restore it on restart. Zero values mean
        // "nothing was advanced" and are omitted (leaves prior
        // watermark intact when merged).
        if result.new_sync_height > 0 {
            cs.sync_height = Some(result.new_sync_height);
        }
        if result.new_sync_timestamp > 0 {
            cs.sync_timestamp = Some(result.new_sync_timestamp);
        }

        // Update the provider's incremental state from the result.
        provider.update_sync_state(&result);

        Ok((result, cs))
    }
}
