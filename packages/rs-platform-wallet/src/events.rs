//! Unified event types for the platform wallet.

pub use key_wallet_manager::WalletEvent;

/// SPV event — groups sync, network, and progress events from dash-spv.
#[derive(Debug, Clone)]
pub enum SpvEvent {
    /// Sync lifecycle events (headers stored, sync complete, chain/instant locks, etc.).
    Sync(dash_spv::sync::SyncEvent),
    /// Network events (peer connected/disconnected/updated).
    Network(dash_spv::network::NetworkEvent),
    /// Overall sync progress update.
    Progress(dash_spv::sync::SyncProgress),
}

/// Unified event enum for the platform wallet system.
///
/// Wraps events from dash-spv and key-wallet-manager directly.
#[derive(Debug, Clone)]
pub enum PlatformWalletEvent {
    /// Wallet-level events (transaction received, balance updated, status changed).
    Wallet(WalletEvent),
    /// SPV events (sync, network, progress).
    Spv(SpvEvent),
}
