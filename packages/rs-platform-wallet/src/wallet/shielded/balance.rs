//! Local, reservation-aware shielded balance snapshots. Reading one never
//! starts a network request or resolves the wallet seed.

use std::collections::BTreeMap;

use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::WalletId;

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

impl ShieldedLocalAccountBalance {
    /// Whether `spendable_credits` is a known amount for this account: its
    /// ledger rests on a restored history or a completed scan. False while
    /// the account has no history, when its zero says nothing about what it
    /// holds. The one rule for it: the ordinary balance uses it, and hosts
    /// receive its result with each account instead of reading `source`.
    pub fn is_established(&self) -> bool {
        self.source != ShieldedBalanceSource::NoHistory
    }
}

/// Every account bound to one wallet, captured under one store read lock.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ShieldedLocalBalanceSnapshot {
    pub accounts: BTreeMap<u32, ShieldedLocalAccountBalance>,
}

/// The wallet's shielded balance with the evidence behind it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ShieldedOrdinaryBalance {
    /// Spendable credits of the wallet's ordinary accounts.
    pub spendable_credits: u64,
    /// Whether every ordinary account's ledger rests on a restored history or
    /// a completed scan ([`ShieldedBalanceSource`] other than `NoHistory`).
    /// When false, `spendable_credits` is what the ledger holds so far, not a
    /// known balance: a hydrated account with no stored notes and no completed
    /// scan reads zero without that zero being known. True for a wallet with
    /// no ordinary account, whose ordinary balance is zero by construction.
    pub established: bool,
}

impl ShieldedLocalBalanceSnapshot {
    /// The wallet's shielded balance: the spendable credits of its ordinary
    /// accounts, and whether each of their ledgers is established. Identity
    /// accounts ([`is_shielded_identity_account`]) are bound and scanned with
    /// the wallet, but an ordinary send cannot spend their notes, so they are
    /// left out. A host shows this and does not add up accounts or notes
    /// itself. Rejects a total that does not fit in `u64`.
    pub fn ordinary_balance(&self) -> Result<ShieldedOrdinaryBalance, PlatformWalletError> {
        self.accounts
            .iter()
            .filter(|(account, _)| !is_shielded_identity_account(**account))
            .try_fold(
                ShieldedOrdinaryBalance {
                    spendable_credits: 0,
                    established: true,
                },
                |total, (_, balance)| {
                    let spendable_credits = total
                        .spendable_credits
                        .checked_add(balance.spendable_credits)
                        .ok_or_else(|| {
                            PlatformWalletError::ShieldedStoreError(
                                "wallet-wide shielded balance exceeds u64".to_string(),
                            )
                        })?;
                    Ok(ShieldedOrdinaryBalance {
                        spendable_credits,
                        established: total.established && balance.is_established(),
                    })
                },
            )
    }
}

/// An unavailable ledger must never masquerade as a successfully read zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShieldedLocalBalanceState {
    Unbound,
    /// Binding registered the keys, but neither host restoration nor completed
    /// in-session scans establish the ledger for every account. The balance
    /// emitter also delivers it for a ledger it could not read (one account's
    /// stored notes past `u64`): that ledger is not established either.
    /// `NetworkShieldedCoordinator::local_balance_snapshot` itself returns the
    /// read error in that case.
    RestoreIncomplete,
    Ready(ShieldedLocalBalanceSnapshot),
}

/// One wallet's local balance state, pushed to the host whenever the state
/// the emitter reads differs from the state last pushed for that wallet
/// (`PlatformEventHandler::on_shielded_balance_changed`). It is the latest
/// state read, not every transition: changes between two reads coalesce, and
/// a reservation taken and released before the next read produces no update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShieldedBalanceUpdate {
    pub wallet_id: WalletId,
    /// Identifies a delivered state, not a ledger mutation. Per wallet,
    /// starting at 1 and strictly increasing for the life of the coordinator.
    /// A wallet's updates are delivered in revision order, so a host keeps the
    /// one with the highest revision.
    pub revision: u64,
    /// What `NetworkShieldedCoordinator::local_balance_snapshot` returns for
    /// the wallet now.
    pub state: ShieldedLocalBalanceState,
}
