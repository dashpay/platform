//! Signer for identity operations using wallet-derived keys.

use std::sync::Arc;

use dpp::address_funds::AddressWitness;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyType;
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use key_wallet::Network;
use tokio::sync::RwLock;
use zeroize::Zeroizing;

use crate::wallet::identity::wallet::IdentityWallet;
use crate::wallet::platform_wallet::PlatformWalletInfo;

/// A signer that uses wallet-derived keys to sign identity state transitions.
pub struct IdentitySigner {
    state: Arc<RwLock<PlatformWalletInfo>>,
    network: Network,
    identity_index: u32,
}

impl IdentitySigner {
    /// Create a new IdentitySigner for a specific identity index.
    pub(crate) fn new(state: Arc<RwLock<PlatformWalletInfo>>, network: Network, identity_index: u32) -> Self {
        Self {
            state,
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
    fn derive_private_key_bytes(
        &self,
        identity_public_key: &IdentityPublicKey,
    ) -> Result<Zeroizing<[u8; 32]>, ProtocolError> {
        let info_guard = self.state.blocking_read();
        IdentityWallet::derive_identity_key_bytes(
            info_guard.managed_state.wallet(),
            self.network,
            self.identity_index,
            identity_public_key,
        )
        .map_err(|e| ProtocolError::Generic(e.to_string()))
    }
}

impl Signer<IdentityPublicKey> for IdentitySigner {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let private_key_bytes = self.derive_private_key_bytes(identity_public_key)?;

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
                    &*private_key_bytes,
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
                    dashcore::ed25519_dalek::SigningKey::from_bytes(&*private_key_bytes);
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

    fn sign_create_witness(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(identity_public_key, data)?;

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

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        self.derive_private_key_bytes(identity_public_key).is_ok()
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

// ---------------------------------------------------------------------------
// ManagedIdentitySigner
// ---------------------------------------------------------------------------

use crate::wallet::identity::managed_identity::key_storage::{KeyStorage, PrivateKeyData};

/// Signer that resolves keys from a [`ManagedIdentity`]'s `key_storage`.
///
/// For [`PrivateKeyData::AtWalletDerivationPath`] keys the wallet is used to
/// derive the private key on demand. For [`PrivateKeyData::Clear`] keys the
/// stored bytes are used directly. If a key is not found in `key_storage`
/// the signer falls back to the standard DIP-9 identity authentication path
/// derivation (same logic as [`IdentitySigner`]).
pub struct ManagedIdentitySigner {
    key_storage: KeyStorage,
    state: Arc<RwLock<PlatformWalletInfo>>,
    identity_index: u32,
    network: Network,
}

impl ManagedIdentitySigner {
    /// Create a new `ManagedIdentitySigner`.
    pub fn new(
        key_storage: KeyStorage,
        state: Arc<RwLock<PlatformWalletInfo>>,
        identity_index: u32,
        network: Network,
    ) -> Self {
        Self {
            key_storage,
            state,
            identity_index,
            network,
        }
    }

    /// Derive private key bytes for a given identity public key.
    ///
    /// 1. If the key is in `key_storage` with `Clear` data, return those bytes.
    /// 2. If the key is in `key_storage` with `AtWalletDerivationPath`, derive
    ///    from the wallet at that path.
    /// 3. Otherwise fall back to the standard DIP-9 identity authentication
    ///    path derivation via [`IdentityWallet::derive_identity_key_bytes`].
    fn derive_private_key_bytes(
        &self,
        identity_public_key: &IdentityPublicKey,
    ) -> Result<Zeroizing<[u8; 32]>, ProtocolError> {
        let key_id = identity_public_key.id();

        // Check key_storage first.
        if let Some((_pub_key, private_key_data)) = self.key_storage.get(&key_id) {
            return match private_key_data {
                PrivateKeyData::Clear(bytes) => Ok(bytes.clone()),
                PrivateKeyData::AtWalletDerivationPath {
                    derivation_path, ..
                } => {
                    let info_guard = self.state.blocking_read();
                    let secret_key = info_guard.managed_state.wallet().derive_private_key(derivation_path).map_err(|e| {
                        ProtocolError::Generic(format!(
                            "Failed to derive private key for identity key {}: {}",
                            key_id, e
                        ))
                    })?;
                    Ok(Zeroizing::new(secret_key.secret_bytes()))
                }
            };
        }

        // Fallback: standard DIP-9 derivation from identity_index + key_id.
        let info_guard = self.state.blocking_read();
        IdentityWallet::derive_identity_key_bytes(
            info_guard.managed_state.wallet(),
            self.network,
            self.identity_index,
            identity_public_key,
        )
        .map_err(|e| ProtocolError::Generic(e.to_string()))
    }
}

impl Signer<IdentityPublicKey> for ManagedIdentitySigner {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let private_key_bytes = self.derive_private_key_bytes(identity_public_key)?;

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
                    &*private_key_bytes,
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
                    dashcore::ed25519_dalek::SigningKey::from_bytes(&*private_key_bytes);
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

    fn sign_create_witness(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(identity_public_key, data)?;

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

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        self.derive_private_key_bytes(identity_public_key).is_ok()
    }
}

impl std::fmt::Debug for ManagedIdentitySigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedIdentitySigner")
            .field("network", &self.network)
            .field("identity_index", &self.identity_index)
            .field(
                "key_storage_keys",
                &self.key_storage.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}
