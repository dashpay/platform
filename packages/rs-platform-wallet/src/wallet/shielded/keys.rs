//! Orchard key management for the shielded wallet.
//!
//! Provides [`OrchardKeySet`] which derives the full ZIP-32 key hierarchy
//! from a wallet seed. The derivation path follows the Zcash Orchard spec:
//!
//!   `m / 32' / coin_type' / account'`
//!
//! where `coin_type` is 5 for Dash mainnet and 1 for testnet (BIP-44).
//!
//! All key types are re-exported from `grovedb_commitment_tree` which
//! wraps the upstream `orchard` crate.

use dashcore::Network;
use grovedb_commitment_tree::{
    FullViewingKey, IncomingViewingKey, OutgoingViewingKey, PaymentAddress,
    PreparedIncomingViewingKey, Scope, SpendAuthorizingKey, SpendingKey,
};
use zip32::AccountId;

use crate::error::PlatformWalletError;

/// Dash coin types per BIP-44.
const DASH_COIN_TYPE_MAINNET: u32 = 5;
const DASH_COIN_TYPE_TESTNET: u32 = 1;

/// ZIP-32 derived Orchard key hierarchy.
///
/// Contains all key material needed for shielded operations:
/// - `spending_key` — master secret, needed to authorize spends
/// - `full_viewing_key` — derived from SK, can view all transactions
/// - `spend_auth_key` — signs individual spend authorizations
/// - `incoming_viewing_key` — detects incoming notes (trial decryption)
/// - `outgoing_viewing_key` — recovers sent notes (wallet recovery)
/// - `default_address` — the default payment address at index 0
pub struct OrchardKeySet {
    /// The spending key (master secret).
    pub spending_key: SpendingKey,
    /// Full viewing key derived from the spending key.
    pub full_viewing_key: FullViewingKey,
    /// Spend authorization key for signing spends.
    pub spend_auth_key: SpendAuthorizingKey,
    /// Incoming viewing key for trial decryption.
    pub incoming_viewing_key: IncomingViewingKey,
    /// Outgoing viewing key for wallet recovery.
    pub outgoing_viewing_key: OutgoingViewingKey,
    /// Default payment address (index 0, external scope).
    pub default_address: PaymentAddress,
}

impl OrchardKeySet {
    /// Derive the full Orchard key set from a wallet seed.
    ///
    /// The `seed` should be the BIP-39 seed bytes (typically 64 bytes).
    /// `SpendingKey::from_zip32_seed` accepts seeds of 32-252 bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the seed is invalid or the ZIP-32 derivation
    /// fails (e.g. the derived key is the zero scalar).
    pub fn from_seed(
        seed: &[u8],
        network: Network,
        account: u32,
    ) -> Result<Self, PlatformWalletError> {
        let coin_type = match network {
            Network::Mainnet => DASH_COIN_TYPE_MAINNET,
            _ => DASH_COIN_TYPE_TESTNET,
        };

        let account_id = AccountId::try_from(account).map_err(|_| {
            PlatformWalletError::ShieldedKeyDerivation(format!(
                "invalid account index: {}",
                account
            ))
        })?;

        let sk = SpendingKey::from_zip32_seed(seed, coin_type, account_id).map_err(|e| {
            PlatformWalletError::ShieldedKeyDerivation(format!("ZIP-32 derivation failed: {}", e))
        })?;

        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);
        let ivk = fvk.to_ivk(Scope::External);
        let ovk = fvk.to_ovk(Scope::External);
        let default_address = fvk.address_at(0u32, Scope::External);

        Ok(Self {
            spending_key: sk,
            full_viewing_key: fvk,
            spend_auth_key: ask,
            incoming_viewing_key: ivk,
            outgoing_viewing_key: ovk,
            default_address,
        })
    }

    /// Derive a payment address at the given diversifier index.
    pub fn address_at(&self, index: u32) -> PaymentAddress {
        self.full_viewing_key.address_at(index, Scope::External)
    }

    /// Prepare the incoming viewing key for efficient trial decryption.
    ///
    /// `PreparedIncomingViewingKey` pre-computes values that are reused
    /// across many trial decryption attempts, making batch scanning faster.
    pub fn prepared_ivk(&self) -> PreparedIncomingViewingKey {
        PreparedIncomingViewingKey::new(&self.incoming_viewing_key)
    }
}
