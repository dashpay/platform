//! Versioned, feature-specific persistence capability contract.
//!
//! Persistence implementations attest only the features they can actually
//! round-trip. Callers compose the bits required by an operation and fail
//! closed when any bit is absent; a generic "durable" boolean cannot express
//! that an otherwise transactional backend is missing one feature callback.

/// Version of the bit assignments in [`PersistenceCapabilities`].
///
/// Bit meanings are append-only within a version. Existing values must never
/// be renumbered or reused, including after a capability is retired.
pub const PERSISTENCE_CAPABILITIES_VERSION: u32 = 1;

/// Feature-specific persistence capabilities.
///
/// This is a transparent `u64` wrapper rather than a Rust enum so unknown bits
/// survive additive upgrades. Bits describe persistence contracts, not product
/// feature flags: hosts must not set a bit merely because a schema/table exists
/// when the corresponding contract's required paths are not all reachable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PersistenceCapabilities(u64);

impl PersistenceCapabilities {
    pub const NONE: Self = Self(0);

    /// A changeset is committed or rolled back as one unit.
    pub const ATOMIC_CHANGESETS: Self = Self(1 << 0);
    /// Invitation records can be persisted.
    pub const INVITATIONS: Self = Self(1 << 1);
    /// Asset-lock funding indices and their owning account/address-pool state
    /// can be persisted. Restart hydration is the separate `WALLET_RESTORE`
    /// contract and operation composites require it where needed.
    pub const ASSET_LOCK_FUNDING_INDICES: Self = Self(1 << 2);
    /// Source-compatible alias for the original capability name.
    pub const ACCOUNT_ADDRESS_POOLS: Self = Self::ASSET_LOCK_FUNDING_INDICES;
    /// Orchard FVKs have paired persist, load, and load-allocation-free paths.
    pub const SHIELDED_VIEWING_KEYS: Self = Self(1 << 3);
    /// Provider special transactions have persist and wallet-restore paths.
    pub const PROVIDER_TRANSACTIONS: Self = Self(1 << 4);
    /// Token balances are stored through a lossless unsigned-`u64` contract.
    pub const UNSIGNED_TOKEN_STORAGE: Self = Self(1 << 5);
    /// Pending contact-crypto queue additions and removals survive restart.
    pub const PENDING_CONTACT_CRYPTO: Self = Self(1 << 6);
    /// Source-compatible alias for the original capability name.
    pub const DEFERRED_CONTACT_CRYPTO: Self = Self::PENDING_CONTACT_CRYPTO;
    /// A persisted core wallet snapshot can be loaded after process restart.
    pub const WALLET_RESTORE: Self = Self(1 << 7);
    /// DPNS name-state (username marketplace) rows can be persisted.
    pub const DPNS_NAME_STATES: Self = Self(1 << 8);
    /// Tracked asset-lock rows, including status and proof updates, can be
    /// persisted. Restart hydration is the separate `WALLET_RESTORE` contract.
    pub const TRACKED_ASSET_LOCKS: Self = Self(1 << 9);
    /// A stored `CoreChangeSet` whose `sweeps` are non-empty is durably
    /// applied batch by batch and in order: each swept transaction and its
    /// outputs are excluded from every restore and enumeration path (whether
    /// by physical deletion or a durable marker), each released outpoint is
    /// freed unless a later surviving claim supersedes that release, and each
    /// non-released input retains a durable spend claim even when its funding
    /// TXO has not materialized yet. Physical row deletion is an
    /// implementation detail, not the contract — the in-tree stores keep an
    /// inert globally-swept row until every wallet's scoped cleanup lands,
    /// and a detached tombstone MUST outlive its loser or the consumed coin
    /// later reads unspent. On the FFI surface sweeps travel through the
    /// persistence extension's size-negotiated sweep callback — a slot Rust
    /// never reads unless the host's declared `struct_size` proved it exists
    /// — so an older host processes the rest of the round, returns success,
    /// and never sees the sweeps at all; this bit tells the wallet that the
    /// complete sweep contract was implemented rather than silently
    /// truncated.
    pub const CORE_SWEEP_REMOVAL: Self = Self(1 << 10);
    /// A stored changeset's `dashpay_payments_overlay` rows are durably
    /// applied. This is what lets the wallet-event adapter couple a sweep's
    /// payment consequence (`Pending → Failed` for the losers' sent
    /// entries) to the sweep's own atomic store round: the flip is staged
    /// onto the round ONLY for a backend attesting this bit, because a
    /// sweep never re-emits once its round is durable — an
    /// accepted-and-ignored overlay would leave the adapter believing a
    /// flip persisted that a host without a payments store silently
    /// dropped. A non-attesting backend keeps the in-memory flip (the
    /// truthful session state; the transaction IS dead) with nothing
    /// round-coupled — funds-safe, since payment entries are display
    /// metadata; the funds-critical half of the sweep still gates on
    /// `CORE_SWEEP_REMOVAL`. On the FFI surface Rust honours the
    /// declaration only when `on_persist_dashpay_payments_fn` is actually
    /// wired.
    pub const DASHPAY_PAYMENTS: Self = Self(1 << 11);

    /// Capabilities required before exporting and funding an invitation voucher.
    pub const INVITATION_CREATION: Self = Self(
        Self::ATOMIC_CHANGESETS.0
            | Self::INVITATIONS.0
            | Self::ASSET_LOCK_FUNDING_INDICES.0
            | Self::WALLET_RESTORE.0,
    );

    /// Capabilities required for seed-backed FVK persistence and seedless rebind.
    pub const SHIELDED_FVK_RESTART: Self =
        Self(Self::ATOMIC_CHANGESETS.0 | Self::SHIELDED_VIEWING_KEYS.0);

    /// Capabilities required to durably reconcile an asset-lock status and
    /// restore that exact row after process restart.
    pub const ASSET_LOCK_RECONCILIATION: Self =
        Self(Self::ATOMIC_CHANGESETS.0 | Self::TRACKED_ASSET_LOCKS.0 | Self::WALLET_RESTORE.0);

    pub const fn from_bits_retain(bits: u64) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub const fn contains(self, required: Self) -> bool {
        (self.0 & required.0) == required.0
    }

    pub const fn missing(self, required: Self) -> Self {
        Self(required.0 & !self.0)
    }

    /// Stable names for diagnostics. Unknown future bits are intentionally
    /// omitted; their numeric mask remains available through [`Self::bits`].
    pub fn names(self) -> Vec<&'static str> {
        const KNOWN: &[(PersistenceCapabilities, &str)] = &[
            (
                PersistenceCapabilities::ATOMIC_CHANGESETS,
                "atomic_changesets",
            ),
            (PersistenceCapabilities::INVITATIONS, "invitations"),
            (
                PersistenceCapabilities::ASSET_LOCK_FUNDING_INDICES,
                "asset_lock_funding_indices",
            ),
            (
                PersistenceCapabilities::SHIELDED_VIEWING_KEYS,
                "shielded_viewing_keys",
            ),
            (
                PersistenceCapabilities::PROVIDER_TRANSACTIONS,
                "provider_transactions",
            ),
            (
                PersistenceCapabilities::UNSIGNED_TOKEN_STORAGE,
                "unsigned_token_storage",
            ),
            (
                PersistenceCapabilities::PENDING_CONTACT_CRYPTO,
                "pending_contact_crypto",
            ),
            (PersistenceCapabilities::WALLET_RESTORE, "wallet_restore"),
            (
                PersistenceCapabilities::DPNS_NAME_STATES,
                "dpns_name_states",
            ),
            (
                PersistenceCapabilities::TRACKED_ASSET_LOCKS,
                "tracked_asset_locks",
            ),
            (
                PersistenceCapabilities::CORE_SWEEP_REMOVAL,
                "core_sweep_removal",
            ),
            (
                PersistenceCapabilities::DASHPAY_PAYMENTS,
                "dashpay_payments",
            ),
        ];

        KNOWN
            .iter()
            .filter_map(|(bit, name)| self.contains(*bit).then_some(*name))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_bit_values_are_stable() {
        assert_eq!(PERSISTENCE_CAPABILITIES_VERSION, 1);
        assert_eq!(PersistenceCapabilities::ATOMIC_CHANGESETS.bits(), 0x01);
        assert_eq!(PersistenceCapabilities::INVITATIONS.bits(), 0x02);
        assert_eq!(
            PersistenceCapabilities::ASSET_LOCK_FUNDING_INDICES.bits(),
            0x04
        );
        assert_eq!(PersistenceCapabilities::SHIELDED_VIEWING_KEYS.bits(), 0x08);
        assert_eq!(PersistenceCapabilities::PROVIDER_TRANSACTIONS.bits(), 0x10);
        assert_eq!(PersistenceCapabilities::UNSIGNED_TOKEN_STORAGE.bits(), 0x20);
        assert_eq!(PersistenceCapabilities::PENDING_CONTACT_CRYPTO.bits(), 0x40);
        assert_eq!(PersistenceCapabilities::WALLET_RESTORE.bits(), 0x80);
        assert_eq!(PersistenceCapabilities::DPNS_NAME_STATES.bits(), 0x100);
        assert_eq!(PersistenceCapabilities::TRACKED_ASSET_LOCKS.bits(), 0x200);
        assert_eq!(PersistenceCapabilities::CORE_SWEEP_REMOVAL.bits(), 0x400);
        assert_eq!(PersistenceCapabilities::DASHPAY_PAYMENTS.bits(), 0x800);
        assert_eq!(
            PersistenceCapabilities::ASSET_LOCK_RECONCILIATION.bits(),
            0x281
        );
    }

    #[test]
    fn required_sets_report_only_missing_bits() {
        let actual =
            PersistenceCapabilities::ATOMIC_CHANGESETS.union(PersistenceCapabilities::INVITATIONS);
        let missing = actual.missing(PersistenceCapabilities::INVITATION_CREATION);
        assert_eq!(
            missing.names(),
            vec!["asset_lock_funding_indices", "wallet_restore"]
        );
    }

    /// Every declarable bit must be nameable. A bit missing from `KNOWN`
    /// still gates behaviour but vanishes from every diagnostic that
    /// reports capabilities by name, so a host debugging why its rows
    /// never landed sees nothing about the capability that withheld them
    /// — which is exactly what `DASHPAY_PAYMENTS` did until this test.
    #[test]
    fn every_declared_bit_has_a_stable_name() {
        for shift in 0..12u32 {
            let bit = PersistenceCapabilities::from_bits_retain(1 << shift);
            assert_eq!(
                bit.names().len(),
                1,
                "bit 1 << {shift} is declarable but has no name in KNOWN"
            );
        }
    }
}
