//! Seed-backed `Signer<PlatformAddress>` adapter.
//!
//! At construction time the signer eagerly derives every key in the
//! `account=0, key_class=0` (clear-funds) gap window from the
//! provided seed bytes via the DIP-17 path
//! `m/9'/coin_type'/17'/account'/key_class'/index`, computes each
//! address (RIPEMD160(SHA256(compressed pubkey))), and stores the
//! 32-byte ECDSA secret keyed by 20-byte address hash. Signing
//! requests then become a synchronous map lookup — no wallet round
//! trip, no async derivation in the hot path, and `can_sign_with`
//! reports honestly (it's a real cache check, not a permissive
//! `true`).
//!
//! Keeping the keying material entirely on the test-framework side
//! also keeps the upstream `rs-platform-wallet` production surface
//! free of any test-only convenience accessors — the wallet doesn't
//! expose seed bytes or per-address derivation info, and the
//! framework doesn't need it to sign.

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
/// payments. Mirrors `WalletAccountCreationOptions::Default` which
/// the e2e bank and test wallets both use.
const DEFAULT_ACCOUNT_INDEX: u32 = 0;
const DEFAULT_KEY_CLASS: u32 = 0;

/// Default gap window pre-derived at construction. 20 keys is the
/// `key-wallet` `DIP17_GAP_LIMIT` and matches the e2e harness's
/// per-account address pool default. The current test scope uses
/// at most 2 fresh receive addresses per wallet — 20 is comfortably
/// above the working set.
pub const DEFAULT_GAP_LIMIT: u32 = 20;

/// Pre-derived address keymap. Values are 32-byte secp256k1 secret
/// keys keyed by the 20-byte P2PKH address hash. The map is built
/// once in [`SeedBackedPlatformAddressSigner::new`]; signing
/// requests then become a synchronous `HashMap::get` away from a
/// real ECDSA signature.
type AddressKeyMap = HashMap<[u8; 20], [u8; 32]>;

/// Signer that resolves `Signer<PlatformAddress>::sign` against a
/// seed-derived key cache.
///
/// Construction is fallible (the seed must produce a valid root
/// extended private key + DIP-17 derivation path); after that the
/// signer is fully synchronous on the hot path.
#[derive(Clone)]
pub struct SeedBackedPlatformAddressSigner {
    /// `Arc` so the signer can be cloned cheaply (e.g. one bank
    /// signer + N test-wallet signers all share the same backing
    /// map type without re-keying it). The map itself is read-only
    /// after construction; the `Mutex` is just here so we can
    /// extend it later if a future test exceeds the gap window.
    cache: Arc<Mutex<AddressKeyMap>>,
}

impl SeedBackedPlatformAddressSigner {
    /// Build a new signer by pre-deriving every clear-funds address
    /// in the gap window for `seed_bytes` on `network`.
    ///
    /// `gap_limit` controls how many leaf indices `0..gap_limit`
    /// are pre-derived. [`DEFAULT_GAP_LIMIT`] (20) is plenty for
    /// the current test scope; bump it via [`Self::new_with_gap`]
    /// if a future test needs a wider window.
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
            // `DerivationPath::extend` returns a fresh path with
            // the leaf appended; the account path is reused
            // across iterations (it has no mutating accessor).
            let leaf_path = account_path.extend([leaf]);
            let xpriv = root_xpriv.derive_priv(&secp, &leaf_path).map_err(|err| {
                FrameworkError::Wallet(format!(
                    "SeedBackedPlatformAddressSigner: derive_priv at index {index}: {err}"
                ))
            })?;
            let secret: SecretKey = xpriv.private_key;
            let pubkey: PublicKey = PublicKey::from_secret_key(&secp, &secret);
            // 33-byte compressed public key → RIPEMD160(SHA256(.))
            // → 20-byte P2PKH address hash. Matches dashcore's
            // `PrivateKey::public_key().pubkey_hash()` shape used
            // by `simple-signer` and the SDK's address-funds path.
            let pkh = ripemd160_sha256(&pubkey.serialize());
            cache.insert(pkh, secret.secret_bytes());
        }
        Ok(Self {
            cache: Arc::new(Mutex::new(cache)),
        })
    }

    /// Number of pre-derived keys currently in the cache. Useful
    /// for diagnostic logs and for tests that want to assert on
    /// the gap window without poking at the internals.
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

/// Resolve a [`PlatformAddress`] to its pre-derived 32-byte secret
/// key, or surface a [`ProtocolError`] naming the missing address.
///
/// `ProtocolError` is large (`clippy::result_large_err`) but the
/// crate as a whole already allows it (`#![allow(clippy::result_large_err)]`
/// in `src/lib.rs`); the test binary doesn't share that root attr,
/// so we silence the lint locally rather than box every call site.
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
