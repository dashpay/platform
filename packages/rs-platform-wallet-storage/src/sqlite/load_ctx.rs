//! Load-time policy context — the one place [`LoadPolicy`] is branched on.
//!
//! Every reader that meets a recoverable inconsistency routes it through
//! `LoadCtx::tolerate` (fatal under [`LoadPolicy::Strict`]) or
//! `LoadCtx::note_degraded` (never fatal). No site open-codes the branch,
//! so strictness cannot drift apart between readers. Both are crate-private:
//! the policy decision belongs to the readers, not to callers.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt::{self, Display};

use crate::sqlite::config::LoadPolicy;
use crate::sqlite::error::WalletStorageError;

/// A persisted inconsistency `load()` can meet, one variant per site.
///
/// Used as the key of [`LoadDegradation::by_site`]; [`as_str`](Self::as_str)
/// gives the snake_case tag that appears in logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoadSite {
    /// `core_sync_state.last_applied_chain_lock` failed to decode.
    ChainLockBlob,
    /// A `shielded_viewing_keys` row failed to decode.
    ShieldedViewingKeyRow,
    /// A `core_transactions` row's typed columns disagreed with its blob.
    CoreTransactionColumnDrift,
    /// Rehydration could not derive a resolved index into an address pool.
    RehydrationEnsureDerived,
    /// Rehydration's gap-limit refill failed for an address pool.
    RehydrationGapLimit,
    /// A restored UTXO or used address names an account this wallet lacks.
    /// Counted per address, though one record covers a whole account's.
    OrphanedUtxoOwner,
    /// A restored address did not resolve against its account's xpub.
    /// Counted per address, though one record covers a whole account's.
    UnresolvedUtxoAddress,
    /// One used address resolves to two different owning accounts.
    UsedAddressOwnerConflict,
    /// An `identity_keys` / `contacts` row's owner identity is tombstoned.
    /// Counted per row, though `route_by_owner` decides once per collection
    /// after its walk, so one log line can carry many counts.
    TombstonedIdentityOrphan,
    /// An identity owned by no wallet carries a registration index.
    UnownedIdentityHasRegistrationIndex,
    /// Two live `identities` rows of one wallet claim the same
    /// `identity_index`. Only one can occupy the derivation slot, so the
    /// loser is dropped from the wallet's identity map.
    IdentityIndexCollision,
}

impl LoadSite {
    /// Short snake_case tag for tracing fields and per-site counter keys.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChainLockBlob => "chain_lock_blob",
            Self::ShieldedViewingKeyRow => "shielded_viewing_key_row",
            Self::CoreTransactionColumnDrift => "core_transaction_column_drift",
            Self::RehydrationEnsureDerived => "rehydration_ensure_derived",
            Self::RehydrationGapLimit => "rehydration_gap_limit",
            Self::OrphanedUtxoOwner => "orphaned_utxo_owner",
            Self::UnresolvedUtxoAddress => "unresolved_utxo_address",
            Self::UsedAddressOwnerConflict => "used_address_owner_conflict",
            Self::TombstonedIdentityOrphan => "tombstoned_identity_orphan",
            Self::UnownedIdentityHasRegistrationIndex => "unowned_identity_has_registration_index",
            Self::IdentityIndexCollision => "identity_index_collision",
        }
    }
}

impl Display for LoadSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What one `load()` tolerated instead of returning.
///
/// Snapshot semantics are **per-load**: `load()` replaces the persister's
/// slot, so a database restored from backup and reloaded clean reports
/// clean. Reading the snapshot does not clear it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadDegradation {
    /// `true` iff at least one site was tolerated — equals
    /// `!by_site.is_empty()`. Never set by `unimplemented_rows`.
    pub degraded: bool,
    /// Sum of `by_site`'s values.
    pub total: u32,
    /// Per-site tolerated counts, one per occurrence — a row, an entry, a
    /// blob — never one per decision the reader took. Absent sites had
    /// nothing to tolerate.
    pub by_site: BTreeMap<LoadSite, u32>,
    /// Rows present in tables `load()` has no reader for. Informational:
    /// the data is intact, merely not rehydrated, so it never sets
    /// `degraded`.
    pub unimplemented_rows: u32,
}

impl LoadDegradation {
    /// Fold another read's tally into this one.
    ///
    /// The single owner of the two field invariants — `total` is the sum of
    /// `by_site`, `degraded` is `!by_site.is_empty()` — so no caller
    /// re-derives them and drifts.
    pub(crate) fn merge(&mut self, other: Self) {
        let mut by_site = std::mem::take(&mut self.by_site);
        for (site, count) in other.by_site {
            let slot = by_site.entry(site).or_insert(0);
            *slot = slot.saturating_add(count);
        }
        let unimplemented_rows = self
            .unimplemented_rows
            .saturating_add(other.unimplemented_rows);
        *self = Self::from_counts(by_site, unimplemented_rows);
    }

    /// Derive the invariant fields from the raw counters.
    fn from_counts(by_site: BTreeMap<LoadSite, u32>, unimplemented_rows: u32) -> Self {
        Self {
            degraded: !by_site.is_empty(),
            total: by_site.values().copied().fold(0u32, u32::saturating_add),
            by_site,
            unimplemented_rows,
        }
    }
}

/// Where a degraded site fired, as structured log fields.
///
/// `account_type` is `dyn Debug` so this stays free of the wallet types;
/// `affected` is how many rows, addresses or entries the incident covers.
pub(crate) struct SiteCoords<'a> {
    pub wallet_id: [u8; 32],
    pub account_type: &'a dyn fmt::Debug,
    pub affected: usize,
}

/// Per-`load()` policy + counters, created on the loading thread's stack.
///
/// Not stored on the persister (which keeps only the resulting
/// [`LoadDegradation`]), so the interior mutability here never crosses a
/// thread boundary.
#[derive(Debug)]
pub struct LoadCtx {
    policy: LoadPolicy,
    counts: RefCell<BTreeMap<LoadSite, u32>>,
    unimplemented_rows: Cell<u32>,
}

impl LoadCtx {
    /// Context for `policy`.
    pub fn new(policy: LoadPolicy) -> Self {
        Self {
            policy,
            counts: RefCell::new(BTreeMap::new()),
            unimplemented_rows: Cell::new(0),
        }
    }

    /// Context that aborts the load on any inconsistency. A production
    /// load takes its policy from the config, so this exists for the
    /// builds where the module is public.
    #[cfg(any(test, feature = "__test-helpers", feature = "rehydration-apply"))]
    pub fn strict() -> Self {
        Self::new(LoadPolicy::Strict)
    }

    /// Context that tolerates, logs, and counts recoverable
    /// inconsistencies. Public on the same terms as
    /// [`strict`](Self::strict).
    #[cfg(any(test, feature = "__test-helpers", feature = "rehydration-apply"))]
    pub fn recovery() -> Self {
        Self::new(LoadPolicy::Recovery)
    }

    /// Fatal-or-tolerated dispatch for a recoverable inconsistency.
    ///
    /// Returns `Err(err)` under [`LoadPolicy::Strict`]. Under
    /// [`LoadPolicy::Recovery`] it warns, counts `site`, and returns
    /// `Ok(())` so the caller continues with its documented degraded
    /// projection.
    pub(crate) fn tolerate(
        &self,
        site: LoadSite,
        err: WalletStorageError,
    ) -> Result<(), WalletStorageError> {
        self.tolerate_many(site, 1, err)
    }

    /// [`tolerate`](Self::tolerate) for `occurrences` incidents at once.
    ///
    /// For the walks that count as they go and decide afterwards, so one
    /// log record covers a whole collection while `by_site` still counts
    /// occurrences like every other site.
    pub(crate) fn tolerate_many(
        &self,
        site: LoadSite,
        occurrences: u32,
        err: WalletStorageError,
    ) -> Result<(), WalletStorageError> {
        if self.policy == LoadPolicy::Strict {
            return Err(err);
        }
        self.count(site, occurrences);
        tracing::warn!(
            site = site.as_str(),
            occurrences,
            error_kind = err.error_kind_str(),
            error = %err,
            "recovery mode: tolerating a persisted inconsistency instead of failing the load"
        );
        Ok(())
    }

    /// Record an inconsistency that is never fatal, in either policy.
    ///
    /// For sites whose signal cannot distinguish corruption from a healthy
    /// wallet, so failing the load would brick legitimate wallets.
    /// `coords` and `cause` land as fields of one record, so nothing has to
    /// be joined against a neighbouring line to know where it happened, and
    /// `coords.affected` is what the site counts — one incident covering
    /// nine hundred addresses is nine hundred, like every other site.
    pub(crate) fn note_degraded(&self, site: LoadSite, coords: SiteCoords<'_>, cause: &str) {
        // Floored at one: a caller that reports nothing affected still met
        // an inconsistency, and a zero would leave a site keyed with no
        // count behind it.
        let occurrences = u32::try_from(coords.affected).unwrap_or(u32::MAX).max(1);
        self.count(site, occurrences);
        tracing::warn!(
            site = site.as_str(),
            wallet_id = %hex::encode(coords.wallet_id),
            account_type = ?coords.account_type,
            affected = coords.affected,
            cause,
            "load degraded: an ambiguous persisted inconsistency was accepted as-is"
        );
    }

    /// Add rows found in a table `load()` cannot rehydrate. Informational —
    /// does not mark the load degraded.
    pub(crate) fn add_unimplemented_rows(&self, rows: u32) {
        self.unimplemented_rows
            .set(self.unimplemented_rows.get().saturating_add(rows));
    }

    /// Snapshot the counters accumulated so far.
    pub fn degradation(&self) -> LoadDegradation {
        LoadDegradation::from_counts(self.counts.borrow().clone(), self.unimplemented_rows.get())
    }

    fn count(&self, site: LoadSite, occurrences: u32) {
        let mut counts = self.counts.borrow_mut();
        let slot = counts.entry(site).or_insert(0);
        *slot = slot.saturating_add(occurrences);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_returns_the_error_and_counts_nothing() {
        let ctx = LoadCtx::strict();
        let err = ctx
            .tolerate(
                LoadSite::ChainLockBlob,
                WalletStorageError::blob_decode("test"),
            )
            .expect_err("strict must propagate");
        assert!(matches!(err, WalletStorageError::BlobDecode { .. }));
        assert_eq!(ctx.degradation(), LoadDegradation::default());
    }

    #[test]
    fn recovery_counts_per_site_and_sets_degraded() {
        let ctx = LoadCtx::recovery();
        ctx.tolerate(
            LoadSite::ChainLockBlob,
            WalletStorageError::blob_decode("one"),
        )
        .expect("recovery must tolerate");
        ctx.tolerate(
            LoadSite::ChainLockBlob,
            WalletStorageError::blob_decode("two"),
        )
        .expect("recovery must tolerate");
        let snapshot = ctx.degradation();
        assert!(snapshot.degraded);
        assert_eq!(snapshot.total, 2);
        assert_eq!(snapshot.by_site.get(&LoadSite::ChainLockBlob), Some(&2));
    }

    #[test]
    fn tolerate_many_counts_every_occurrence_from_one_record() {
        let ctx = LoadCtx::recovery();
        ctx.tolerate_many(
            LoadSite::TombstonedIdentityOrphan,
            5,
            WalletStorageError::blob_decode("five leftover rows"),
        )
        .expect("recovery must tolerate");
        let snapshot = ctx.degradation();
        assert_eq!(snapshot.total, 5);
        assert_eq!(
            snapshot.by_site.get(&LoadSite::TombstonedIdentityOrphan),
            Some(&5)
        );
    }

    #[test]
    fn note_degraded_counts_every_affected_row() {
        let ctx = LoadCtx::strict();
        ctx.note_degraded(
            LoadSite::UnresolvedUtxoAddress,
            SiteCoords {
                wallet_id: [7u8; 32],
                account_type: &"Standard[0]",
                affected: 900,
            },
            "nine hundred addresses did not resolve",
        );
        assert_eq!(
            ctx.degradation()
                .by_site
                .get(&LoadSite::UnresolvedUtxoAddress),
            Some(&900)
        );
    }

    #[test]
    fn note_degraded_counts_under_strict_too() {
        let ctx = LoadCtx::strict();
        ctx.note_degraded(
            LoadSite::OrphanedUtxoOwner,
            SiteCoords {
                wallet_id: [7u8; 32],
                account_type: &"Standard[0]",
                affected: 1,
            },
            "ambiguous owner",
        );
        let snapshot = ctx.degradation();
        assert!(snapshot.degraded);
        assert_eq!(snapshot.by_site.get(&LoadSite::OrphanedUtxoOwner), Some(&1));
    }

    #[test]
    fn merge_re_derives_the_invariants_from_the_folded_counters() {
        let first = LoadCtx::recovery();
        first
            .tolerate(
                LoadSite::ChainLockBlob,
                WalletStorageError::blob_decode("one"),
            )
            .expect("recovery must tolerate");
        first.add_unimplemented_rows(3);
        let second = LoadCtx::recovery();
        second
            .tolerate(
                LoadSite::ChainLockBlob,
                WalletStorageError::blob_decode("two"),
            )
            .expect("recovery must tolerate");
        second
            .tolerate(
                LoadSite::UnownedIdentityHasRegistrationIndex,
                WalletStorageError::blob_decode("three"),
            )
            .expect("recovery must tolerate");
        second.add_unimplemented_rows(4);

        let mut merged = first.degradation();
        merged.merge(second.degradation());

        assert!(merged.degraded);
        assert_eq!(merged.total, 3, "total is the sum of by_site");
        assert_eq!(merged.by_site.get(&LoadSite::ChainLockBlob), Some(&2));
        assert_eq!(merged.unimplemented_rows, 7);
    }

    #[test]
    fn merging_only_unimplemented_rows_keeps_the_snapshot_clean() {
        let rows_only = LoadCtx::strict();
        rows_only.add_unimplemented_rows(9);

        let mut merged = LoadDegradation::default();
        merged.merge(rows_only.degradation());

        assert!(!merged.degraded);
        assert_eq!(merged.total, 0);
        assert_eq!(merged.unimplemented_rows, 9);
    }

    #[test]
    fn unimplemented_rows_do_not_set_degraded() {
        let ctx = LoadCtx::strict();
        ctx.add_unimplemented_rows(7);
        let snapshot = ctx.degradation();
        assert!(!snapshot.degraded);
        assert_eq!(snapshot.total, 0);
        assert_eq!(snapshot.unimplemented_rows, 7);
    }
}
