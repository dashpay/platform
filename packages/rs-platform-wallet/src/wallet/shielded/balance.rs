//! Local, reservation-aware shielded balance snapshots. Reading one never
//! starts a network request or resolves the wallet seed.

use std::collections::BTreeMap;

use crate::error::PlatformWalletError;

use super::is_shielded_identity_account;

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

impl ShieldedLocalBalanceSnapshot {
    /// Spendable credits of the wallet's ordinary accounts: the wallet's
    /// shielded balance. Identity accounts ([`is_shielded_identity_account`])
    /// are bound and scanned with the wallet, but an ordinary send cannot
    /// spend their notes, so they are left out. A host shows this number and
    /// does not add up accounts or notes itself. Rejects a total that does
    /// not fit in `u64`.
    pub fn ordinary_spendable_total(&self) -> Result<u64, PlatformWalletError> {
        self.accounts
            .iter()
            .filter(|(account, _)| !is_shielded_identity_account(**account))
            .try_fold(0u64, |total, (_, balance)| {
                total.checked_add(balance.spendable_credits).ok_or_else(|| {
                    PlatformWalletError::ShieldedStoreError(
                        "wallet-wide shielded balance exceeds u64".to_string(),
                    )
                })
            })
    }
}

/// An unavailable ledger must never masquerade as a successfully read zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShieldedLocalBalanceState {
    Unbound,
    /// Binding registered the keys, but neither host restoration nor completed
    /// in-session scans establish the ledger for every account.
    RestoreIncomplete,
    Ready(ShieldedLocalBalanceSnapshot),
}
