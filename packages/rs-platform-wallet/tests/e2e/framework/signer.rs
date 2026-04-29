//! Seed-backed `Signer<PlatformAddress>` that pre-derives the
//! `account=0, key_class=0` clear-funds gap window via DIP-17
//! (`m/9'/coin_type'/17'/account'/key_class'/index`) and serves
//! signing requests via a `HashMap<address_hash, secret>` lookup.
//! `can_sign_with` is a real cache check, not a permissive `true`.
//! Keeps keying material on the test side so the production wallet
//! API stays free of test-only seed accessors.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use dpp::dashcore::signer as core_signer;
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use dpp::util::hash::ripemd160_sha256;
use dpp::ProtocolError;
use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
use key_wallet::{AccountType, ChildNumber, Network};
use parking_lot::Mutex;

use super::{FrameworkError, FrameworkResult};

/// DIP-17 default account / key-class for clear-funds platform
/// payments. Matches `WalletAccountCreationOptions::Default`.
const DEFAULT_ACCOUNT_INDEX: u32 = 0;
const DEFAULT_KEY_CLASS: u32 = 0;

/// Default gap window pre-derived at construction
/// (`key-wallet`'s `DIP17_GAP_LIMIT`).
pub const DEFAULT_GAP_LIMIT: u32 = 20;

/// 20-byte P2PKH address hash → 32-byte secp256k1 secret.
type AddressKeyMap = HashMap<[u8; 20], [u8; 32]>;

/// Resolves `Signer<PlatformAddress>::sign` against a seed-derived
/// key cache. Construction is fallible; the hot path is sync.
#[derive(Clone)]
pub struct SeedBackedPlatformAddressSigner {
    /// `Arc<Mutex<_>>` for cheap cloning across signers; the
    /// `Mutex` keeps the map extensible if a test exceeds the
    /// gap window.
    cache: Arc<Mutex<AddressKeyMap>>,
}

impl SeedBackedPlatformAddressSigner {
    /// Pre-derive the [`DEFAULT_GAP_LIMIT`] window for `seed_bytes`
    /// on `network`. Use [`Self::new_with_gap`] for a custom window.
    pub fn new(seed_bytes: &[u8; 64], network: Network) -> FrameworkResult<Self> {
        Self::new_with_gap(seed_bytes, network, DEFAULT_GAP_LIMIT)
    }

    /// Same as [`Self::new`] but with an explicit gap-window size.
    pub fn new_with_gap(
        seed_bytes: &[u8; 64],
        network: Network,
        gap_limit: u32,
    ) -> FrameworkResult<Self> {
        let root_priv = RootExtendedPrivKey::new_master(seed_bytes).map_err(|err| {
            FrameworkError::Wallet(format!(
                "SeedBackedPlatformAddressSigner: invalid seed for root xpriv: {err}"
            ))
        })?;
        let root_xpriv = root_priv.to_extended_priv_key(network);

        let account_path = AccountType::PlatformPayment {
            account: DEFAULT_ACCOUNT_INDEX,
            key_class: DEFAULT_KEY_CLASS,
        }
        .derivation_path(network)
        .map_err(|err| {
            FrameworkError::Wallet(format!(
                "SeedBackedPlatformAddressSigner: derivation path: {err}"
            ))
        })?;

        let secp = Secp256k1::new();
        let mut cache = AddressKeyMap::with_capacity(gap_limit as usize);
        for index in 0..gap_limit {
            let leaf = ChildNumber::from_normal_idx(index).map_err(|err| {
                FrameworkError::Wallet(format!(
                    "SeedBackedPlatformAddressSigner: invalid leaf index {index}: {err}"
                ))
            })?;
            // `extend` returns a fresh path; account_path is reused
            // across iterations.
            let leaf_path = account_path.extend([leaf]);
            let xpriv = root_xpriv.derive_priv(&secp, &leaf_path).map_err(|err| {
                FrameworkError::Wallet(format!(
                    "SeedBackedPlatformAddressSigner: derive_priv at index {index}: {err}"
                ))
            })?;
            let secret: SecretKey = xpriv.private_key;
            let pubkey: PublicKey = PublicKey::from_secret_key(&secp, &secret);
            // Compressed pubkey → RIPEMD160(SHA256(·)) → 20-byte
            // P2PKH address hash; matches dashcore's
            // `PrivateKey::public_key().pubkey_hash()`.
            let pkh = ripemd160_sha256(&pubkey.serialize());
            cache.insert(pkh, secret.secret_bytes());
        }
        Ok(Self {
            cache: Arc::new(Mutex::new(cache)),
        })
    }

    /// Number of pre-derived keys in the cache.
    pub fn cached_key_count(&self) -> usize {
        self.cache.lock().len()
    }
}

impl std::fmt::Debug for SeedBackedPlatformAddressSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeedBackedPlatformAddressSigner")
            .field("cache_size", &self.cache.lock().len())
            .finish()
    }
}

#[async_trait]
impl Signer<PlatformAddress> for SeedBackedPlatformAddressSigner {
    async fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        let secret = lookup_secret(&self.cache, key)?;
        let signature = core_signer::sign(data, &secret)?;
        Ok(signature.to_vec().into())
    }

    async fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(key, data).await?;
        match key {
            PlatformAddress::P2pkh(_) => Ok(AddressWitness::P2pkh { signature }),
            PlatformAddress::P2sh(_) => Err(ProtocolError::Generic(
                "SeedBackedPlatformAddressSigner: P2SH witnesses are not supported".into(),
            )),
        }
    }

    fn can_sign_with(&self, key: &PlatformAddress) -> bool {
        match key {
            PlatformAddress::P2pkh(hash) => self.cache.lock().contains_key(hash),
            PlatformAddress::P2sh(_) => false,
        }
    }
}

/// Resolve a [`PlatformAddress`] to its pre-derived secret, or
/// surface a [`ProtocolError`] naming the missing address. Local
/// `result_large_err` allow because the test binary doesn't inherit
/// the crate-root `#![allow(...)]`.
#[allow(clippy::result_large_err)]
fn lookup_secret(
    cache: &Mutex<AddressKeyMap>,
    addr: &PlatformAddress,
) -> Result<[u8; 32], ProtocolError> {
    let hash = match addr {
        PlatformAddress::P2pkh(h) => h,
        PlatformAddress::P2sh(_) => {
            return Err(ProtocolError::Generic(
                "SeedBackedPlatformAddressSigner: P2SH addresses are not supported".into(),
            ));
        }
    };
    cache.lock().get(hash).copied().ok_or_else(|| {
        ProtocolError::Generic(format!(
            "SeedBackedPlatformAddressSigner: address {} not in pre-derived gap window",
            hex::encode(hash)
        ))
    })
}
