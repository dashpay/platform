//! Unified event types for the platform wallet.

#[cfg(feature = "manager")]
pub use key_wallet_manager::WalletEvent;

#[cfg(not(feature = "manager"))]
#[derive(Debug, Clone)]
pub enum WalletEvent {
    TransactionReceived {
        wallet_id: [u8; 32],
        account_index: u32,
        txid: dashcore::Txid,
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

#[derive(Debug, Clone)]
pub enum PlatformWalletEvent {
    Wallet(WalletEvent),
    Spv(SpvEvent),
    Finality(FinalityEvent),
}

#[derive(Debug, Clone)]
pub enum SpvEvent {
    SyncProgress { height: u32, total: u32 },
    PeerConnected { address: String },
    PeerDisconnected { address: String },
}

#[derive(Debug, Clone)]
pub enum FinalityEvent {
    InstantLock { txid: dashcore::Txid },
    ChainLock { height: u32 },
}
