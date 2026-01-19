//! Address provider implementation for platform wallet sync
//!
//! This module provides an `AddressProvider` implementation that wraps
//! `ManagedPlatformAccount` for privacy-preserving address balance synchronization.

use crate::error::PlatformWalletError;
use dash_sdk::platform::address_sync::{AddressFunds, AddressIndex, AddressKey, AddressProvider};
use dpp::address_funds::PlatformAddress;
use key_wallet::managed_account::platform_address::PlatformP2PKHAddress;
use key_wallet::ManagedPlatformAccount;
use std::collections::{BTreeMap, BTreeSet};

/// Address provider wrapper for ManagedPlatformAccount
///
/// This wrapper implements `AddressProvider` using addresses already generated
/// in the account's address pool. No key material is required - it uses only
/// pre-existing addresses.
pub struct AccountAddressProvider<'a> {
    /// The account being synced
    account: &'a mut ManagedPlatformAccount,

    /// Gap limit for HD address scanning
    gap_limit: AddressIndex,

    /// Pending addresses: index -> (21-byte key, 20-byte P2PKH hash)
    pending: BTreeMap<AddressIndex, (AddressKey, PlatformP2PKHAddress)>,

    /// Found addresses with balances: index -> (P2PKH hash, funds)
    found: BTreeMap<AddressIndex, (PlatformP2PKHAddress, AddressFunds)>,

    /// Indices proven absent from the tree
    absent: BTreeSet<AddressIndex>,

    /// Highest found index (for gap limit tracking)
    highest_found: Option<AddressIndex>,

    /// Highest index we've added to pending
    highest_pending: AddressIndex,
}

impl<'a> AccountAddressProvider<'a> {
    /// Create a new address provider from a ManagedPlatformAccount
    ///
    /// Uses addresses already in the account's pool. No key derivation is performed.
    pub fn new(account: &'a mut ManagedPlatformAccount) -> Result<Self, PlatformWalletError> {
        let gap_limit = account.gap_limit();

        let mut provider = Self {
            account,
            gap_limit,
            pending: BTreeMap::new(),
            found: BTreeMap::new(),
            absent: BTreeSet::new(),
            highest_found: None,
            highest_pending: 0,
        };

        // Initialize pending addresses from the account's existing pool
        provider.initialize_pending()?;

        Ok(provider)
    }

    /// Initialize pending addresses from account's existing address pool
    fn initialize_pending(&mut self) -> Result<(), PlatformWalletError> {
        let stats = self.account.address_pool_stats();
        let highest_used = stats.highest_used.unwrap_or(0);
        let end_index = highest_used.saturating_add(self.gap_limit);

        // Add all existing addresses up to the gap limit
        for index in 0..=end_index {
            if let Some(address) = self.account.addresses.address_at_index(index) {
                let (key, p2pkh) = Self::address_to_key_and_p2pkh(&address)?;
                self.pending.insert(index, (key, p2pkh));
            }
        }

        self.highest_pending = end_index;
        Ok(())
    }

    /// Convert a dashcore Address to (AddressKey, PlatformP2PKHAddress)
    fn address_to_key_and_p2pkh(
        address: &dashcore::Address,
    ) -> Result<(AddressKey, PlatformP2PKHAddress), PlatformWalletError> {
        let platform_addr = PlatformAddress::try_from(address.clone()).map_err(|e| {
            PlatformWalletError::InvalidPlatformAddress(format!("Failed to convert address: {}", e))
        })?;

        let key = platform_addr.to_bytes(); // 21 bytes: type + hash

        let p2pkh = Self::key_to_p2pkh(&key).ok_or_else(|| {
            PlatformWalletError::InvalidPlatformAddress("Invalid P2PKH address".to_string())
        })?;

        Ok((key, p2pkh))
    }

    /// Convert 21-byte AddressKey to 20-byte PlatformP2PKHAddress
    fn key_to_p2pkh(key: &[u8]) -> Option<PlatformP2PKHAddress> {
        if key.len() != 21 || key[0] != 0x00 {
            return None;
        }
        let hash: [u8; 20] = key[1..21].try_into().ok()?;
        Some(PlatformP2PKHAddress::new(hash))
    }

    /// Get the total balance found across all addresses
    pub fn total_found_balance(&self) -> u64 {
        self.found.values().map(|(_, funds)| funds.balance).sum()
    }

    /// Get the number of addresses with non-zero balance
    pub fn funded_address_count(&self) -> usize {
        self.found
            .values()
            .filter(|(_, funds)| funds.balance > 0)
            .count()
    }

    /// Apply found balances to the account
    pub fn apply_to_account(&mut self) {
        for (_, (p2pkh, funds)) in &self.found {
            // Pass None for key_source since we're using pre-existing addresses
            self.account
                .set_address_credit_balance(*p2pkh, funds.balance, None);
        }
    }
}

impl<'a> AddressProvider for AccountAddressProvider<'a> {
    fn gap_limit(&self) -> AddressIndex {
        self.gap_limit
    }

    fn pending_addresses(&self) -> Vec<(AddressIndex, AddressKey)> {
        self.pending
            .iter()
            .map(|(idx, (key, _))| (*idx, key.clone()))
            .collect()
    }

    fn on_address_found(&mut self, index: AddressIndex, key: &[u8], funds: AddressFunds) {
        if let Some((_, p2pkh)) = self.pending.remove(&index) {
            self.found.insert(index, (p2pkh, funds));
            self.highest_found = Some(self.highest_found.map_or(index, |h| h.max(index)));

            // Extend pending if this address has a balance
            if funds.balance > 0 {
                let new_end = index.saturating_add(self.gap_limit);

                for new_idx in (self.highest_pending + 1)..=new_end {
                    if !self.found.contains_key(&new_idx)
                        && !self.absent.contains(&new_idx)
                        && !self.pending.contains_key(&new_idx)
                    {
                        // Try to add from existing pool
                        if let Some(address) = self.account.addresses.address_at_index(new_idx) {
                            if let Ok((new_key, new_p2pkh)) =
                                Self::address_to_key_and_p2pkh(&address)
                            {
                                self.pending.insert(new_idx, (new_key, new_p2pkh));
                            }
                        }
                    }
                }

                if new_end > self.highest_pending {
                    self.highest_pending = new_end;
                }
            }
        } else if let Some(p2pkh) = Self::key_to_p2pkh(key) {
            self.found.insert(index, (p2pkh, funds));
            self.highest_found = Some(self.highest_found.map_or(index, |h| h.max(index)));
        }
    }

    fn on_address_absent(&mut self, index: AddressIndex, _key: &[u8]) {
        self.pending.remove(&index);
        self.absent.insert(index);
    }

    fn highest_found_index(&self) -> Option<AddressIndex> {
        self.highest_found
    }
}

/// Result of platform address synchronization
#[derive(Debug, Clone)]
pub struct PlatformSyncResult {
    /// Whether a full sync was performed
    pub full_sync_performed: bool,

    /// Checkpoint height from full sync (Platform block height)
    pub checkpoint_height: u64,

    /// Highest terminal block processed
    pub highest_terminal_block: u64,

    /// Total credits found across all addresses
    pub total_balance: u64,

    /// Number of addresses with non-zero balance
    pub funded_address_count: usize,

    /// Highest address index found with a balance
    pub highest_found_index: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_to_p2pkh_valid() {
        let mut key = vec![0x00]; // P2PKH type
        key.extend_from_slice(&[0xAB; 20]); // 20-byte hash

        let p2pkh = AccountAddressProvider::key_to_p2pkh(&key);
        assert!(p2pkh.is_some());
        assert_eq!(p2pkh.unwrap().as_bytes(), &[0xAB; 20]);
    }

    #[test]
    fn test_key_to_p2pkh_wrong_type() {
        let mut key = vec![0x01]; // P2SH type (not P2PKH)
        key.extend_from_slice(&[0xAB; 20]);

        let p2pkh = AccountAddressProvider::key_to_p2pkh(&key);
        assert!(p2pkh.is_none());
    }

    #[test]
    fn test_key_to_p2pkh_wrong_length() {
        let key = vec![0x00, 0xAB, 0xAB]; // Too short

        let p2pkh = AccountAddressProvider::key_to_p2pkh(&key);
        assert!(p2pkh.is_none());
    }
}
