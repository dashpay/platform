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
/// (and this type's `Display`) give the snake_case tag that appears in
/// logs. [`explanation`](Self::explanation) gives the human-readable prose
/// for the same site — the two are deliberately separate: a UI layer wants
/// the prose, a log consumer wants the tag, and neither should have to
/// derive one from the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoadSite {
    /// `core_sync_state.last_applied_chain_lock` failed to decode.
    ChainLockBlob,
    /// A `shielded_viewing_keys` row failed to decode.
    ShieldedViewingKeyRow,
    /// A `core_transactions` row's typed columns disagreed with its blob.
    CoreTransactionColumnDrift,
    /// An ECDSA account-registration row's typed columns disagreed with its blob.
    AccountRegistrationDrift,
    /// A provider account-registration row's typed columns disagreed with its blob.
    ProviderKeyRegistrationDrift,
    /// A provider account-registration row carries the wrong key curve.
    ProviderKeyCurveMismatch,
    /// An `asset_locks` row's typed status disagreed with its lifecycle blob.
    AssetLockStatusDrift,
    /// Rehydration could not derive a resolved index into an address pool.
    RehydrationEnsureDerived,
    /// Rehydration rejected an address pool's oversized gap-limit refill.
    RehydrationGapLimit,
    /// Rehydration could not maintain an address pool's gap limit.
    RehydrationMaintainGapLimit,
    /// A restored UTXO or used address names an account this wallet lacks.
    /// Counted per address, though one record covers a whole account's.
    OrphanedUtxoOwner,
    /// A restored address did not resolve against its account's xpub.
    /// Counted per address, though one record covers a whole account's.
    UnresolvedUtxoAddress,
    /// A stored UTXO or pool script could not be decoded as an address.
    UndecodableAddressScript,
    /// One used address resolves to two different owning accounts.
    UsedAddressOwnerConflict,
    /// An `identity_keys` / `contacts` row's owner identity is tombstoned.
    /// Counted per row, though `route_by_owner` decides once per collection
    /// after its walk, so one log line can carry many counts.
    TombstonedIdentityOrphan,
    /// An identity owned by no wallet carries a registration index.
    UnownedIdentityHasRegistrationIndex,
    /// Two live `identities` rows of one wallet claim the same
    /// `identity_index`. Only one can occupy the derivation slot; the loser
    /// is moved to `out_of_wallet_identities` rather than dropped, since
    /// nothing persisted establishes which row truly owns the slot and a
    /// Recovery load is read-only — anything discarded here could never be
    /// re-persisted.
    IdentityIndexCollision,
    /// An `identity_scan_states` row claims a complete scan while unanswered
    /// indices sit beside it. Clamped toward incomplete, which costs one
    /// extra scan instead of an identity that never reappears.
    IdentityScanStateContradiction,
}

impl LoadSite {
    /// Short snake_case tag for tracing fields and per-site counter keys.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChainLockBlob => "chain_lock_blob",
            Self::ShieldedViewingKeyRow => "shielded_viewing_key_row",
            Self::CoreTransactionColumnDrift => "core_transaction_column_drift",
            Self::AccountRegistrationDrift => "account_registration_drift",
            Self::ProviderKeyRegistrationDrift => "provider_key_registration_drift",
            Self::ProviderKeyCurveMismatch => "provider_key_curve_mismatch",
            Self::AssetLockStatusDrift => "asset_lock_status_drift",
            Self::RehydrationEnsureDerived => "rehydration_ensure_derived",
            Self::RehydrationGapLimit => "rehydration_gap_limit",
            Self::RehydrationMaintainGapLimit => "rehydration_maintain_gap_limit",
            Self::OrphanedUtxoOwner => "orphaned_utxo_owner",
            Self::UnresolvedUtxoAddress => "unresolved_utxo_address",
            Self::UndecodableAddressScript => "undecodable_address_script",
            Self::UsedAddressOwnerConflict => "used_address_owner_conflict",
            Self::TombstonedIdentityOrphan => "tombstoned_identity_orphan",
            Self::UnownedIdentityHasRegistrationIndex => "unowned_identity_has_registration_index",
            Self::IdentityIndexCollision => "identity_index_collision",
            Self::IdentityScanStateContradiction => "identity_scan_state_contradiction",
        }
    }

    /// Human-readable prose for this site — public so a host application
    /// can render *what* degraded without re-deriving eighteen strings
    /// this crate already holds, or falling back to showing the user
    /// [`as_str`](Self::as_str)'s log tag. This is the text every
    /// `tracing::warn!` emitted for `self` also carries as its `message`
    /// field, so a log reader and an API caller see the same wording.
    ///
    /// One entry per site, no `_` catch-all: adding a `LoadSite` must fail
    /// to compile here and force a decision about its explanatory text,
    /// instead of silently inheriting wording that describes it wrongly.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::ShieldedViewingKeyRow => {
                "recovery mode: skipping an unreadable shielded viewing-key row"
            }
            Self::RehydrationEnsureDerived => {
                "recovery mode: leaving an address pool short after derivation failed"
            }
            Self::RehydrationGapLimit => {
                "recovery mode: refusing an oversized address-pool gap refill"
            }
            Self::RehydrationMaintainGapLimit => {
                "recovery mode: leaving an address pool short after gap maintenance failed"
            }
            Self::TombstonedIdentityOrphan => {
                "recovery mode: skipping rows owned by a tombstoned identity"
            }
            // The only two sites `note_degraded` ever reaches (see its
            // callers) — never-fatal in either policy, so their prose
            // describes an accepted-as-is degradation rather than a
            // tolerated inconsistency.
            Self::OrphanedUtxoOwner => {
                "load degraded: routing addresses from an unavailable owner to the first funds account"
            }
            Self::UnresolvedUtxoAddress => {
                "load degraded: deferring addresses that did not resolve against the account xpub"
            }
            // Sites whose site tag plus the logged error already say
            // everything a reader needs, so they share the generic line.
            Self::ChainLockBlob
            | Self::CoreTransactionColumnDrift
            | Self::AccountRegistrationDrift
            | Self::ProviderKeyRegistrationDrift
            | Self::ProviderKeyCurveMismatch
            | Self::AssetLockStatusDrift
            | Self::UndecodableAddressScript
            | Self::UsedAddressOwnerConflict
            | Self::UnownedIdentityHasRegistrationIndex
            | Self::IdentityIndexCollision
            | Self::IdentityScanStateContradiction => {
                "recovery mode: tolerating a persisted inconsistency instead of failing the load"
            }
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

impl Display for LoadDegradation {
    /// A short, human-readable summary: one line naming the total and site
    /// count when clean or degraded, then one line per site pairing its
    /// log tag with [`LoadSite::explanation`] and its tolerated count.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.degraded {
            return write!(f, "load not degraded");
        }
        write!(
            f,
            "load degraded: {} inconsistenc{} tolerated across {} site{}",
            self.total,
            if self.total == 1 { "y" } else { "ies" },
            self.by_site.len(),
            if self.by_site.len() == 1 { "" } else { "s" },
        )?;
        for (site, count) in &self.by_site {
            write!(f, "\n  - {site} (x{count}): {}", site.explanation())?;
        }
        Ok(())
    }
}

/// Where a degraded site fired, as structured log fields.
///
/// `account_type` and optional `detail` are `dyn Debug` so this stays free of
/// wallet types; `affected` is how many rows, addresses or entries it covers.
pub(crate) struct SiteCoords<'a> {
    pub wallet_id: Option<[u8; 32]>,
    pub account_type: &'a dyn fmt::Debug,
    pub affected: usize,
    pub detail: Option<&'a dyn fmt::Debug>,
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
    /// load takes its policy from the config; this is the shorthand for a
    /// caller driving a reader directly.
    pub fn strict() -> Self {
        Self::new(LoadPolicy::Strict)
    }

    /// Context that tolerates, logs, and counts recoverable
    /// inconsistencies. The counterpart to [`strict`](Self::strict).
    pub fn recovery() -> Self {
        Self::new(LoadPolicy::Recovery)
    }

    /// Fatal-or-tolerated dispatch for a recoverable inconsistency.
    ///
    /// Returns `Err(err)` under [`LoadPolicy::Strict`]. Under
    /// [`LoadPolicy::Recovery`] it warns, counts `site` once, and returns
    /// `Ok(())` so the caller continues with its documented degraded
    /// projection. For a walk that counts several occurrences under one
    /// log record, use [`tolerate_at`](Self::tolerate_at) with
    /// `coords.affected` set to the count.
    pub(crate) fn tolerate(
        &self,
        site: LoadSite,
        err: WalletStorageError,
    ) -> Result<(), WalletStorageError> {
        if self.policy == LoadPolicy::Strict {
            return Err(err);
        }
        self.count(site, 1);
        tracing::warn!(
            site = site.as_str(),
            error_kind = err.error_kind_str(),
            error = %err,
            message = site.explanation(),
        );
        Ok(())
    }

    /// [`tolerate`](Self::tolerate) with coordinates in the recovery log.
    ///
    /// Strict returns `err`; Recovery counts the incident and logs its error
    /// kind and location. Unlike [`note_degraded`](Self::note_degraded), this
    /// is never used for incidents accepted under Strict.
    pub(crate) fn tolerate_at(
        &self,
        site: LoadSite,
        coords: SiteCoords<'_>,
        err: WalletStorageError,
    ) -> Result<(), WalletStorageError> {
        if self.policy == LoadPolicy::Strict {
            return Err(err);
        }
        let occurrences = u32::try_from(coords.affected).unwrap_or(u32::MAX).max(1);
        self.count(site, occurrences);
        tracing::warn!(
            site = site.as_str(),
            wallet_id = ?coords.wallet_id.map(hex::encode),
            account_type = ?coords.account_type,
            affected = coords.affected,
            detail = ?coords.detail,
            error_kind = err.error_kind_str(),
            error = %err,
            message = site.explanation(),
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
            wallet_id = ?coords.wallet_id.map(hex::encode),
            account_type = ?coords.account_type,
            affected = coords.affected,
            detail = ?coords.detail,
            cause,
            message = site.explanation(),
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

    /// RUST-003: `tolerate_many` (public-in-name-only, its only non-test
    /// caller passed a constant 1) is gone — `tolerate_at` is the one path
    /// for a walk that counts several occurrences under one log record.
    #[test]
    fn tolerate_at_counts_every_occurrence_from_one_record() {
        let ctx = LoadCtx::recovery();
        ctx.tolerate_at(
            LoadSite::TombstonedIdentityOrphan,
            SiteCoords {
                wallet_id: Some([9u8; 32]),
                account_type: &"n/a",
                affected: 5,
                detail: None,
            },
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

    /// RUST-003: `tolerate` must log the site's bespoke
    /// [`LoadSite::explanation`], not the old hard-coded generic literal —
    /// `TombstonedIdentityOrphan` has bespoke prose that the previous
    /// literal could never surface.
    #[tracing_test::traced_test]
    #[test]
    fn tolerate_logs_the_site_explanation_as_the_message_field() {
        let ctx = LoadCtx::recovery();
        ctx.tolerate(
            LoadSite::TombstonedIdentityOrphan,
            WalletStorageError::blob_decode("orphaned row"),
        )
        .expect("recovery must tolerate");
        assert!(logs_contain(
            "recovery mode: skipping rows owned by a tombstoned identity"
        ));
    }

    /// RUST-003: `tolerate` and `tolerate_at` must both log via
    /// `site.explanation()` — the same per-site text, computed the same
    /// way — instead of `tolerate`'s old hard-coded literal that was
    /// identical for every site regardless of which one fired.
    #[tracing_test::traced_test]
    #[test]
    fn tolerate_and_tolerate_at_both_log_the_site_explanation() {
        let ctx = LoadCtx::recovery();
        ctx.tolerate(
            LoadSite::ChainLockBlob,
            WalletStorageError::blob_decode("test"),
        )
        .expect("recovery must tolerate");
        ctx.tolerate_at(
            LoadSite::ShieldedViewingKeyRow,
            SiteCoords {
                wallet_id: None,
                account_type: &"n/a",
                affected: 1,
                detail: None,
            },
            WalletStorageError::blob_decode("test"),
        )
        .expect("recovery must tolerate");
        assert!(logs_contain(LoadSite::ChainLockBlob.explanation()));
        assert!(logs_contain(LoadSite::ShieldedViewingKeyRow.explanation()));
    }

    #[test]
    fn note_degraded_counts_every_affected_row() {
        let ctx = LoadCtx::strict();
        ctx.note_degraded(
            LoadSite::UnresolvedUtxoAddress,
            SiteCoords {
                wallet_id: Some([7u8; 32]),
                account_type: &"Standard[0]",
                affected: 900,
                detail: None,
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
                wallet_id: Some([7u8; 32]),
                account_type: &"Standard[0]",
                affected: 1,
                detail: None,
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

    /// RUST-002/RUST-003: `note_degraded` must log the same `message`
    /// field shape as `tolerate`/`tolerate_at`, carrying the site's
    /// bespoke [`LoadSite::explanation`].
    #[tracing_test::traced_test]
    #[test]
    fn note_degraded_logs_the_site_explanation_as_the_message_field() {
        let ctx = LoadCtx::strict();
        ctx.note_degraded(
            LoadSite::OrphanedUtxoOwner,
            SiteCoords {
                wallet_id: Some([7u8; 32]),
                account_type: &"Standard[0]",
                affected: 1,
                detail: None,
            },
            "ambiguous owner",
        );
        assert!(logs_contain(
            "load degraded: routing addresses from an unavailable owner to the first funds account"
        ));
    }

    /// RUST-002: `Display`/`as_str` stay the snake_case log tag —
    /// `explanation` is the separate, human-readable rendering. A caller
    /// must not get jargon from one and prose from the other by accident.
    #[test]
    fn display_is_the_tag_and_explanation_is_the_prose() {
        assert_eq!(
            LoadSite::IdentityIndexCollision.to_string(),
            "identity_index_collision"
        );
        assert_eq!(
            LoadSite::IdentityIndexCollision.as_str(),
            LoadSite::IdentityIndexCollision.to_string()
        );
        assert_ne!(
            LoadSite::IdentityIndexCollision.explanation(),
            LoadSite::IdentityIndexCollision.as_str()
        );
        assert!(!LoadSite::IdentityIndexCollision
            .explanation()
            .contains("identity_index_collision"));
    }

    /// RUST-002: `LoadDegradation`'s `Display` is the public rendering a
    /// host app reaches for instead of re-deriving prose from `by_site`'s
    /// tags — it must name the site's log tag, its count, and its prose.
    #[test]
    fn load_degradation_display_summarizes_by_site() {
        assert_eq!(LoadDegradation::default().to_string(), "load not degraded");

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
        let rendered = ctx.degradation().to_string();
        assert!(rendered.starts_with("load degraded: 2 inconsistencies tolerated across 1 site"));
        assert!(rendered.contains("chain_lock_blob"));
        assert!(rendered.contains("(x2)"));
        assert!(rendered.contains(LoadSite::ChainLockBlob.explanation()));
    }
}
