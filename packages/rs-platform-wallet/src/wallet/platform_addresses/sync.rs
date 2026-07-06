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
/// `found` and `absent` are NOT guaranteed disjoint: the SDK's full
/// scan can prove an address absent at the trunk/branch checkpoint and
/// then re-find it during incremental catch-up (a credit op between
/// checkpoint and tip inserts into `found` without removing the earlier
/// absent proof). The catch-up find reflects the chain tip, so it wins —
/// the zero-emission loop skips any address that is also in `found`.
pub(crate) fn compute_address_balance_diff(
    before: &BTreeMap<PlatformAddressTag, AddressFunds>,
    found: &BTreeMap<(PlatformAddressTag, PlatformP2PKHAddress), AddressFunds>,
    absent: &BTreeSet<(PlatformAddressTag, PlatformP2PKHAddress)>,
    absent_as_of_height: u64,
) -> Vec<PlatformAddressBalanceEntry> {
    let mut entries = Vec::new();

    // 1. Found-and-changed.
    for (&(tag, p2pkh), &funds) in found {
        if let Some(existing) = before.get(&tag) {
            // Skip when unchanged, or when the scanned pin is OLDER than
            // the committed pin — a stale-but-valid proof from a lagging
            // node must not clobber a row a fresher reconcile committed
            // (mirrors `commit_reconciliation`'s height-freshness rule,
            // and `sync_finished`'s scratch merge applies the same guard
            // so memory and disk stay in agreement).
            if existing == &funds || existing.as_of_height > funds.as_of_height {
                continue;
            }
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
        // Absent at the checkpoint but re-found during incremental
        // catch-up — the find is newer (tip), so never zero it.
        if found.contains_key(&(tag, p2pkh)) {
            continue;
        }
        // Only emit when we actually had non-default cached funds for the
        // address. An address that was already empty (or never cached)
        // doesn't need a zeroing write.
        let Some(existing) = before.get(&tag) else {
            continue;
        };
        if existing.balance == 0 && existing.nonce == 0 {
            continue;
        }
        // A stale absence proof must not zero a row pinned by a fresher
        // proof or transition reconcile: on a lagging node an address
        // funded seconds ago legitimately does not exist yet at the scan
        // checkpoint, but the committed row is newer truth.
        if existing.as_of_height > absent_as_of_height {
            continue;
        }
        let (wallet_id, account_index, address_index) = tag;
        entries.push(PlatformAddressBalanceEntry {
            wallet_id,
            account_index,
            address_index,
            address: p2pkh,
            // Absent in state ⇒ no balance and no nonce. Reset both. The
            // absence was proven by the tree scan, so the zero is pinned
            // at the scan's checkpoint height (`absent_as_of_height`) —
            // a later re-credit carries a higher pin and wins.
            funds: AddressFunds {
                balance: 0,
                nonce: 0,
                as_of_height: absent_as_of_height,
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

        // Found-and-changed entries plus zeroing entries for addresses
        // proven absent this pass that previously carried cached funds.
        // The latter is what zeroes a stale balance after a chain reset.
        let mut cs = PlatformAddressChangeSet {
            addresses: compute_address_balance_diff(
                &before,
                &result.found,
                &result.absent,
                // Absence is only ever proven by the trunk/branch scan,
                // so its checkpoint height pins the zeroing entries.
                result.checkpoint_height,
            ),
            ..Default::default()
        };
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

        // Persist BEFORE releasing the provider lock. The reconciliation
        // seam (`reconcile_address_infos`) serializes on this lock and
        // persists its proof-attested entries while holding it too, so
        // both writers' stores are totally ordered by the lock: whichever
        // commits its seed later also lands on disk later, and an older
        // diff can never overwrite a fresher row.
        if !cs.is_empty() {
            if let Err(e) = self.persister.store(cs.into()) {
                tracing::error!("Failed to persist address sync changeset: {}", e);
            }
        }
        drop(guard);

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
        AddressFunds {
            balance,
            nonce,
            as_of_height: 0,
        }
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

        let entries = compute_address_balance_diff(&before, &found, &absent, 0);

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

        let entries = compute_address_balance_diff(&before, &found, &absent, 0);
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

        let entries = compute_address_balance_diff(&before, &found, &absent, 0);
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

        let entries = compute_address_balance_diff(&before, &found, &absent, 0);
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

        let entries = compute_address_balance_diff(&before, &found, &absent, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].address_index, 0);
        assert_eq!(entries[0].address, p2pkh(10));
        assert_eq!(entries[0].funds, funds(150, 2));
    }

    /// An address can be proven absent at the full-scan checkpoint and
    /// then re-found during incremental catch-up, landing in BOTH
    /// `found` and `absent`. The find reflects the chain tip, so the
    /// fresh funds must be emitted and no zeroing entry may follow it —
    /// downstream changeset application is order-sensitive and a
    /// trailing zero would clobber the real balance.
    #[test]
    fn address_in_both_found_and_absent_keeps_found_entry() {
        let mut before = BTreeMap::new();
        before.insert(tag(0), funds(100, 1));

        let mut found = BTreeMap::new();
        found.insert((tag(0), p2pkh(1)), funds(250, 2));

        let mut absent = BTreeSet::new();
        absent.insert((tag(0), p2pkh(1)));

        let entries = compute_address_balance_diff(&before, &found, &absent, 0);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].address, p2pkh(1));
        assert_eq!(entries[0].funds, funds(250, 2));
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

        let mut entries = compute_address_balance_diff(&before, &found, &absent, 0);
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

    /// A stale-but-valid scan result (older pin) must not be emitted over
    /// a row a fresher reconcile committed: on a lagging node the scanned
    /// absolute predates the ST-attested balance, and persisting it would
    /// regress the durable row until the next pass self-heals.
    #[test]
    fn found_with_older_pin_is_not_emitted() {
        let mut before = BTreeMap::new();
        // Committed by the reconcile seam at the funding proof height.
        before.insert(
            tag(0),
            AddressFunds {
                balance: 9_985_071_720,
                nonce: 0,
                as_of_height: 379_731,
            },
        );
        let mut found = BTreeMap::new();
        // Scan against a lagging node: pre-funding value at an older pin.
        found.insert(
            (tag(0), p2pkh(0)),
            AddressFunds {
                balance: 0,
                nonce: 0,
                as_of_height: 379_728,
            },
        );
        let entries = compute_address_balance_diff(&before, &found, &BTreeSet::new(), 379_728);
        assert!(
            entries.is_empty(),
            "a stale-pinned scan absolute must not clobber a fresher row"
        );

        // A genuinely newer scan (same or later pin) still emits.
        found.insert(
            (tag(0), p2pkh(0)),
            AddressFunds {
                balance: 5,
                nonce: 1,
                as_of_height: 379_740,
            },
        );
        let entries = compute_address_balance_diff(&before, &found, &BTreeSet::new(), 379_740);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].funds.balance, 5);
    }

    /// A stale absence proof (scan checkpoint below the committed pin)
    /// must not zero a row a fresher reconcile committed — on a lagging
    /// node an address funded seconds ago legitimately does not exist at
    /// the scan checkpoint.
    #[test]
    fn absent_with_older_checkpoint_is_not_emitted() {
        let mut before = BTreeMap::new();
        before.insert(
            tag(0),
            AddressFunds {
                balance: 9_985_071_720,
                nonce: 0,
                as_of_height: 379_731,
            },
        );
        let mut absent = BTreeSet::new();
        absent.insert((tag(0), p2pkh(0)));

        // Lagging checkpoint: no zeroing entry.
        let entries = compute_address_balance_diff(&before, &BTreeMap::new(), &absent, 379_728);
        assert!(
            entries.is_empty(),
            "a stale absence proof must not zero a fresher-pinned row"
        );

        // A checkpoint at/after the pin still zeroes (a real drain).
        let entries = compute_address_balance_diff(&before, &BTreeMap::new(), &absent, 379_731);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].funds.balance, 0);
        assert_eq!(entries[0].funds.as_of_height, 379_731);
    }
}
