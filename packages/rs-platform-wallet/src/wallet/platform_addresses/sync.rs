use crate::changeset::Merge;
use crate::wallet::{PlatformAddressTag, PlatformAddressWallet};
use crate::{PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::address_sync::{
    AddressFunds, AddressProvider, AddressSyncConfig, AddressSyncResult,
};
use key_wallet::PlatformP2PKHAddress;
use std::collections::{BTreeMap, BTreeSet};

/// Compute the `PlatformAddressBalanceEntry` diff for one sync pass.
///
/// Pure function over the three inputs that drive a BLAST diff so it can
/// be unit-tested without an `Sdk` or network:
///
/// * `before` — the provider's known balances keyed by the SDK `Tag`,
///   snapshotted *before* the sync call.
/// * `found` — `result.found`: addresses proven present this pass, with
///   their fresh funds.
/// * `absent` — `result.absent`: addresses proven absent this pass.
///
/// Two kinds of entries are emitted:
///
/// 1. **Found-and-changed.** For each `(tag, p2pkh)` in `found` whose
///    funds differ from the pre-sync snapshot, emit an entry carrying the
///    new funds. Unchanged found entries are skipped (no-op persist).
/// 2. **Absent-but-previously-funded.** For each `(tag, p2pkh)` in
///    `absent` that the snapshot tracked with non-default funds (balance
///    or nonce non-zero), emit a **zeroed** entry (balance 0, nonce 0).
///    A proven-absent address has no balance and no nonce in state, so
///    resetting both is correct — without this the persister never learns
///    the cached balance went away after a chain reset, and the stale
///    value lingers as an identity-creation funding source.
///
/// `found` and `absent` are disjoint per pass (the SDK proves an address
/// either present or absent, never both), so the two loops can't both
/// emit for the same address.
pub(crate) fn compute_address_balance_diff(
    before: &BTreeMap<PlatformAddressTag, AddressFunds>,
    found: &BTreeMap<(PlatformAddressTag, PlatformP2PKHAddress), AddressFunds>,
    absent: &BTreeSet<(PlatformAddressTag, PlatformP2PKHAddress)>,
) -> Vec<PlatformAddressBalanceEntry> {
    let mut entries = Vec::new();

    // 1. Found-and-changed.
    for (&(tag, p2pkh), &funds) in found {
        if before.get(&tag) == Some(&funds) {
            continue;
        }
        let (wallet_id, account_index, address_index) = tag;
        entries.push(PlatformAddressBalanceEntry {
            wallet_id,
            account_index,
            address_index,
            address: p2pkh,
            funds,
        });
    }

    // 2. Absent-but-previously-funded — zero out cached balances that the
    //    new chain no longer knows about.
    for &(tag, p2pkh) in absent {
        // Only emit when we actually had non-default cached funds for the
        // address. An address that was already empty (or never cached)
        // doesn't need a zeroing write.
        let had_funds = before
            .get(&tag)
            .is_some_and(|f| f.balance != 0 || f.nonce != 0);
        if !had_funds {
            continue;
        }
        let (wallet_id, account_index, address_index) = tag;
        entries.push(PlatformAddressBalanceEntry {
            wallet_id,
            account_index,
            address_index,
            address: p2pkh,
            // Absent in state ⇒ no balance and no nonce. Reset both.
            funds: AddressFunds {
                balance: 0,
                nonce: 0,
            },
        });
    }

    entries
}

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
        // Found-and-changed entries plus zeroing entries for addresses
        // proven absent this pass that previously carried cached funds.
        // The latter is what zeroes a stale balance after a chain reset.
        cs.addresses = compute_address_balance_diff(&before, &result.found, &result.absent);
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

#[cfg(test)]
mod tests {
    use super::*;

    const WALLET: crate::wallet::platform_wallet::WalletId = [7u8; 32];

    fn tag(address_index: u32) -> PlatformAddressTag {
        (WALLET, 0, address_index)
    }

    fn p2pkh(byte: u8) -> PlatformP2PKHAddress {
        PlatformP2PKHAddress::new([byte; 20])
    }

    fn funds(balance: u64, nonce: u32) -> AddressFunds {
        AddressFunds { balance, nonce }
    }

    /// An address that previously carried a cached balance and is proven
    /// absent this pass must produce a single zeroed entry (balance 0,
    /// nonce 0). This is the chain-reset bug: without it the stale
    /// balance is never written back to 0.
    #[test]
    fn absent_previously_funded_address_emits_zero_entry() {
        let mut before = BTreeMap::new();
        before.insert(tag(0), funds(294_627_247_940, 5));

        let found = BTreeMap::new();
        let mut absent = BTreeSet::new();
        absent.insert((tag(0), p2pkh(1)));

        let entries = compute_address_balance_diff(&before, &found, &absent);

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.wallet_id, WALLET);
        assert_eq!(entry.account_index, 0);
        assert_eq!(entry.address_index, 0);
        assert_eq!(entry.address, p2pkh(1));
        assert_eq!(entry.funds.balance, 0);
        assert_eq!(entry.funds.nonce, 0);
    }

    /// An address proven absent that we never had cached funds for
    /// produces no entry — there's nothing to zero, and emitting a
    /// spurious zero would churn the persister.
    #[test]
    fn absent_never_funded_address_emits_nothing() {
        let before = BTreeMap::new();
        let found = BTreeMap::new();
        let mut absent = BTreeSet::new();
        absent.insert((tag(0), p2pkh(1)));

        let entries = compute_address_balance_diff(&before, &found, &absent);
        assert!(entries.is_empty());
    }

    /// An address whose cached funds were already zero balance but still
    /// carries a non-zero nonce is treated as "had funds" and gets a
    /// zeroing entry — the absent address has no nonce in state either,
    /// so the nonce must be reset too.
    #[test]
    fn absent_zero_balance_nonzero_nonce_emits_zero_entry() {
        let mut before = BTreeMap::new();
        before.insert(tag(0), funds(0, 9));

        let found = BTreeMap::new();
        let mut absent = BTreeSet::new();
        absent.insert((tag(0), p2pkh(1)));

        let entries = compute_address_balance_diff(&before, &found, &absent);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].funds.balance, 0);
        assert_eq!(entries[0].funds.nonce, 0);
    }

    /// A genuinely empty cached entry (balance 0, nonce 0) that goes
    /// absent emits nothing — it's already in the zeroed state.
    #[test]
    fn absent_default_funds_emits_nothing() {
        let mut before = BTreeMap::new();
        before.insert(tag(0), funds(0, 0));

        let found = BTreeMap::new();
        let mut absent = BTreeSet::new();
        absent.insert((tag(0), p2pkh(1)));

        let entries = compute_address_balance_diff(&before, &found, &absent);
        assert!(entries.is_empty());
    }

    /// Found-and-changed entries still flow through unchanged, and a
    /// found entry whose funds match the snapshot is skipped.
    #[test]
    fn found_changed_emitted_unchanged_skipped() {
        let mut before = BTreeMap::new();
        before.insert(tag(0), funds(100, 1)); // will change
        before.insert(tag(1), funds(200, 2)); // unchanged

        let mut found = BTreeMap::new();
        found.insert((tag(0), p2pkh(10)), funds(150, 2));
        found.insert((tag(1), p2pkh(20)), funds(200, 2));

        let absent = BTreeSet::new();

        let entries = compute_address_balance_diff(&before, &found, &absent);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].address_index, 0);
        assert_eq!(entries[0].address, p2pkh(10));
        assert_eq!(entries[0].funds, funds(150, 2));
    }

    /// Combined pass: one address found with a new balance and a
    /// different address proven absent after being funded. Both kinds of
    /// entries appear together, disjoint addresses.
    #[test]
    fn found_and_absent_combine() {
        let mut before = BTreeMap::new();
        before.insert(tag(0), funds(50, 1)); // found, changes
        before.insert(tag(1), funds(999, 3)); // absent, was funded

        let mut found = BTreeMap::new();
        found.insert((tag(0), p2pkh(10)), funds(75, 2));

        let mut absent = BTreeSet::new();
        absent.insert((tag(1), p2pkh(20)));

        let mut entries = compute_address_balance_diff(&before, &found, &absent);
        entries.sort_by_key(|e| e.address_index);

        assert_eq!(entries.len(), 2);

        // Found-and-changed entry.
        assert_eq!(entries[0].address_index, 0);
        assert_eq!(entries[0].funds, funds(75, 2));

        // Absent-zeroed entry.
        assert_eq!(entries[1].address_index, 1);
        assert_eq!(entries[1].address, p2pkh(20));
        assert_eq!(entries[1].funds, funds(0, 0));
    }
}
