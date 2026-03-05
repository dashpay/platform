//! Shared key derivation utilities for identity authentication keys
//!
//! This module provides helper functions used by both the matured transactions
//! processor and the identity discovery scanner.

use crate::error::PlatformWalletError;
use key_wallet::Network;

/// Derive the 20-byte RIPEMD160(SHA256) hash of the public key at the given
/// identity authentication path.
///
/// Path format: `base_path / identity_index' / key_index'`
/// where `base_path` is `m/9'/COIN_TYPE'/5'/0'` (mainnet or testnet).
///
/// # Arguments
///
/// * `wallet` - The wallet to derive keys from
/// * `network` - Network to select the correct coin type
/// * `identity_index` - The identity index (hardened)
/// * `key_index` - The key index within that identity (hardened)
///
/// # Returns
///
/// Returns the 20-byte public key hash suitable for Platform identity lookup.
pub(crate) fn derive_identity_auth_key_hash(
    wallet: &key_wallet::Wallet,
    network: Network,
    identity_index: u32,
    key_index: u32,
) -> Result<[u8; 20], PlatformWalletError> {
    use dashcore::secp256k1::Secp256k1;
    use dpp::util::hash::ripemd160_sha256;
    use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPubKey};
    use key_wallet::dip9::{
        IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
    };

    let base_path = match network {
        Network::Dash => IDENTITY_AUTHENTICATION_PATH_MAINNET,
        Network::Testnet => IDENTITY_AUTHENTICATION_PATH_TESTNET,
        _ => {
            return Err(PlatformWalletError::InvalidIdentityData(
                "Unsupported network for identity derivation".to_string(),
            ));
        }
    };

    let mut full_path = DerivationPath::from(base_path);
    full_path = full_path.extend([
        ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid identity index: {}", e))
        })?,
        ChildNumber::from_hardened_idx(key_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid key index: {}", e))
        })?,
    ]);

    let auth_key = wallet
        .derive_extended_private_key(&full_path)
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive authentication key: {}",
                e
            ))
        })?;

    let secp = Secp256k1::new();
    let public_key = ExtendedPubKey::from_priv(&secp, &auth_key);
    let public_key_bytes = public_key.public_key.serialize();
    let key_hash = ripemd160_sha256(&public_key_bytes);

    let mut key_hash_array = [0u8; 20];
    key_hash_array.copy_from_slice(&key_hash);

    Ok(key_hash_array)
}
