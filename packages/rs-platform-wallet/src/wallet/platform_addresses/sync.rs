use crate::changeset::Merge;
use crate::wallet::{PlatformAddressTag, PlatformAddressWallet};
use crate::{PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::address_sync::{
    AddressFunds, AddressProvider, AddressSyncConfig, AddressSyncResult,
};
use key_wallet::PlatformP2PKHAddress;
use std::collections::BTreeMap;

impl PlatformAddressWallet {
    /// Sync platform address balances across every platform payment
    /// account on the wallet in a single trunk/branch/compact scan.
    ///
    /// The unified provider presents pending addresses from every
    /// account at once, so the SDK performs one combined sync instead
    /// of N per-account syncs. This is the "BLAST sync" path: fewer
    /// network round trips, one GroveDB proof covering all addresses.
    ///
    /// The change-set is computed as a diff around the SDK call —
    /// snapshot the provider's known balances before, iterate
    /// `result.found` after, emit entries whose funds differ (or are
    /// new). `result.found` carries the full `(tag, p2pkh)` identity
    /// the SDK used, so we don't need a second snapshot.
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

        // Snapshot pre-sync balances keyed by the SDK's `Tag`. That
        // same tag is what `result.found` will be keyed on, so the
        // diff is a direct `before.get(&tag) != Some(&funds)` check.
        let before: BTreeMap<PlatformAddressTag, AddressFunds> =
            AddressProvider::current_balances(&*provider)
                .map(|(tag, _p2pkh, funds)| (tag, funds))
                .collect();

        let result = self
            .sdk
            .sync_address_balances(&mut *provider, config, last_sync_timestamp)
            .await?;

        let mut cs = PlatformAddressChangeSet::default();
        for (&(tag, p2pkh), &funds) in &result.found {
            if before.get(&tag) == Some(&funds) {
                continue;
            }
            let (wallet_id, account_index, address_index) = tag;
            cs.addresses.push(PlatformAddressBalanceEntry {
                wallet_id,
                account_index,
                address_index,
                address: p2pkh,
                funds,
            });
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
