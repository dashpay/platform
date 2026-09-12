//! Local, reservation-aware shielded balance snapshots. Reading one never
//! starts a network request or resolves the wallet seed.

use std::collections::BTreeMap;

/// Evidence backing the local ledger, independent of its current balance.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ShieldedBalanceSource {
    /// Keys are bound, but no notes or completed scan have been restored.
    #[default]
    NoHistory,
    /// Notes and/or a completed scan watermark were restored from persistence.
    Restored,
    /// A complete note scan succeeded in this process, including an empty scan.
    ScannedThisSession,
}

/// Balance and scan coverage for one bound Orchard account.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ShieldedLocalAccountBalance {
    /// Unspent credits excluding notes reserved by pending spends. This is the
    /// same local ledger used by note selection, not a promise of network freshness.
    pub spendable_credits: u64,
    /// Exclusive scan watermark. `None` means no completed scan is known;
    /// `Some(0)` preserves a completed scan of an empty shielded pool.
    pub last_scanned_index: Option<u64>,
    pub source: ShieldedBalanceSource,
}

/// Every account bound to one wallet, captured under one store read lock.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ShieldedLocalBalanceSnapshot {
    pub accounts: BTreeMap<u32, ShieldedLocalAccountBalance>,
}

/// An unavailable ledger must never masquerade as a successfully read zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShieldedLocalBalanceState {
    Unbound,
    /// Binding registered the keys, but restoring the ledger did not complete.
    RestoreIncomplete,
    Ready(ShieldedLocalBalanceSnapshot),
}
