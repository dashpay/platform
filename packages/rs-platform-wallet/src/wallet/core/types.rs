//! Per-address data types for UI consumption.

use std::collections::BTreeMap;

use dashcore::Address;
use key_wallet::bip32::DerivationPath;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

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

impl CoreAddressInfo {
    /// Build a `CoreAddressInfo` list for every address across all accounts.
    ///
    /// Iterates all managed accounts and their address pools, building a
    /// `CoreAddressInfo` for each generated address. UTXO counts are
    /// computed by scanning the account's UTXO map.
    pub fn all_from_wallet_info(info: &ManagedWalletInfo) -> Vec<Self> {
        let mut result = Vec::new();

        for account in info.accounts.all_accounts() {
            let account_index = account.index();

            // Build a quick per-address UTXO count from the account's utxo map.
            let mut utxo_counts: BTreeMap<Address, usize> = BTreeMap::new();
            for utxo in account.utxos.values() {
                *utxo_counts.entry(utxo.address.clone()).or_default() += 1;
            }

            for pool in account.account_type.address_pools() {
                for addr_info in pool.addresses.values() {
                    result.push(CoreAddressInfo {
                        address: addr_info.address.clone(),
                        derivation_path: addr_info.path.clone(),
                        balance: addr_info.balance,
                        total_received: addr_info.total_received,
                        utxo_count: utxo_counts.get(&addr_info.address).copied().unwrap_or(0),
                        is_used: addr_info.used,
                        index: addr_info.index,
                        account_index,
                    });
                }
            }
        }

        result
    }
}
