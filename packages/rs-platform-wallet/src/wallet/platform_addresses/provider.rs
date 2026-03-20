//! DIP-17 platform payment address provider for HD wallet scanning.

use std::collections::BTreeMap;
use std::sync::Arc;

use dpp::address_funds::PlatformAddress;
use key_wallet::bip32::{ChildNumber, DerivationPath};
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use tokio::sync::RwLock;

use dash_sdk::platform::address_sync::{
    AddressFunds, AddressIndex, AddressKey, AddressProvider,
};

/// Default gap limit for HD wallet address scanning.
pub(crate) const DEFAULT_GAP_LIMIT: u32 = 20;

/// Build a DIP-17 platform payment derivation path.
///
/// Path: `m/9'/<coin_type>'/17'/<account>'/<key_class>'/<index>`
fn platform_payment_path(
    network: Network,
    account: u32,
    key_class: u32,
    index: u32,
) -> DerivationPath {
    let coin_type = match network {
        Network::Mainnet => 5,
        _ => 1,
    };
    DerivationPath::from(vec![
        ChildNumber::Hardened { index: 9 },
        ChildNumber::Hardened { index: coin_type },
        ChildNumber::Hardened { index: 17 },
        ChildNumber::Hardened { index: account },
        ChildNumber::Hardened { index: key_class },
        ChildNumber::Normal { index },
    ])
}

/// Derive a platform address at a given index using the wallet's key derivation.
///
/// Returns `(address_key_bytes, core_address)`.
fn derive_platform_address_at(
    wallet: &Wallet,
    network: Network,
    account: u32,
    key_class: u32,
    index: u32,
) -> Result<(AddressKey, dashcore::Address), String> {
    let path = platform_payment_path(network, account, key_class, index);

    let extended_private_key = wallet
        .derive_extended_private_key(&path)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    let secp = dashcore::secp256k1::Secp256k1::new();
    let private_key = extended_private_key.to_priv();
    let public_key = private_key.public_key(&secp);

    let address = dashcore::Address::p2pkh(&public_key, network);

    let platform_addr = PlatformAddress::try_from(address.clone())
        .map_err(|e| format!("Failed to convert to PlatformAddress: {}", e))?;
    let key = platform_addr.to_bytes();

    Ok((key, address))
}

/// Internal address provider implementing [`AddressProvider`] for DIP-17
/// platform payment address discovery.
///
/// This provider pre-derives platform payment addresses from the wallet and
/// supports HD gap limit scanning. Addresses are derived upfront so the wallet
/// lock is not held during the async sync operation.
pub(crate) struct PlatformPaymentAddressProvider {
    /// Network for address derivation.
    network: Network,
    /// Gap limit for HD wallet scanning.
    gap_limit: u32,
    /// Pre-derived addresses: index -> (key_bytes, core_address).
    pending: BTreeMap<u32, (AddressKey, dashcore::Address)>,
    /// Indices that have been resolved (found or absent).
    resolved: std::collections::BTreeSet<u32>,
    /// Highest index found with a non-zero balance.
    highest_found: Option<u32>,
    /// Wallet reference for lazy address extension during gap limit scanning.
    wallet: Arc<RwLock<Wallet>>,
    /// Account index.
    account: u32,
    /// Key class.
    key_class: u32,
}

impl PlatformPaymentAddressProvider {
    /// Create an address provider from a wallet.
    ///
    /// Pre-derives the initial set of addresses (up to the gap limit).
    /// The wallet must support private key derivation (not watch-only).
    pub(crate) fn from_wallet(
        wallet: Arc<RwLock<Wallet>>,
        network: Network,
    ) -> Result<Self, String> {
        let mut provider = Self {
            network,
            gap_limit: DEFAULT_GAP_LIMIT,
            pending: BTreeMap::new(),
            resolved: std::collections::BTreeSet::new(),
            highest_found: None,
            wallet,
            account: 0,
            key_class: 0,
        };

        // Bootstrap initial addresses (0 to gap_limit - 1).
        provider.ensure_addresses_up_to(DEFAULT_GAP_LIMIT.saturating_sub(1))?;

        Ok(provider)
    }

    /// Ensure addresses are derived up to and including the given index.
    fn ensure_addresses_up_to(&mut self, max_index: u32) -> Result<(), String> {
        let current_max = self.pending.keys().max().copied();
        let start = current_max.map(|m| m + 1).unwrap_or(0);

        // Acquire read lock only when we actually need to derive keys.
        if start > max_index {
            return Ok(());
        }

        let wallet = self.wallet.blocking_read();
        for index in start..=max_index {
            if !self.pending.contains_key(&index) && !self.resolved.contains(&index) {
                let (key, address) = derive_platform_address_at(
                    &wallet,
                    self.network,
                    self.account,
                    self.key_class,
                    index,
                )?;
                self.pending.insert(index, (key, address));
            }
        }
        Ok(())
    }

    /// Extend pending addresses based on gap limit after finding an address.
    fn extend_for_gap_limit(&mut self, found_index: u32) -> Result<(), String> {
        let new_end = found_index.saturating_add(self.gap_limit);
        self.ensure_addresses_up_to(new_end)
    }
}

impl AddressProvider for PlatformPaymentAddressProvider {
    fn gap_limit(&self) -> AddressIndex {
        self.gap_limit
    }

    fn pending_addresses(&self) -> Vec<(AddressIndex, AddressKey)> {
        self.pending
            .iter()
            .filter(|(index, _)| !self.resolved.contains(index))
            .map(|(index, (key, _))| (*index, key.clone()))
            .collect()
    }

    fn on_address_found(&mut self, index: AddressIndex, _key: &[u8], _funds: AddressFunds) {
        self.resolved.insert(index);

        // Any found address (including zero-balance) indicates prior use
        // and should extend the scanning window.
        self.highest_found = Some(self.highest_found.map(|h| h.max(index)).unwrap_or(index));

        if let Err(e) = self.extend_for_gap_limit(index) {
            tracing::warn!("Failed to extend addresses for gap limit: {}", e);
        }
    }

    fn on_address_absent(&mut self, index: AddressIndex, _key: &[u8]) {
        self.resolved.insert(index);
    }

    fn has_pending(&self) -> bool {
        self.pending
            .keys()
            .any(|index| !self.resolved.contains(index))
    }

    fn highest_found_index(&self) -> Option<AddressIndex> {
        self.highest_found
    }
}
