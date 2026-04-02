//! Unified event types for the platform wallet.

use dashcore::Txid;

#[cfg(feature = "manager")]
pub use key_wallet_manager::WalletEvent;

#[cfg(not(feature = "manager"))]
#[derive(Debug, Clone)]
pub enum WalletEvent {
    TransactionReceived {
        wallet_id: [u8; 32],
        account_index: u32,
        txid: Txid,
        amount: i64,
        addresses: Vec<dashcore::Address>,
    },
    BalanceUpdated {
        wallet_id: [u8; 32],
        spendable: u64,
        unconfirmed: u64,
        immature: u64,
        locked: u64,
    },
}

/// Transaction finality status lifecycle.
///
/// Progresses: `Unconfirmed → InstantSendLocked → Confirmed → ChainLocked`.
/// Each state is >= the previous, so `PartialOrd`/`Ord` reflect finality ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TransactionStatus {
    /// In mempool, no InstantSend lock.
    Unconfirmed = 0,
    /// InstantSend-locked but not yet mined.
    InstantSendLocked = 1,
    /// Mined in a block.
    Confirmed = 2,
    /// In a chain-locked block (highest finality).
    ChainLocked = 3,
}

impl TransactionStatus {
    /// Deserialize from stored u8 value.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Unconfirmed),
            1 => Some(Self::InstantSendLocked),
            2 => Some(Self::Confirmed),
            3 => Some(Self::ChainLocked),
            _ => None,
        }
    }

    /// User-facing label for this status.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unconfirmed => "Unconfirmed",
            Self::InstantSendLocked => "InstantSend Locked",
            Self::Confirmed => "Confirmed",
            Self::ChainLocked => "Chain Locked",
        }
    }
}

/// Unified event enum for the platform wallet system.
///
/// Wraps events from dash-spv directly — no duplicate enums.
#[derive(Debug, Clone)]
pub enum PlatformWalletEvent {
    /// Wallet-level events (transaction received, balance updated).
    Wallet(WalletEvent),
    /// SPV sync events (headers stored, sync complete, chain/instant locks, etc.).
    #[cfg(feature = "manager")]
    Sync(dash_spv::sync::SyncEvent),
    /// SPV network events (peer connected/disconnected/updated).
    #[cfg(feature = "manager")]
    Network(dash_spv::network::NetworkEvent),
    /// SPV sync progress update.
    #[cfg(feature = "manager")]
    Progress(dash_spv::sync::SyncProgress),
    /// Transaction status changed (finality lifecycle).
    TransactionStatusChanged {
        txid: Txid,
        old_status: TransactionStatus,
        new_status: TransactionStatus,
    },
}
