//! On-demand private-key derivation for a single core (Layer-1) address.
//!
//! The wallet already knows the full BIP-32 derivation path of every
//! address it tracks (it is stored on the address's
//! [`AddressInfo`](key_wallet::managed_account::address_pool::AddressInfo)
//! inside the managed account collection). This module joins that path
//! back to key material so a caller can reveal the private key for one
//! address without knowing anything about derivation-path shapes — the
//! whole lookup-and-derive happens here, on the Rust side, in a single
//! call.
//!
//! # Key source
//!
//! The wallets the iOS app holds are external-signable (watch-only from
//! Rust's point of view): the BIP-39 seed lives in the iOS Keychain, not
//! in the `WalletManager`. For those, the caller resolves the mnemonic on
//! demand and hands us the master [`ExtendedPrivKey`] as `resolved_master`.
//! Wallets that *do* hold resident private keys (created from a raw seed)
//! pass `None` and we derive straight from the in-process key-wallet.
//! Either way the returned scalar is wrapped in [`Zeroizing`] so it is
//! scrubbed when dropped.

use std::str::FromStr;

use dashcore::secp256k1::Secp256k1;
use dashcore::{Address, PrivateKey as DashPrivateKey};
use key_wallet::bip32::{DerivationPath, ExtendedPrivKey};
use zeroize::Zeroizing;

use super::platform_wallet::PlatformWallet;
use crate::error::PlatformWalletError;

/// The private key for one of a wallet's core addresses, in the two
/// forms a caller wants to display: raw 32-byte scalar (sensitive, held
/// in a [`Zeroizing`] buffer) and its network-aware compressed WIF.
///
/// The raw scalar backs the hex rendering the FFI produces; both it and
/// the WIF are wiped by the FFI free function once marshalled to the
/// host.
pub struct CoreAddressPrivateKey {
    /// The address's full BIP-32 derivation path (informational — the
    /// same value already stored on the address row).
    pub derivation_path: DerivationPath,
    /// Raw 32-byte secp256k1 private-key scalar. Zeroized on drop.
    pub private_key: Zeroizing<[u8; 32]>,
    /// Private key in WIF (Wallet Import Format) — network-aware
    /// (mainnet vs testnet/devnet/regtest version byte) and compressed.
    pub wif: String,
}

impl PlatformWallet {
    /// Derive the private key for `address_str`, one of this wallet's
    /// tracked core addresses.
    ///
    /// Looks the address up across *every* managed account pool (standard
    /// BIP44/BIP32, CoinJoin, identity registration/top-up funding,
    /// provider keys, DashPay) to find its full derivation path, then
    /// derives the secp256k1 private key at that path.
    ///
    /// `resolved_master` selects the key source (see the module docs):
    /// `Some(master)` for external-signable / watch-only wallets whose
    /// mnemonic the caller resolved on demand; `None` to derive from a
    /// resident key-bearing wallet.
    ///
    /// # Errors
    /// - [`PlatformWalletError::AddressOperation`] if `address_str` is not
    ///   a valid address for this wallet's network.
    /// - [`PlatformWalletError::AddressNotFound`] if the address is valid
    ///   but not tracked by any of this wallet's accounts.
    /// - [`PlatformWalletError::KeyDerivation`] if key derivation fails —
    ///   including passing `None` for a watch-only wallet that has no
    ///   resident private keys.
    pub fn derive_core_address_private_key(
        &self,
        address_str: &str,
        resolved_master: Option<&ExtendedPrivKey>,
    ) -> Result<CoreAddressPrivateKey, PlatformWalletError> {
        let network = self.network();

        // Parse + network-check the address against this wallet's network
        // so a foreign-network string can't accidentally match a pool
        // entry (base58 prefixes overlap across the test networks).
        let address = Address::from_str(address_str)
            .map_err(|e| {
                PlatformWalletError::AddressOperation(format!(
                    "invalid address '{address_str}': {e}"
                ))
            })?
            .require_network(network)
            .map_err(|e| {
                PlatformWalletError::AddressOperation(format!(
                    "address '{address_str}' is not valid for {network:?}: {e}"
                ))
            })?;

        // Single read-lock: the path lookup reads the managed account
        // collection and the resident-key derive borrows the in-process
        // wallet from the same guard. No Swift callback runs under this
        // guard — the mnemonic resolver (if any) already ran on the FFI
        // side and produced `resolved_master` before we were called.
        let state = self.state_blocking();

        let path = state
            .core_wallet
            .accounts
            .all_accounts()
            .into_iter()
            .find_map(|account| account.get_address_info(&address).map(|info| info.path))
            .ok_or_else(|| {
                PlatformWalletError::AddressNotFound(format!(
                    "address '{address_str}' is not tracked by any account in this wallet"
                ))
            })?;

        // Derive the raw scalar from the selected key source.
        let secret_bytes: Zeroizing<[u8; 32]> = match resolved_master {
            Some(master) => {
                let secp = Secp256k1::new();
                let derived = master.derive_priv(&secp, &path).map_err(|e| {
                    PlatformWalletError::KeyDerivation(format!(
                        "failed to derive private key at {path}: {e}"
                    ))
                })?;
                Zeroizing::new(derived.private_key.secret_bytes())
            }
            None => {
                // Resident key-bearing wallet — derive from its own root.
                // Errors here for a watch-only / external-signable wallet
                // (no resident private key), which the caller must instead
                // service with a `resolved_master`.
                let secret_key = state.wallet().derive_private_key(&path).map_err(|e| {
                    PlatformWalletError::KeyDerivation(format!(
                        "failed to derive private key at {path} from resident wallet: {e}"
                    ))
                })?;
                Zeroizing::new(secret_key.secret_bytes())
            }
        };

        // Build the network-aware compressed WIF. `SecretKey::from_slice`
        // over the just-derived bytes is infallible, but map its error
        // rather than unwrap to keep the boundary panic-free.
        let secret_key = dashcore::secp256k1::SecretKey::from_slice(secret_bytes.as_ref())
            .map_err(|e| {
                PlatformWalletError::KeyDerivation(format!(
                    "derived private key bytes were not a valid secp256k1 scalar: {e}"
                ))
            })?;
        let wif = DashPrivateKey {
            compressed: true,
            network,
            inner: secret_key,
        }
        .to_wif();

        Ok(CoreAddressPrivateKey {
            derivation_path: path,
            private_key: secret_bytes,
            wif,
        })
    }
}
