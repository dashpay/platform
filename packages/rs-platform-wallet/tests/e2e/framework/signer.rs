//! Seed-backed `Signer<PlatformAddress>` adapter.
//!
//! Bridges DPP's [`Signer`] trait to a `platform_wallet::PlatformWallet`
//! by:
//!
//! 1. Looking up `(account_index, key_class, key_index)` for an
//!    address via
//!    [`PlatformAddressWallet::address_derivation_info`] (the
//!    accessor added in Wave 1).
//! 2. Deriving the matching ECDSA private key from the wallet's
//!    seed at the DIP-17 path
//!    `m/9'/coin_type'/17'/account'/key_class'/index`.
//! 3. Caching the 32-byte secret in an internal map keyed by
//!    20-byte address hash so subsequent `sign` calls skip the
//!    derivation walk.
//!
//! Wave 3a delivers the full implementation. Wave 4 wires
//! `wallet_factory::TestWallet::address_signer` to return `&Self`.

use std::sync::Arc;

use async_trait::async_trait;
use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::dashcore::signer as core_signer;
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use key_wallet::{AccountType, ChildNumber};
use parking_lot::Mutex;
use platform_wallet::PlatformWallet;
use std::collections::HashMap;

/// Cached signer that derives ECDSA private keys on demand from the
/// wallet's seed. The wallet itself is the source of truth for
/// derivation paths and seed material — the signer just walks DIP-17
/// to materialise per-address secrets.
///
/// Cloning the signer is cheap (`Arc<PlatformWallet>` clone + a
/// shared cache), so test flows that need multiple in-flight signers
/// for the same wallet share one cache by cloning.
#[derive(Clone)]
pub struct SeedBackedPlatformAddressSigner {
    /// The wallet whose seed material backs this signer.
    wallet: Arc<PlatformWallet>,
    /// Cache: address hash -> 32-byte secp256k1 secret. Populated
    /// lazily by [`SeedBackedPlatformAddressSigner::ensure_key`]; a
    /// `parking_lot::Mutex` is used because the critical section
    /// is purely synchronous (lookup + memcpy).
    cache: Arc<Mutex<HashMap<[u8; 20], [u8; 32]>>>,
}

impl SeedBackedPlatformAddressSigner {
    /// Build a new signer backed by `wallet`'s seed material.
    pub fn new(wallet: Arc<PlatformWallet>) -> Self {
        Self {
            wallet,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Ensure the cache holds the secret for `addr`, deriving it
    /// from the seed if necessary.
    ///
    /// Returns `Ok(secret)` after either a cache hit or a successful
    /// derivation; `Err` propagates as a [`ProtocolError`] so the
    /// `Signer<PlatformAddress>` trait shape stays clean.
    async fn ensure_key(&self, addr: &PlatformAddress) -> Result<[u8; 32], ProtocolError> {
        let hash = match addr {
            PlatformAddress::P2pkh(h) => *h,
            PlatformAddress::P2sh(_) => {
                return Err(ProtocolError::Generic(
                    "SeedBackedPlatformAddressSigner: P2SH addresses are not supported".into(),
                ));
            }
        };

        // Fast path — hit while holding the lock for as little as
        // possible. The HashMap access is lock-free w.r.t. async, so
        // we never `await` while holding the parking_lot mutex.
        if let Some(secret) = self.cache.lock().get(&hash).copied() {
            return Ok(secret);
        }

        // Cold path: resolve derivation coords, walk the path
        // against the wallet's root key, cache and return.
        let info = self
            .wallet
            .platform()
            .address_derivation_info(addr)
            .await
            .ok_or_else(|| {
                ProtocolError::Generic(format!(
                    "SeedBackedPlatformAddressSigner: address {:?} not owned by wallet {}",
                    addr,
                    hex::encode(self.wallet.wallet_id())
                ))
            })?;

        let network = self.wallet.sdk().network;
        let secret = {
            let wm = self.wallet.wallet_manager().read().await;
            let wallet = wm.get_wallet(&self.wallet.wallet_id()).ok_or_else(|| {
                ProtocolError::Generic(format!(
                    "SeedBackedPlatformAddressSigner: wallet {} not in WalletManager",
                    hex::encode(self.wallet.wallet_id())
                ))
            })?;
            let mut path = AccountType::PlatformPayment {
                account: info.account_index,
                key_class: info.key_class,
            }
            .derivation_path(network)
            .map_err(|err| {
                ProtocolError::Generic(format!(
                    "SeedBackedPlatformAddressSigner: derivation path: {err}"
                ))
            })?;
            // DIP-17 leaves are non-hardened.
            path.push(ChildNumber::from_normal_idx(info.key_index).map_err(|err| {
                ProtocolError::Generic(format!(
                    "SeedBackedPlatformAddressSigner: invalid leaf index {}: {err}",
                    info.key_index
                ))
            })?);
            let key = wallet.derive_private_key(&path).map_err(|err| {
                ProtocolError::Generic(format!(
                    "SeedBackedPlatformAddressSigner: derive_private_key: {err}"
                ))
            })?;
            key.secret_bytes()
        };

        self.cache.lock().insert(hash, secret);
        Ok(secret)
    }
}

impl std::fmt::Debug for SeedBackedPlatformAddressSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeedBackedPlatformAddressSigner")
            .field("wallet_id", &hex::encode(self.wallet.wallet_id()))
            .field("cache_size", &self.cache.lock().len())
            .finish()
    }
}

#[async_trait]
impl Signer<PlatformAddress> for SeedBackedPlatformAddressSigner {
    async fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        let secret = self.ensure_key(key).await?;
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
        // Trait is sync; `address_derivation_info` is async. Treat
        // the signer as universally capable of signing P2PKH and
        // let `sign` itself surface ownership errors — the SDK
        // still proceeds correctly because it delegates to `sign`
        // for the actual proof. Cached entries short-circuit.
        match key {
            PlatformAddress::P2pkh(hash) => {
                if self.cache.lock().contains_key(hash) {
                    return true;
                }
                true
            }
            PlatformAddress::P2sh(_) => false,
        }
    }
}
