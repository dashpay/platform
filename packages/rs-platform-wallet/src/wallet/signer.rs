//! Signer for identity operations using wallet-derived keys.

use std::sync::Arc;

use dpp::address_funds::AddressWitness;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::KeyType;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use key_wallet::bip32::{ChildNumber, DerivationPath, KeyDerivationType};
use key_wallet::dip9::{
    IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
};
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use tokio::sync::RwLock;
use zeroize::Zeroizing;

/// A signer that uses wallet-derived keys to sign identity state transitions.
pub struct IdentitySigner {
    wallet: Arc<RwLock<Wallet>>,
    network: Network,
    identity_index: u32,
}

impl IdentitySigner {
    /// Create a new IdentitySigner for a specific identity index.
    pub(crate) fn new(
        wallet: Arc<RwLock<Wallet>>,
        network: Network,
        identity_index: u32,
    ) -> Self {
        Self {
            wallet,
            network,
            identity_index,
        }
    }

    /// Get the identity index this signer is associated with.
    #[allow(dead_code)]
    pub(crate) fn identity_index(&self) -> u32 {
        self.identity_index
    }

    /// Get a reference to the wallet.
    #[allow(dead_code)]
    pub(crate) fn wallet(&self) -> &Arc<RwLock<Wallet>> {
        &self.wallet
    }

    /// Build the identity authentication derivation path for the given key type and key ID.
    ///
    /// Path format: `m/9'/coin_type'/5'/0'/key_type'/identity_index'/key_id'`
    fn derivation_path(
        &self,
        key_derivation_type: KeyDerivationType,
        key_id: u32,
    ) -> Result<DerivationPath, ProtocolError> {
        let base_path: DerivationPath = match self.network {
            Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
            _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
        }
        .into();

        let key_type_index: u32 = key_derivation_type.into();

        Ok(base_path.extend([
            ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
                ProtocolError::Generic(format!("Invalid key type index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(self.identity_index).map_err(|e| {
                ProtocolError::Generic(format!("Invalid identity index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(key_id).map_err(|e| {
                ProtocolError::Generic(format!("Invalid key ID: {}", e))
            })?,
        ]))
    }

    /// Derive the raw private key bytes for a given identity public key.
    ///
    /// Returns the bytes wrapped in [`Zeroizing`] so they are automatically
    /// wiped from memory when the value is dropped.
    ///
    /// The wallet lock is acquired and released within this method.
    fn derive_private_key_bytes(
        &self,
        identity_public_key: &IdentityPublicKey,
    ) -> Result<Zeroizing<[u8; 32]>, ProtocolError> {
        let key_id = identity_public_key.id();
        let key_derivation_type = match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => KeyDerivationType::ECDSA,
            KeyType::BLS12_381 => KeyDerivationType::BLS,
            // EdDSA uses the ECDSA derivation path; the raw bytes are reinterpreted as Ed25519 seed
            KeyType::EDDSA_25519_HASH160 => KeyDerivationType::ECDSA,
            KeyType::BIP13_SCRIPT_HASH => {
                return Err(ProtocolError::Generic(
                    "BIP13_SCRIPT_HASH keys are not supported for signing".to_string(),
                ));
            }
        };

        let path = self.derivation_path(key_derivation_type, key_id)?;

        // Acquire the wallet lock, derive the key, then drop the lock
        let wallet = self.wallet.blocking_read();
        let secret_key = wallet.derive_private_key(&path).map_err(|e| {
            ProtocolError::Generic(format!(
                "Failed to derive private key for identity key {}: {}",
                key_id, e
            ))
        })?;

        Ok(Zeroizing::new(secret_key.secret_bytes()))
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
                let signature =
                    dashcore::signer::sign(data, private_key_bytes.as_ref())
                        .map_err(|e| {
                            ProtocolError::Generic(format!("ECDSA signing failed: {}", e))
                        })?;
                Ok(BinaryData::new(signature.to_vec()))
            }
            #[cfg(feature = "bls")]
            KeyType::BLS12_381 => {
                use dashcore::blsful::{Bls12381G2Impl, SignatureSchemes};

                let secret_key =
                    dashcore::blsful::SecretKey::<Bls12381G2Impl>::from_be_bytes(
                        &*private_key_bytes,
                    )
                    .into_option()
                    .ok_or_else(|| {
                        ProtocolError::Generic(
                            "BLS private key from bytes is not valid".to_string(),
                        )
                    })?;
                let signature = secret_key.sign(SignatureSchemes::Basic, data).map_err(|e| {
                    ProtocolError::Generic(format!("BLS signing failed: {}", e))
                })?;
                Ok(BinaryData::new(
                    signature
                        .as_raw_value()
                        .to_compressed()
                        .to_vec(),
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
