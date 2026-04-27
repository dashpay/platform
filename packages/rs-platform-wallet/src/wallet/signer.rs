//! Signer for identity operations using wallet-derived keys.

use std::sync::Arc;

use async_trait::async_trait;
use dpp::address_funds::AddressWitness;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyType;
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use key_wallet::Network;
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;
use zeroize::Zeroizing;

use crate::broadcaster::SpvBroadcaster;
use crate::wallet::identity::network::IdentityWallet;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

/// A signer that uses wallet-derived keys to sign identity state transitions.
pub struct IdentitySigner {
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: WalletId,
    network: Network,
    identity_index: u32,
}

impl IdentitySigner {
    /// Create a new IdentitySigner for a specific identity index.
    pub(crate) fn new(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        network: Network,
        identity_index: u32,
    ) -> Self {
        Self {
            wallet_manager,
            wallet_id,
            network,
            identity_index,
        }
    }

    /// Get the identity index this signer is associated with.
    #[allow(dead_code)]
    pub(crate) fn identity_index(&self) -> u32 {
        self.identity_index
    }

    /// Derive the raw private key bytes for a given identity public key.
    ///
    /// Delegates to [`IdentityWallet::derive_identity_key_bytes`] for the
    /// actual DIP-9 path construction and key derivation.
    ///
    /// Returns the bytes wrapped in [`Zeroizing`] so they are automatically
    /// wiped from memory when the value is dropped.
    ///
    /// The shared lock is acquired and released within this method.
    /// Uses async `read().await` — calling this from inside a Tokio
    /// worker (which the `Signer<IdentityPublicKey>::sign` impl below
    /// is) cannot use `blocking_read()` without panicking the runtime.
    async fn derive_private_key_bytes(
        &self,
        identity_public_key: &IdentityPublicKey,
    ) -> Result<Zeroizing<[u8; 32]>, ProtocolError> {
        let wm = self.wallet_manager.read().await;
        let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
            ProtocolError::Generic("Wallet not found in wallet manager".to_string())
        })?;
        IdentityWallet::<SpvBroadcaster>::derive_identity_key_bytes(
            wallet,
            self.network,
            self.identity_index,
            identity_public_key,
        )
        .map_err(|e| ProtocolError::Generic(e.to_string()))
    }
}

#[async_trait]
impl Signer<IdentityPublicKey> for IdentitySigner {
    async fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let private_key_bytes = self.derive_private_key_bytes(identity_public_key).await?;

        match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let signature = dashcore::signer::sign(data, private_key_bytes.as_ref())
                    .map_err(|e| ProtocolError::Generic(format!("ECDSA signing failed: {}", e)))?;
                Ok(BinaryData::new(signature.to_vec()))
            }
            #[cfg(feature = "bls")]
            KeyType::BLS12_381 => {
                use dashcore::blsful::{Bls12381G2Impl, SignatureSchemes};

                let secret_key = dashcore::blsful::SecretKey::<Bls12381G2Impl>::from_be_bytes(
                    &private_key_bytes,
                )
                .into_option()
                .ok_or_else(|| {
                    ProtocolError::Generic("BLS private key from bytes is not valid".to_string())
                })?;
                let signature = secret_key
                    .sign(SignatureSchemes::Basic, data)
                    .map_err(|e| ProtocolError::Generic(format!("BLS signing failed: {}", e)))?;
                Ok(BinaryData::new(
                    signature.as_raw_value().to_compressed().to_vec(),
                ))
            }
            #[cfg(not(feature = "bls"))]
            KeyType::BLS12_381 => Err(ProtocolError::Generic(
                "BLS signing is not enabled (missing 'bls' feature)".to_string(),
            )),
            #[cfg(feature = "eddsa")]
            KeyType::EDDSA_25519_HASH160 => {
                use dashcore::ed25519_dalek::Signer as _;

                let signing_key =
                    dashcore::ed25519_dalek::SigningKey::from_bytes(&private_key_bytes);
                let signature = signing_key.sign(data);
                Ok(BinaryData::new(signature.to_vec()))
            }
            #[cfg(not(feature = "eddsa"))]
            KeyType::EDDSA_25519_HASH160 => Err(ProtocolError::Generic(
                "EdDSA signing is not enabled (missing 'eddsa' feature)".to_string(),
            )),
            KeyType::BIP13_SCRIPT_HASH => Err(ProtocolError::Generic(
                "BIP13_SCRIPT_HASH keys are not supported for signing".to_string(),
            )),
        }
    }

    async fn sign_create_witness(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(identity_public_key, data).await?;

        match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                Ok(AddressWitness::P2pkh { signature })
            }
            _ => Err(ProtocolError::Generic(format!(
                "Key type {:?} is not supported for address witnesses",
                identity_public_key.key_type()
            ))),
        }
    }

    fn can_sign_with(&self, _identity_public_key: &IdentityPublicKey) -> bool {
        // Optimistic: any wallet-internal signer is assumed to be able
        // to derive ANY identity key it's asked about. The real
        // failure mode (watch-only wallet, missing seed, wrong
        // network) surfaces from the actual `sign` call. Cannot do a
        // real probe here — `derive_private_key_bytes` is async, and
        // this trait method is sync, and bridging via `block_on` from
        // inside a Tokio worker panics the runtime.
        true
    }
}

impl std::fmt::Debug for IdentitySigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentitySigner")
            .field("network", &self.network)
            .field("identity_index", &self.identity_index)
            .finish()
    }
}

// NOTE: The `SeedBackedIdentitySigner` and `SeedBackedPlatformAddressSigner`
// impls were removed alongside the deleted
// `platform_wallet_register_identity_from_addresses` FFI. They were the
// only seed-driven signing path in this crate, and that path is now
// served by external `SignerHandle`s (see
// `rs-platform-wallet-ffi/src/identity_registration_with_signer.rs`).
// If a future flow needs in-memory seed signing, prefer wiring it
// through the `Signer<K>` trait at the call site rather than reviving
// these wrappers — the seed should not cross the FFI boundary just so
// Rust can finish an operation (see `swift-sdk/CLAUDE.md`).
