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
    fn derive_private_key_bytes(
        &self,
        identity_public_key: &IdentityPublicKey,
    ) -> Result<Zeroizing<[u8; 32]>, ProtocolError> {
        let wm = self.wallet_manager.blocking_read();
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

/// Signer for a managed identity that derives private keys on demand
/// from the wallet at the DIP-9 identity authentication path.
///
/// `ManagedIdentity` no longer carries a `KeyStorage` field — private
/// keys live in the iOS Keychain on the client side, and the Rust side
/// derives them on demand via the wallet seed using the DIP-9 path
/// `m/9'/coin'/5'/0'/ECDSA'/identity_index'/key_index'`. This is
/// effectively the same shape as [`IdentitySigner`]; kept as a separate
/// type for the existing call sites that take it as the signer impl.
pub struct ManagedIdentitySigner {
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: WalletId,
    identity_index: u32,
    network: Network,
}

impl ManagedIdentitySigner {
    /// Create a new `ManagedIdentitySigner`.
    pub fn new(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        identity_index: u32,
        network: Network,
    ) -> Self {
        Self {
            wallet_manager,
            wallet_id,
            identity_index,
            network,
        }
    }

    /// Derive private key bytes for a given identity public key by
    /// re-deriving from the wallet seed at the DIP-9 path.
    fn derive_private_key_bytes(
        &self,
        identity_public_key: &IdentityPublicKey,
    ) -> Result<Zeroizing<[u8; 32]>, ProtocolError> {
        let wm = self.wallet_manager.blocking_read();
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
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Seed-backed signers (no wallet key material required)
// ---------------------------------------------------------------------------
//
// These signers derive private keys from a BIP-39 seed held in memory for
// the duration of a single operation (typically `register_from_addresses`).
// They exist so that watch-only / external-signable wallets — which carry
// no key material in `Wallet::wallet_type` — can still drive flows that
// need actual signatures. The seed is the caller's responsibility: it
// comes from iOS Keychain, travels through FFI inside a `Zeroizing`
// buffer, and is dropped as soon as the operation completes.

use dashcore::secp256k1::Secp256k1;
use dpp::address_funds::PlatformAddress;
use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, KeyDerivationType};
use key_wallet::dip9::{
    IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
};
use key_wallet::PlatformP2PKHAddress;

use crate::wallet::platform_addresses::PlatformAddressWallet;

/// Build the DIP-9 identity authentication path
/// `m/9'/COIN'/5'/0'/ECDSA'/identity_index'/key_index'`.
fn dip9_identity_auth_path(
    network: Network,
    identity_index: u32,
    key_index: u32,
) -> Result<DerivationPath, ProtocolError> {
    let base = match network {
        Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
        _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
    };
    let key_type_index: u32 = KeyDerivationType::ECDSA.into();

    Ok(DerivationPath::from(base).extend([
        ChildNumber::from_hardened_idx(key_type_index)
            .map_err(|e| ProtocolError::Generic(format!("Invalid key type index: {}", e)))?,
        ChildNumber::from_hardened_idx(identity_index)
            .map_err(|e| ProtocolError::Generic(format!("Invalid identity index: {}", e)))?,
        ChildNumber::from_hardened_idx(key_index)
            .map_err(|e| ProtocolError::Generic(format!("Invalid key index: {}", e)))?,
    ]))
}

/// Derive a 32-byte ECDSA private key at `path` from a BIP-39 seed.
fn derive_ecdsa_bytes_from_seed(
    seed: &[u8],
    network: Network,
    path: &DerivationPath,
) -> Result<Zeroizing<[u8; 32]>, ProtocolError> {
    let master = ExtendedPrivKey::new_master(network, seed)
        .map_err(|e| ProtocolError::Generic(format!("Failed to derive master key: {}", e)))?;
    let secp = Secp256k1::new();
    let derived = master
        .derive_priv(&secp, path)
        .map_err(|e| ProtocolError::Generic(format!("Failed to derive key at path: {}", e)))?;
    Ok(Zeroizing::new(derived.private_key.secret_bytes()))
}

/// Sign `data` with an ECDSA secp256k1 private key and return a
/// `BinaryData`-wrapped compact signature (the form DPP expects).
fn sign_ecdsa_with_bytes(
    data: &[u8],
    secret_bytes: &[u8; 32],
) -> Result<BinaryData, ProtocolError> {
    let signature = dashcore::signer::sign(data, secret_bytes)
        .map_err(|e| ProtocolError::Generic(format!("ECDSA signing failed: {}", e)))?;
    Ok(BinaryData::new(signature.to_vec()))
}

/// `Signer<IdentityPublicKey>` impl backed by a BIP-39 seed.
///
/// Derives the DIP-9 identity authentication key on every `sign` call
/// (master key HMAC is ~microseconds). Drop the signer as soon as the
/// operation completes so the seed is scrubbed.
pub struct SeedBackedIdentitySigner {
    seed: Zeroizing<Vec<u8>>,
    network: Network,
    identity_index: u32,
}

impl SeedBackedIdentitySigner {
    /// Construct from an already-computed BIP-39 seed (typically 64
    /// bytes from `Mnemonic::to_seed`). The seed is cloned into a
    /// `Zeroizing` buffer owned by the signer.
    pub fn new(seed: &[u8], network: Network, identity_index: u32) -> Self {
        Self {
            seed: Zeroizing::new(seed.to_vec()),
            network,
            identity_index,
        }
    }
}

impl Signer<IdentityPublicKey> for SeedBackedIdentitySigner {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        // Identity auth keys only — the DIP-9 path is keyed by the
        // `IdentityPublicKey.id` (KeyID == key_index on our tree).
        match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let path = dip9_identity_auth_path(
                    self.network,
                    self.identity_index,
                    identity_public_key.id(),
                )?;
                let secret = derive_ecdsa_bytes_from_seed(&self.seed, self.network, &path)?;
                sign_ecdsa_with_bytes(data, &secret)
            }
            other => Err(ProtocolError::Generic(format!(
                "Seed-backed signer does not support key type {:?}",
                other
            ))),
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
            other => Err(ProtocolError::Generic(format!(
                "Key type {:?} is not supported for address witnesses",
                other
            ))),
        }
    }

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        matches!(
            identity_public_key.key_type(),
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160
        )
    }
}

impl std::fmt::Debug for SeedBackedIdentitySigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeedBackedIdentitySigner")
            .field("network", &self.network)
            .field("identity_index", &self.identity_index)
            .finish()
    }
}

/// `Signer<PlatformAddress>` impl backed by a BIP-39 seed.
///
/// Looks up the DIP-17 derivation path for the P2PKH hash on the
/// wrapped [`PlatformAddressWallet`] (the pool still knows which path
/// produced which address), then derives the private key from the
/// seed using that path.
///
/// The wallet is held by value — [`PlatformAddressWallet`] is
/// `Clone` and all its internal state lives behind `Arc`s, so this
/// is a cheap refcount bump that frees the signer from any
/// lifetime tied to the caller's stack. That in turn lets it be
/// captured by a `'static + Send` future (e.g. one handed to
/// `tokio::spawn`).
pub struct SeedBackedPlatformAddressSigner {
    seed: Zeroizing<Vec<u8>>,
    network: Network,
    wallet: PlatformAddressWallet,
}

impl SeedBackedPlatformAddressSigner {
    pub fn new(seed: &[u8], network: Network, wallet: PlatformAddressWallet) -> Self {
        Self {
            seed: Zeroizing::new(seed.to_vec()),
            network,
            wallet,
        }
    }

    /// Synchronously look up the DIP-17 path for a P2PKH platform
    /// address by scanning all platform-payment accounts in the
    /// wallet manager. Mirrors the path-lookup portion of
    /// [`PlatformAddressWallet::find_private_key_for_platform_address`]
    /// but stops before the privkey-derivation step.
    fn path_for(&self, p2pkh: &PlatformP2PKHAddress) -> Result<DerivationPath, ProtocolError> {
        let dashcore_addr = p2pkh.to_address(self.wallet.sdk.network);
        let handle = tokio::runtime::Handle::current();
        let found = tokio::task::block_in_place(|| {
            handle.block_on(async {
                let wm = self.wallet.wallet_manager.read().await;
                // `PlatformWalletInfo` is not `Clone`, so resolve the
                // path while the read guard is still held and copy
                // out only the `DerivationPath`.
                wm.get_wallet_info(&self.wallet.wallet_id).and_then(|info| {
                    info.core_wallet
                        .accounts
                        .platform_payment_accounts
                        .values()
                        .find_map(|account| {
                            account
                                .addresses
                                .address_info(&dashcore_addr)
                                .map(|ai| ai.path.clone())
                        })
                })
            })
        });
        found.ok_or_else(|| {
            ProtocolError::Generic(format!(
                "Platform address {} not found in wallet's address pools",
                p2pkh
            ))
        })
    }
}

impl Signer<PlatformAddress> for SeedBackedPlatformAddressSigner {
    fn sign(
        &self,
        platform_address: &PlatformAddress,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let PlatformAddress::P2pkh(hash) = platform_address else {
            return Err(ProtocolError::Generic(
                "Only P2PKH Platform addresses are supported for signing".to_string(),
            ));
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);
        let path = self.path_for(&p2pkh)?;
        let secret = derive_ecdsa_bytes_from_seed(&self.seed, self.network, &path)?;
        sign_ecdsa_with_bytes(data, &secret)
    }

    fn sign_create_witness(
        &self,
        platform_address: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let signature = self.sign(platform_address, data)?;
        Ok(AddressWitness::P2pkh { signature })
    }

    fn can_sign_with(&self, platform_address: &PlatformAddress) -> bool {
        let PlatformAddress::P2pkh(hash) = platform_address else {
            return false;
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);
        self.path_for(&p2pkh).is_ok()
    }
}

impl std::fmt::Debug for SeedBackedPlatformAddressSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeedBackedPlatformAddressSigner")
            .field("network", &self.network)
            .field("wallet_id", &hex::encode(self.wallet.wallet_id))
            .finish()
    }
}
