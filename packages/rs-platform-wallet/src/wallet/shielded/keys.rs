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
/// Contains the key material needed for shielded sync and address
/// generation. The master `SpendingKey` is intentionally not retained:
/// it is derived inside [`Self::from_seed`] only long enough to extract
/// the FVK / ASK / IVK / OVK and is dropped before this struct is
/// returned. Spend authorization for an actual transaction re-derives
/// the SK transiently from the wallet seed via the host signer.
///
/// - `full_viewing_key` — derived from SK, can view all transactions
/// - `spend_auth_key` — signs individual spend authorizations
/// - `incoming_viewing_key` — detects incoming notes (trial decryption)
/// - `outgoing_viewing_key` — recovers sent notes (wallet recovery)
/// - `default_address` — the default payment address at index 0
pub struct OrchardKeySet {
    /// Full viewing key derived from the spending key.
    pub full_viewing_key: FullViewingKey,
    /// Spend authorization key for signing spends. Crate-private.
    pub(crate) spend_auth_key: SpendAuthorizingKey,
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
        // `sk` falls out of scope here. The FVK / ASK / IVK / OVK
        // already capture every quantity the wallet needs; spend
        // authorization is re-derived transiently from the wallet
        // seed via the host signer at sign time. (Orchard
        // `SpendingKey` is `Copy`, so explicit zeroization of this
        // local would require wrapping in `Zeroizing`; revisit when
        // the spend signer lands.)

        Ok(Self {
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

    /// Strip the spend-authorizing key and return only the
    /// viewing-grade material. Used to populate the
    /// network-scoped shielded coordinator's account registry —
    /// the coordinator runs sync (trial-decrypt + tree append +
    /// nullifier scan), none of which needs spend authority, so
    /// keeping the ASK on the per-wallet side preserves the
    /// privilege separation. Spend operations re-attach the ASK
    /// by passing the full [`OrchardKeySet`] back into the
    /// coordinator's spend methods at call time.
    pub fn viewing_keys(&self) -> AccountViewingKeys {
        AccountViewingKeys {
            full_viewing_key: self.full_viewing_key.clone(),
            incoming_viewing_key: self.incoming_viewing_key.clone(),
            prepared_ivk: self.prepared_ivk(),
            outgoing_viewing_key: self.outgoing_viewing_key.clone(),
            default_address: self.default_address,
        }
    }
}

/// Viewing-grade subset of an [`OrchardKeySet`] — the material
/// needed to detect, decrypt, and recover Orchard notes, with no
/// ability to authorize a spend.
///
/// The network-scoped shielded coordinator holds these for every
/// bound `(walletId, accountIndex)`; it never sees a
/// `SpendAuthorizingKey`. Spend operations are driven from the
/// per-wallet side, which holds the full [`OrchardKeySet`] (ASK
/// included) and passes it into the coordinator's spend methods
/// only for the duration of that call.
#[derive(Clone)]
pub struct AccountViewingKeys {
    pub full_viewing_key: FullViewingKey,
    pub incoming_viewing_key: IncomingViewingKey,
    /// Pre-computed for fast trial-decrypt across many notes per
    /// sync pass. Cached at registration time so the sync loop
    /// doesn't pay [`PreparedIncomingViewingKey::new`] per pass.
    pub prepared_ivk: PreparedIncomingViewingKey,
    pub outgoing_viewing_key: OutgoingViewingKey,
    pub default_address: PaymentAddress,
}
