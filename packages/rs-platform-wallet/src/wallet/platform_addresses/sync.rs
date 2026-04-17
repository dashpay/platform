use crate::changeset::Merge;
use crate::wallet::{PlatformAddressTag, PlatformAddressWallet};
use crate::{PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::address_sync::{AddressSyncConfig, AddressSyncResult};
use key_wallet::PlatformP2PKHAddress;

impl PlatformAddressWallet {
    /// Sync platform address balances across every platform payment
    /// account on the wallet in a single trunk/branch/compact scan.
    ///
    /// The unified provider presents pending addresses from every
    /// account at once, so the SDK performs one combined sync instead
    /// of N per-account syncs. This is the "BLAST sync" path: fewer
    /// network round trips, one GroveDB proof covering all addresses.
    ///
    /// The change-set is computed as a diff around the SDK call — we
    /// snapshot the provider's `found` map before, run the sync, then
    /// emit any `(wallet, account, address)` whose funds changed or
    /// whose entry is new. Unchanged entries stay out of the
    /// change-set so persisters don't re-write rows that didn't
    /// actually change.
    pub async fn sync_balances(
        &self,
        config: Option<AddressSyncConfig>,
    ) -> Result<AddressSyncResult<PlatformAddressTag, PlatformP2PKHAddress>, PlatformWalletError>
    {
        let mut guard = self.provider.write().await;
        let provider = guard.as_mut().ok_or_else(|| {
            PlatformWalletError::AddressSync(
                "Platform address provider not initialized — call initialize() first".into(),
            )
        })?;

        let last_sync_timestamp = provider.last_sync_timestamp();
        provider.prepare_for_sync().await?;

        let before = provider.balances_snapshot();

        let result = self
            .sdk
            .sync_address_balances(&mut *provider, config, last_sync_timestamp)
            .await?;

        // Diff against the post-sync snapshot. Only entries whose
        // funds differ from `before` (or that are new) make it into
        // the change-set.
        let after = provider.balances_snapshot();
        let mut cs = PlatformAddressChangeSet::default();
        for (&(wallet_id, account_index, address), &funds) in &after {
            if before.get(&(wallet_id, account_index, address)) != Some(&funds) {
                cs.addresses.push(PlatformAddressBalanceEntry {
                    wallet_id,
                    account_index,
                    address,
                    funds,
                });
            }
        }
        if result.new_sync_height > 0 {
            cs.sync_height = Some(result.new_sync_height);
        }
        if result.new_sync_timestamp > 0 {
            cs.sync_timestamp = Some(result.new_sync_timestamp);
        }
        if result.last_known_recent_block > 0 {
            cs.last_known_recent_block = Some(result.last_known_recent_block);
        }

        provider.update_sync_state(&result);
        drop(guard);

        if !cs.is_empty() {
            if let Err(e) = self.persister.store(cs.into()) {
                tracing::error!("Failed to persist address sync changeset: {}", e);
            }
        }

        Ok(result)
    }
}
