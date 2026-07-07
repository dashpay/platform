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
    /// ZIP-32 requires master seeds of 32-252 bytes; the underlying
    /// `SpendingKey::from_zip32_seed` does not enforce that bound
    /// itself, so it is checked here.
    ///
    /// # Errors
    ///
    /// Returns an error if the seed length is out of range or the
    /// ZIP-32 derivation fails (e.g. the derived key is the zero
    /// scalar).
    pub fn from_seed(
        seed: &[u8],
        network: Network,
        account: u32,
    ) -> Result<Self, PlatformWalletError> {
        if seed.len() < 32 || seed.len() > 252 {
            return Err(PlatformWalletError::ShieldedKeyDerivation(format!(
                "seed must be 32..=252 bytes per ZIP-32, got {}",
                seed.len()
            )));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb_commitment_tree::SpendingKey;

    // Test vector 0 from zcash-test-vectors
    // (`orchard_key_components.py`), the same data the orchard
    // fork's own `keys::tests::test_vectors` checks against. Here
    // it runs through the `grovedb_commitment_tree` re-exports —
    // the exact dependency chain `OrchardKeySet::from_seed` uses —
    // so a grovedb or orchard bump that changes sk → FVK / IVK /
    // OVK / address derivation fails this test, not a wallet sync
    // in the field.
    const TV_SK: [u8; 32] = [
        0x5d, 0x7a, 0x8f, 0x73, 0x9a, 0x2d, 0x9e, 0x94, 0x5b, 0x0c, 0xe1, 0x52, 0xa8, 0x04, 0x9e,
        0x29, 0x4c, 0x4d, 0x6e, 0x66, 0xb1, 0x64, 0x93, 0x9d, 0xaf, 0xfa, 0x2e, 0xf6, 0xee, 0x69,
        0x21, 0x48,
    ];
    const TV_IVK: [u8; 32] = [
        0x85, 0xc8, 0xb5, 0xcd, 0x1a, 0xc3, 0xec, 0x3a, 0xd7, 0x09, 0x21, 0x32, 0xf9, 0x7f, 0x01,
        0x78, 0xb0, 0x75, 0xc8, 0x1a, 0x13, 0x9f, 0xd4, 0x60, 0xbb, 0xe0, 0xdf, 0xcd, 0x75, 0x51,
        0x47, 0x24,
    ];
    const TV_OVK: [u8; 32] = [
        0xbc, 0xc7, 0x06, 0x5e, 0x59, 0x91, 0x0b, 0x35, 0x99, 0x3f, 0x59, 0x50, 0x5b, 0xe2, 0x09,
        0xb1, 0x4b, 0xf0, 0x24, 0x88, 0x75, 0x0b, 0xbc, 0x8b, 0x1a, 0xcd, 0xcf, 0x10, 0x8c, 0x36,
        0x20, 0x04,
    ];
    const TV_DK: [u8; 32] = [
        0x31, 0xd6, 0xa6, 0x85, 0xbe, 0x57, 0x0f, 0x9f, 0xaf, 0x3c, 0xa8, 0xb0, 0x52, 0xe8, 0x87,
        0x84, 0x0b, 0x2c, 0x9f, 0x8d, 0x67, 0x22, 0x4c, 0xa8, 0x2a, 0xef, 0xb9, 0xe2, 0xee, 0x5b,
        0xed, 0xaf,
    ];
    const TV_DEFAULT_D: [u8; 11] = [
        0x8f, 0xf3, 0x38, 0x69, 0x71, 0xcb, 0x64, 0xb8, 0xe7, 0x78, 0x99,
    ];
    const TV_DEFAULT_PK_D: [u8; 32] = [
        0x08, 0xdd, 0x8e, 0xbd, 0x7d, 0xe9, 0x2a, 0x68, 0xe5, 0x86, 0xa3, 0x4d, 0xb8, 0xfe, 0xa9,
        0x99, 0xef, 0xd2, 0x01, 0x6f, 0xae, 0x76, 0x75, 0x0a, 0xfa, 0xe7, 0xee, 0x94, 0x16, 0x46,
        0xbc, 0xb9,
    ];

    #[test]
    fn key_pipeline_matches_official_orchard_test_vector() {
        let sk = Option::<SpendingKey>::from(SpendingKey::from_bytes(TV_SK))
            .expect("test vector spending key is valid");
        let fvk = FullViewingKey::from(&sk);

        // Raw IVK encoding is dk ‖ ivk (Zcash protocol § 5.6.4.3).
        let ivk_bytes = fvk.to_ivk(Scope::External).to_bytes();
        assert_eq!(&ivk_bytes[..32], &TV_DK, "diversifier key mismatch");
        assert_eq!(&ivk_bytes[32..], &TV_IVK, "incoming viewing key mismatch");

        let ovk = fvk.to_ovk(Scope::External);
        assert_eq!(ovk.as_ref(), &TV_OVK, "outgoing viewing key mismatch");

        // Raw address encoding is d ‖ pk_d; the "default" address in
        // the vectors is diversifier index 0, which is what
        // `OrchardKeySet::from_seed` exposes as `default_address`.
        let raw = fvk.address_at(0u32, Scope::External).to_raw_address_bytes();
        assert_eq!(&raw[..11], &TV_DEFAULT_D, "default diversifier mismatch");
        assert_eq!(&raw[11..], &TV_DEFAULT_PK_D, "default pk_d mismatch");
    }

    #[test]
    fn from_seed_is_deterministic_and_domain_separated() {
        let seed = [0x42u8; 64];

        let a = OrchardKeySet::from_seed(&seed, Network::Testnet, 0).expect("derivation succeeds");
        let b = OrchardKeySet::from_seed(&seed, Network::Testnet, 0).expect("derivation succeeds");
        assert_eq!(
            a.default_address.to_raw_address_bytes(),
            b.default_address.to_raw_address_bytes(),
            "same seed/network/account must derive the same address"
        );
        assert_eq!(
            a.incoming_viewing_key.to_bytes(),
            b.incoming_viewing_key.to_bytes(),
            "same seed/network/account must derive the same IVK"
        );

        // coin_type 5 vs 1 — a mainnet wallet must not share keys
        // with a testnet wallet on the same seed.
        let mainnet =
            OrchardKeySet::from_seed(&seed, Network::Mainnet, 0).expect("derivation succeeds");
        assert_ne!(
            mainnet.default_address.to_raw_address_bytes(),
            a.default_address.to_raw_address_bytes(),
            "mainnet and testnet must derive different addresses"
        );

        // Devnet intentionally shares the testnet coin type (the
        // `_ =>` arm in `from_seed`).
        let devnet =
            OrchardKeySet::from_seed(&seed, Network::Devnet, 0).expect("derivation succeeds");
        assert_eq!(
            devnet.default_address.to_raw_address_bytes(),
            a.default_address.to_raw_address_bytes(),
            "devnet shares the testnet coin type"
        );

        let account1 =
            OrchardKeySet::from_seed(&seed, Network::Testnet, 1).expect("derivation succeeds");
        assert_ne!(
            account1.default_address.to_raw_address_bytes(),
            a.default_address.to_raw_address_bytes(),
            "accounts must derive different addresses"
        );

        // ZIP-32 hardened child bounds: account indices are u31.
        assert!(
            OrchardKeySet::from_seed(&seed, Network::Testnet, 1 << 31).is_err(),
            "account index ≥ 2^31 must be rejected"
        );
        // ZIP-32 master seed must be 32..=252 bytes.
        assert!(
            OrchardKeySet::from_seed(&[0u8; 16], Network::Testnet, 0).is_err(),
            "16-byte seed must be rejected"
        );
    }

    /// Known-answer pin for the full `from_seed` path (ZIP-32
    /// m/32'/1'/0' on a fixed seed). The expected bytes were
    /// generated by this code at the verified dependency pin
    /// (dashpay/orchard `dashified-0.14.0`, whose ZIP-32 and key
    /// test vectors pass upstream's official suite). If this test
    /// ever fails, derivation changed — existing wallets would
    /// stop seeing their notes. Do not update the constants
    /// without a migration story.
    #[test]
    fn from_seed_known_answer_pin() {
        let seed = [0x42u8; 64];
        let ks = OrchardKeySet::from_seed(&seed, Network::Testnet, 0).expect("derivation succeeds");

        let addr = ks.default_address.to_raw_address_bytes();
        let ivk = ks.incoming_viewing_key.to_bytes();

        const EXPECTED_ADDRESS: &str =
            "ee9f8174f92a3f035570ecbfe969aeb46f5e2f64ad69f78d34316c47ea38c2f0085b5788bebf478ce736a8";
        const EXPECTED_IVK: &str =
            "fae18cbcf032c37f646b0e3f211bda62dc79535f5276abbf274f46ba1d28d571946102f72db50fd672aadddc8346c513221c82e3fbc0c62058a2effb9669f228";
        assert_eq!(
            hex::encode(addr),
            EXPECTED_ADDRESS,
            "default address drifted"
        );
        assert_eq!(hex::encode(ivk), EXPECTED_IVK, "raw IVK encoding drifted");
    }
}
