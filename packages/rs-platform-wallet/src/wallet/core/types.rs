//! Per-address and per-account data types for UI consumption.

use dashcore::Address;
use key_wallet::bip32::DerivationPath;
use key_wallet::WalletCoreBalance;

/// Per-address info for UI consumption.
#[derive(Debug, Clone, PartialEq)]
pub struct CoreAddressInfo {
    /// The address itself.
    pub address: Address,
    /// Full HD derivation path for this address.
    pub derivation_path: DerivationPath,
    /// Current balance held at this address (in satoshis).
    pub balance: u64,
    /// Total amount ever received by this address (in satoshis).
    pub total_received: u64,
    /// Number of UTXOs currently held at this address.
    pub utxo_count: usize,
    /// Whether this address has ever been used in a transaction.
    pub is_used: bool,
    /// Index within its address pool.
    pub index: u32,
    /// Account index this address belongs to, if applicable.
    pub account_index: Option<u32>,
}

/// Account-level summary.
#[derive(Debug, Clone)]
pub struct CoreAccountSummary {
    /// Account index, if applicable.
    pub account_index: Option<u32>,
    /// Aggregate balance for this account.
    pub balance: WalletCoreBalance,
    /// Total number of generated addresses across all pools.
    pub address_count: usize,
    /// Number of addresses that have been used.
    pub used_address_count: usize,
}
