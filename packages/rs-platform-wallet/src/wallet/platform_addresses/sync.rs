use crate::changeset::Merge;
use crate::wallet::{PlatformAddressTag, PlatformAddressWallet};
use crate::{PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::address_sync::{AddressSyncConfig, AddressSyncResult};
use dpp::address_funds::PlatformAddress;
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
    /// Persists the resulting changeset internally via the wallet
    /// persister. Pass `None` for `config` to use SDK defaults.
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

        let result = self
            .sdk
            .sync_address_balances(&mut *provider, config, last_sync_timestamp)
            .await?;

        // Build the changeset from every address found during this
        // pass. Balances have already been written into the wallet's
        // ManagedPlatformAccounts by `on_address_found_in_pool`; the
        // changeset carries the full AddressFunds snapshot so
        // persisters can record balance + nonce in one write.
        let mut cs = PlatformAddressChangeSet::default();
        for (_account_index, _index, p2pkh, funds) in
            provider.found_iter_for_wallet(&self.wallet_id)
        {
            cs.addresses
                .insert(PlatformAddress::P2pkh(p2pkh.to_bytes()), *funds);
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
