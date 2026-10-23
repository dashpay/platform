//! Seed-based signer derivation for the offline and live e2e harness.

use dpp::util::hash::ripemd160_sha256;
use key_wallet::bip32::{Error as Bip32Error, KeyDerivationType};
use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
use key_wallet::{AccountType, ChildNumber, DerivationPath, Error as KeyWalletError, Network};
use simple_signer::signer::SimpleSigner;

/// Errors returned by the seed-based eager-derivation helpers.
#[derive(Debug, thiserror::Error)]
pub enum SignerDerivationError {
    /// The seed produced an invalid root extended private key.
    #[error("invalid seed for root xpriv: {0}")]
    InvalidSeed(#[source] KeyWalletError),
    /// The DIP-17 / DIP-9 derivation path failed to construct.
    #[error("derivation path: {0}")]
    DerivationPath(#[source] KeyWalletError),
    /// `derive_priv` failed at the given leaf index.
    #[error("derive_priv at index {index}: {source}")]
    DerivePriv {
        /// Leaf index that failed.
        index: u32,
        /// Underlying key-wallet BIP-32 error.
        #[source]
        source: Bip32Error,
    },
    /// A leaf [`ChildNumber`] could not be constructed from the requested index.
    #[error("invalid leaf index {index}: {source}")]
    InvalidIndex {
        /// Offending leaf index.
        index: u32,
        /// Underlying key-wallet BIP-32 error.
        #[source]
        source: Bip32Error,
    },
}

/// Build a [`SimpleSigner`] populated with the DIP-17 platform-payment
/// gap window for `(account, key_class)`. Each leaf
/// `m/9'/coin_type'/17'/account'/key_class'/index` derives a
/// secp256k1 keypair; the 20-byte RIPEMD160(SHA256(pubkey)) hash is
/// inserted into [`SimpleSigner::address_private_keys`].
pub fn from_seed_for_platform_address_account(
    seed: &[u8; 64],
    network: Network,
    account: u32,
    key_class: u32,
    gap_limit: u32,
) -> Result<SimpleSigner, SignerDerivationError> {
    from_seed_for_platform_addresses(seed, network, account, key_class, 0..gap_limit)
}

/// Build a [`SimpleSigner`] populated with the DIP-17 platform-payment
/// keys for an explicit set of derivation `indices` under
/// `(account, key_class)`. Each `index` derives
/// `m/9'/coin_type'/17'/account'/key_class'/index`; the 20-byte
/// RIPEMD160(SHA256(pubkey)) hash is inserted into
/// [`SimpleSigner::address_private_keys`].
///
/// Use this over [`from_seed_for_platform_address_account`] when
/// the address set is not the contiguous `0..gap_limit` window — e.g.
/// a long-lived wallet whose synced funded pool has cycled past the
/// first gap window. Duplicate indices are deduplicated by the
/// underlying key map. Returns the first derivation error encountered.
pub fn from_seed_for_platform_addresses<I: IntoIterator<Item = u32>>(
    seed: &[u8; 64],
    network: Network,
    account: u32,
    key_class: u32,
    indices: I,
) -> Result<SimpleSigner, SignerDerivationError> {
    let root_priv =
        RootExtendedPrivKey::new_master(seed).map_err(SignerDerivationError::InvalidSeed)?;
    let root_xpriv = root_priv.to_extended_priv_key(network);

    let account_path = AccountType::PlatformPayment { account, key_class }
        .derivation_path(network)
        .map_err(SignerDerivationError::DerivationPath)?;

    let mut signer = SimpleSigner::default();
    for index in indices {
        let leaf = ChildNumber::from_normal_idx(index)
            .map_err(|source| SignerDerivationError::InvalidIndex { index, source })?;
        // `extend` returns a fresh path; account_path is reused.
        let leaf_path = account_path.extend([leaf]);
        let xpriv = root_xpriv
            .derive_priv(&leaf_path)
            .map_err(|source| SignerDerivationError::DerivePriv { index, source })?;
        let pkh = ripemd160_sha256(&xpriv.private_key.public_key().to_compressed());
        signer.add_address_key(pkh, *xpriv.private_key.to_bytes());
    }
    Ok(signer)
}

/// Build a [`SimpleSigner`] populated with the DIP-9 identity-authentication
/// (ECDSA) gap window for `identity_index`. The returned signer holds raw
/// secp256k1 secrets keyed on `(pubkey-hash, secret)` via
/// [`SimpleSigner::address_private_keys`] — callers that need a `Signer<IdentityPublicKey>`
/// view must additionally register `IdentityPublicKey` records via
/// [`SimpleSigner::add_identity_public_key`] using the matching pubkey bytes.
pub fn from_seed_for_identity(
    seed: &[u8; 64],
    network: Network,
    identity_index: u32,
    gap_limit: u32,
) -> Result<SimpleSigner, SignerDerivationError> {
    let root_priv =
        RootExtendedPrivKey::new_master(seed).map_err(SignerDerivationError::InvalidSeed)?;
    let root_xpriv = root_priv.to_extended_priv_key(network);

    let mut signer = SimpleSigner::default();
    for key_index in 0..gap_limit {
        let leaf_path = DerivationPath::identity_authentication_path(
            network,
            KeyDerivationType::ECDSA,
            identity_index,
            key_index,
        );
        let xpriv = root_xpriv.derive_priv(&leaf_path).map_err(|source| {
            SignerDerivationError::DerivePriv {
                index: key_index,
                source,
            }
        })?;
        let pkh = ripemd160_sha256(&xpriv.private_key.public_key().to_compressed());
        signer.add_address_key(pkh, *xpriv.private_key.to_bytes());
    }
    Ok(signer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::{ExtendedPrivKey, Network};
    use std::collections::BTreeMap;
    use std::str::FromStr;

    const SEED: [u8; 64] = [0x42; 64];

    /// `(pubkey hash, secret)` of `path` derived straight from the seed,
    /// independent of the constructors' `AccountType` / DIP-9 path builders.
    fn expected_entry(path: &str) -> ([u8; 20], [u8; 32]) {
        let master = ExtendedPrivKey::new_master(Network::Testnet, &SEED).expect("master key");
        let path = DerivationPath::from_str(path).expect("valid path literal");
        let xpriv = master.derive_priv(&path).expect("derive");
        let hash = ripemd160_sha256(&xpriv.private_key.public_key().to_compressed());
        (hash, *xpriv.private_key.to_bytes())
    }

    #[test]
    fn should_derive_dip17_platform_payment_keys_for_requested_indices() {
        let signer = from_seed_for_platform_addresses(&SEED, Network::Testnet, 2, 0, [0, 5])
            .expect("derive");

        let expected: BTreeMap<_, _> = ["m/9'/1'/17'/2'/0'/0", "m/9'/1'/17'/2'/0'/5"]
            .into_iter()
            .map(expected_entry)
            .collect();
        assert_eq!(signer.address_private_keys, expected);
        assert!(signer.private_keys.is_empty());
    }

    #[test]
    fn should_derive_the_contiguous_gap_window_for_an_account() {
        let window = from_seed_for_platform_address_account(&SEED, Network::Testnet, 0, 0, 3)
            .expect("derive window");
        let explicit = from_seed_for_platform_addresses(&SEED, Network::Testnet, 0, 0, 0..3)
            .expect("derive explicit");

        assert_eq!(window.address_private_keys.len(), 3);
        assert_eq!(window.address_private_keys, explicit.address_private_keys);
    }

    #[test]
    fn should_reject_a_hardened_range_index_with_a_typed_error() {
        let index = 1u32 << 31;
        let result = from_seed_for_platform_addresses(&SEED, Network::Testnet, 0, 0, [index]);

        assert!(
            matches!(
                result,
                Err(SignerDerivationError::InvalidIndex {
                    index: reported,
                    source: Bip32Error::InvalidChildNumber(child),
                }) if reported == index && child == index
            ),
            "expected InvalidIndex for {index}, got {result:?}"
        );
    }

    #[test]
    fn should_derive_dip9_identity_authentication_keys() {
        let signer = from_seed_for_identity(&SEED, Network::Testnet, 4, 2).expect("derive");

        let expected: BTreeMap<_, _> = ["m/9'/1'/5'/0'/0'/4'/0'", "m/9'/1'/5'/0'/0'/4'/1'"]
            .into_iter()
            .map(expected_entry)
            .collect();
        assert_eq!(signer.address_private_keys, expected);
    }
}
