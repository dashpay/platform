//! Shared fail-closed `wallet_id` gate for resolver-fed sign entrypoints.
//!
//! Every FFI entrypoint that takes a `MnemonicResolverHandle` plus an
//! `expected wallet_id` and derives signing keys from the resolved
//! mnemonic MUST run the gate immediately after deriving the root
//! extended public key, *before* it touches `derive_priv` (and well
//! before any signature material is emitted).
//!
//! The gate constant-time compares the loaded `wallet_id` against the
//! one recomputed from the resolver-supplied root extended public key.
//! On mismatch it returns the structural failure tag the caller hands
//! to its error path — no rendered hex, no key bytes.
//!
//! This is the fail-closed counterpart to the seedless load path:
//! `load_from_persistor` reconstructs each persisted wallet
//! **watch-only** (no derivation, no gate); wrong-seed detection moves
//! here, where the gate is meaningful (the resolver actually produced
//! signing material).

use key_wallet::bip32::ExtendedPubKey;
use key_wallet::wallet::root_extended_keys::RootExtendedPubKey;
use key_wallet::wallet::Wallet;
use subtle::ConstantTimeEq;

/// Recompute `wallet_id` from the supplied root extended public key and
/// constant-time compare against `expected_wallet_id`.
///
/// Returns `true` on match, `false` on mismatch. The boolean is
/// intentionally information-free beyond the verdict: the caller maps
/// `false` onto its own structural failure tag
/// (`SIGN_WITH_RESOLVER_ERR_WRONG_SEED` and friends). No key material
/// crosses this surface in either direction.
#[inline]
#[must_use]
pub fn verify_seed_matches_wallet_id(
    root_pub: &ExtendedPubKey,
    expected_wallet_id: &[u8; 32],
) -> bool {
    let root_extended = RootExtendedPubKey::from_extended_pub_key(root_pub);
    let derived = Wallet::compute_wallet_id_from_root_extended_pub_key(&root_extended);
    bool::from(derived.ct_eq(expected_wallet_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::secp256k1::Secp256k1;
    use key_wallet::bip32::ExtendedPrivKey;

    fn root_xpub_for_seed(seed: &[u8; 64]) -> (ExtendedPubKey, [u8; 32]) {
        let master =
            ExtendedPrivKey::new_master(key_wallet::Network::Testnet, seed.as_ref()).unwrap();
        let secp = Secp256k1::new();
        let xpub = ExtendedPubKey::from_priv(&secp, &master);
        let root = RootExtendedPubKey::from_extended_pub_key(&xpub);
        let wid = Wallet::compute_wallet_id_from_root_extended_pub_key(&root);
        (xpub, wid)
    }

    #[test]
    fn matching_seed_passes() {
        let seed = [0x11; 64];
        let (xpub, wid) = root_xpub_for_seed(&seed);
        assert!(verify_seed_matches_wallet_id(&xpub, &wid));
    }

    #[test]
    fn mismatched_seed_fails() {
        let seed_a = [0x11; 64];
        let seed_b = [0x22; 64];
        let (xpub_a, _) = root_xpub_for_seed(&seed_a);
        let (_, wid_b) = root_xpub_for_seed(&seed_b);
        assert!(!verify_seed_matches_wallet_id(&xpub_a, &wid_b));
    }
}
