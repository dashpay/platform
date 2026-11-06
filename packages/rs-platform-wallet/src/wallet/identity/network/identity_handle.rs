//! Identity wallet for managing Platform identities + DashPay state.
//!
//! Provides methods for the full identity lifecycle: registration, discovery
//! (gap-limit scan), top-up, withdrawal, credit transfer — **plus** the
//! DashPay-contract operations that live on the same identity (contact
//! requests, contacts, payments, profile, account labels).
//!
//! Historically DashPay lived on a separate `DashPayWallet<B>` facade, but
//! both views operated on the same underlying state (a single
//! [`ManagedIdentity`](crate::wallet::identity::ManagedIdentity) carries
//! both identity fields and DashPay fields). The two facades were merged to
//! cut handle-juggling at the FFI boundary and to give the DashPay
//! operations access to the same asset-lock / signer plumbing the identity
//! lifecycle already uses.
//!
//! The `B` generic parameter picks the transaction broadcaster used by
//! DashPay payment operations (`send_payment`). It defaults to
//! [`SpvBroadcaster`] — the only production broadcaster — so the majority
//! of call sites don't need to name it. Asset-lock funding still pins to
//! `SpvBroadcaster` because the [`AssetLockManager`] itself is pinned; that
//! invariant lives in `PlatformWallet::new`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex as StdMutex};

use dashcore::crypto::ecdsa::EcdsaPublicKey as PublicKey;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::{IdentityPublicKey, KeyType, SecurityLevel};
use dpp::prelude::Identifier;
use key_wallet::bip32::{
    ApplicationKeyPurpose, ChildNumber, DerivationPath, Error as Bip32Error, ExtendedPrivKey,
    KeyDerivationType,
};
use key_wallet::dip9::{
    FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_APPLICATION_ENCRYPTION,
    FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_APPLICATION_SESSION_AUTHENTICATION,
    IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
};
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;
use zeroize::{Zeroize, Zeroizing};

use crate::broadcaster::{SpvBroadcaster, TransactionBroadcaster};
use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::manager::AssetLockManager;
use crate::wallet::platform_wallet::{PlatformWallet, PlatformWalletInfo, WalletId};

/// Default gap limit for identity discovery scanning.
///
/// Promoted to `pub` so external consumers (in particular the FFI
/// crate's identity-registration-key preview path) can align their
/// preview slot count with the scan window `discover()` walks. Keep
/// this and [`MASTER_KEY_INDEX`] as the single source of truth for
/// the discovery-scan policy.
pub const IDENTITY_GAP_LIMIT: u32 = 5;

/// Identity-authentication key index that the discovery scan probes
/// and that every identity this crate registers uses as its MASTER
/// auth key. Public so preview helpers can align on the same slot.
pub const MASTER_KEY_INDEX: u32 = 0;

/// Build the DIP-9 identity-authentication derivation path for the
/// given `(identity_index, key_index)` on `network`.
///
/// Path format: `m/9'/COIN_TYPE'/5'/0'/ECDSA'/identity_index'/key_index'`
/// where `COIN_TYPE'` picks mainnet vs testnet. ECDSA is hardcoded —
/// this is the only derivation type the discovery scan probes today.
pub(crate) fn identity_auth_derivation_path(
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<DerivationPath, PlatformWalletError> {
    identity_auth_derivation_path_for_type(
        network,
        KeyDerivationType::ECDSA,
        identity_index,
        key_index,
    )
}

/// The DIP-9 derivation the wallet derives an identity key of
/// `key_type` with, at the identity's authentication key slots, or
/// `None` for a key type the wallet does not derive. A key of such a
/// type has no slot key to check it against or to re-derive it from.
///
/// [`IdentityWallet::derive_identity_key_bytes`] derives by this
/// answer and [`check_identity_key_from_master`] checks by it, both
/// through [`identity_key_derivation_path`].
pub fn identity_key_derivation_type(key_type: KeyType) -> Option<KeyDerivationType> {
    match key_type {
        KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => Some(KeyDerivationType::ECDSA),
        KeyType::BLS12_381 => Some(KeyDerivationType::BLS),
        // EdDSA uses the ECDSA derivation path; the raw bytes are
        // reinterpreted as an Ed25519 seed.
        KeyType::EDDSA_25519_HASH160 => Some(KeyDerivationType::ECDSA),
        // A script hash has no private key to derive.
        KeyType::BIP13_SCRIPT_HASH => None,
        // A Hecate signer key's derivation path is not settled yet.
        KeyType::REDPALLAS_HECATE => None,
    }
}

/// Build the DIP-9 identity-authentication derivation path for the
/// given `(key_derivation_type, identity_index, key_index)` on
/// `network`. Generalizes the ECDSA-hardcoded
/// [`identity_auth_derivation_path`] so callers building keys for
/// different key types (BLS, EdDSA) can reach the right slot.
///
/// Path format:
/// `m/9'/COIN_TYPE'/5'/0'/key_derivation_type'/identity_index'/key_index'`
///
/// Promoted to `pub` so the FFI crate's mnemonic-driven derivation
/// path can call the library version instead of duplicating the
/// path-building logic.
pub fn identity_auth_derivation_path_for_type(
    network: key_wallet::Network,
    key_derivation_type: KeyDerivationType,
    identity_index: u32,
    key_index: u32,
) -> Result<DerivationPath, PlatformWalletError> {
    let base_path: DerivationPath = match network {
        key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
        _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
    }
    .into();

    let key_type_index: u32 = key_derivation_type.into();

    Ok(base_path.extend([
        ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid key type index: {}", e))
        })?,
        ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid identity index: {}", e))
        })?,
        ChildNumber::from_hardened_idx(key_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid key index: {}", e))
        })?,
    ]))
}

/// The DIP-9 path the wallet derives `identity_public_key` at: the
/// identity's authentication key slot for the key's id, under the
/// derivation of the key's type ([`identity_key_derivation_type`]).
/// `None` for a key type the wallet does not derive.
///
/// Signing ([`IdentityWallet::derive_identity_key_bytes`]) and the key
/// check ([`check_identity_key_from_master`]) both pick the slot here,
/// so they cannot disagree on which key belongs to an identity key.
pub fn identity_key_derivation_path(
    network: Network,
    identity_index: u32,
    identity_public_key: &IdentityPublicKey,
) -> Result<Option<DerivationPath>, PlatformWalletError> {
    let Some(key_derivation_type) = identity_key_derivation_type(identity_public_key.key_type())
    else {
        return Ok(None);
    };
    identity_auth_derivation_path_for_type(
        network,
        key_derivation_type,
        identity_index,
        identity_public_key.id(),
    )
    .map(Some)
}

/// A DashPay Connect key: the DIP-13 application sub-features
/// (dashpay/dips#191), whose paths key-wallet builds. Identity, request and
/// contract ids are DIP-14 256-bit children, so two devices restored from
/// one seed derive the same key without coordination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectKey {
    /// `m/9'/coin'/5'/6'/0'/identity_id'/request_id'`; `request_id` is
    /// `hash256` of the app's ephemeral public key.
    SessionAuthentication {
        identity_id: Identifier,
        request_id: [u8; 32],
    },
    /// `m/9'/coin'/5'/7'/0'/identity_id'/contract_id'/purpose'/rotation'`:
    /// one half of the encryption pair an identity holds for a contract.
    /// `rotation` is a 31-bit hardened index, `0` for the first pair; each
    /// rotation is a fresh pair. Derive new keys with this variant.
    AppEncryption {
        identity_id: Identifier,
        contract_id: [u8; 32],
        purpose: ApplicationKeyPurpose,
        rotation: u32,
    },
    /// The pre-rotation layout
    /// `m/9'/coin'/5'/7'/0'/identity_id'/contract_id'/purpose'`, without the
    /// trailing `rotation'` level. Only for recognising keys registered on
    /// testnet/devnet before DIP-13 gained that level; never derive new keys
    /// here.
    AppEncryptionLegacy {
        identity_id: Identifier,
        contract_id: [u8; 32],
        purpose: ApplicationKeyPurpose,
    },
}

impl ConnectKey {
    /// Build a key from the raw DIP-13 levels a foreign caller (FFI, JNI)
    /// passes, validating the combination:
    ///
    /// - `sub_feature` `6` with `purpose` `0` and `rotation` `0`:
    ///   [`SessionAuthentication`](Self::SessionAuthentication), `leaf` is the
    ///   request id (the path has no purpose or rotation level);
    /// - `sub_feature` `7` with `purpose` `1` (ENCRYPTION) or `2`
    ///   (DECRYPTION) and a 31-bit `rotation`:
    ///   [`AppEncryption`](Self::AppEncryption), `leaf` is the contract id.
    ///
    /// Anything else is [`PlatformWalletError::InvalidParameter`]. The
    /// pre-rotation layout is never selected here; it has its own explicit
    /// constructor, [`legacy_app_encryption_from_parts`](Self::legacy_app_encryption_from_parts).
    pub fn from_parts(
        sub_feature: u32,
        identity_id: Identifier,
        leaf: [u8; 32],
        purpose: u32,
        rotation: u32,
    ) -> Result<Self, PlatformWalletError> {
        match (sub_feature, purpose, rotation) {
            (FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_APPLICATION_SESSION_AUTHENTICATION, 0, 0) => {
                Ok(ConnectKey::SessionAuthentication {
                    identity_id,
                    request_id: leaf,
                })
            }
            (FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_APPLICATION_ENCRYPTION, purpose, rotation) => {
                let purpose = app_encryption_purpose(purpose)?;
                hardened_rotation(rotation)?;
                Ok(ConnectKey::AppEncryption {
                    identity_id,
                    contract_id: leaf,
                    purpose,
                    rotation,
                })
            }
            (sub_feature, purpose, rotation) => {
                Err(PlatformWalletError::InvalidParameter(format!(
                "sub_feature {sub_feature} with purpose {purpose} and rotation {rotation} is not \
                 a DashPay Connect key (6 with purpose 0 and rotation 0, or 7 with purpose 1 \
                 or 2)"
            )))
            }
        }
    }

    /// Build a pre-rotation [`AppEncryptionLegacy`](Self::AppEncryptionLegacy)
    /// key from a raw `purpose` (`1` ENCRYPTION or `2` DECRYPTION); anything
    /// else is [`PlatformWalletError::InvalidParameter`].
    pub fn legacy_app_encryption_from_parts(
        identity_id: Identifier,
        contract_id: [u8; 32],
        purpose: u32,
    ) -> Result<Self, PlatformWalletError> {
        Ok(ConnectKey::AppEncryptionLegacy {
            identity_id,
            contract_id,
            purpose: app_encryption_purpose(purpose)?,
        })
    }

    /// The key's derivation path on `network`. Fails with
    /// [`PlatformWalletError::InvalidParameter`] when an
    /// [`AppEncryption`](Self::AppEncryption) rotation does not fit a
    /// hardened index (`rotation >= 2^31`).
    pub fn derivation_path(
        &self,
        network: key_wallet::Network,
    ) -> Result<DerivationPath, PlatformWalletError> {
        match *self {
            ConnectKey::SessionAuthentication {
                identity_id,
                request_id,
            } => Ok(DerivationPath::application_session_authentication_path(
                network,
                identity_id.to_buffer(),
                request_id,
            )),
            ConnectKey::AppEncryption {
                identity_id,
                contract_id,
                purpose,
                rotation,
            } => DerivationPath::application_encryption_path(
                network,
                identity_id.to_buffer(),
                contract_id,
                purpose,
                rotation,
            )
            .map_err(|e| invalid_rotation(rotation, e)),
            ConnectKey::AppEncryptionLegacy {
                identity_id,
                contract_id,
                purpose,
            } => Ok(
                DerivationPath::application_encryption_path_without_rotation(
                    network,
                    identity_id.to_buffer(),
                    contract_id,
                    purpose,
                ),
            ),
        }
    }
}

/// The app encryption half a raw DIP-13 `purpose` level names.
fn app_encryption_purpose(purpose: u32) -> Result<ApplicationKeyPurpose, PlatformWalletError> {
    match purpose {
        p if p == ApplicationKeyPurpose::Encryption as u32 => Ok(ApplicationKeyPurpose::Encryption),
        p if p == ApplicationKeyPurpose::Decryption as u32 => Ok(ApplicationKeyPurpose::Decryption),
        purpose => Err(PlatformWalletError::InvalidParameter(format!(
            "purpose {purpose} is not an app encryption purpose (1 ENCRYPTION or 2 DECRYPTION)"
        ))),
    }
}

/// The hardened `rotation'` level; a rotation of `2^31` or more does not fit
/// one and is refused rather than wrapped onto another key.
fn hardened_rotation(rotation: u32) -> Result<ChildNumber, PlatformWalletError> {
    ChildNumber::from_hardened_idx(rotation).map_err(|e| invalid_rotation(rotation, e))
}

fn invalid_rotation(rotation: u32, e: Bip32Error) -> PlatformWalletError {
    PlatformWalletError::InvalidParameter(format!(
        "Invalid app encryption rotation {rotation}: {e}"
    ))
}

/// Derive the ECDSA secp256k1 keypair of `key` from a master xpriv. Pure,
/// like [`derive_ecdsa_identity_auth_keypair_from_master`], so it works for
/// watch-only wallets whose seed the FFI resolves on demand.
pub fn derive_connect_keypair_from_master(
    master: &ExtendedPrivKey,
    network: key_wallet::Network,
    key: ConnectKey,
) -> Result<DerivedIdentityAuthKey, PlatformWalletError> {
    use key_wallet::bip32::ExtendedPubKey;

    let path = key.derivation_path(network)?;
    // See `derive_ecdsa_identity_auth_keypair_from_master` for why the
    // intermediate `ExtendedPrivKey` needs no explicit wipe.
    let derived = master.derive_priv(&path).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!("Failed to derive connect key: {e}"))
    })?;
    let extended_pub = ExtendedPubKey::from_priv(&derived);

    Ok(DerivedIdentityAuthKey {
        derivation_path: path,
        private_key: derived.private_key.to_bytes(),
        public_key: extended_pub.public_key.to_compressed(),
    })
}

/// One ECDSA identity-authentication keypair derived from a master
/// xpriv at a specific `(identity_index, key_index)` slot. Wraps
/// the secret scalar in [`Zeroizing`] so it is wiped on drop —
/// callers that ship the bytes elsewhere (FFI, Keychain) should
/// copy what they need and let this struct fall out of scope to
/// reclaim the secret memory.
pub struct DerivedIdentityAuthKey {
    /// Full DIP-9 path used (so callers can persist the breadcrumb).
    pub derivation_path: DerivationPath,
    /// 32-byte secp256k1 secret scalar.
    pub private_key: Zeroizing<[u8; 32]>,
    /// 33-byte compressed secp256k1 public key.
    pub public_key: [u8; 33],
}

/// Derive a single ECDSA identity-authentication keypair from a
/// master xpriv at `(identity_index, key_index)`. Pure function —
/// no `Wallet` required, so this works for watch-only wallets where
/// the seed is held outside the in-memory wallet manager.
///
/// Used by the FFI's mnemonic-driven derivation paths
/// (`platform_wallet_derive_identity_key_at_slot`,
/// `dash_sdk_derive_identity_keys_from_mnemonic`) so the path
/// builder + secp256k1 derive pass aren't duplicated in the FFI
/// crate. The mnemonic-to-master step still lives in the FFI
/// because mnemonic parsing pulls `key_wallet::mnemonic`, which
/// the library doesn't otherwise need.
pub fn derive_ecdsa_identity_auth_keypair_from_master(
    master: &ExtendedPrivKey,
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<DerivedIdentityAuthKey, PlatformWalletError> {
    use key_wallet::bip32::ExtendedPubKey;

    let path = identity_auth_derivation_path_for_type(
        network,
        KeyDerivationType::ECDSA,
        identity_index,
        key_index,
    )?;
    // The derived scalar and its exported byte copy zeroize on drop.
    let derived = master.derive_priv(&path).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to derive private key at (identity={identity_index}, key={key_index}): {e}"
        ))
    })?;
    let extended_pub = ExtendedPubKey::from_priv(&derived);

    Ok(DerivedIdentityAuthKey {
        derivation_path: path,
        private_key: derived.private_key.to_bytes(),
        public_key: extended_pub.public_key.to_compressed(),
    })
}

/// Derive the DIP-9 identity-authentication keypair at
/// `(identity_index, key_index)` on `network`.
///
/// Returns the full derivation path, the extended private key (the
/// caller can wrap it in [`Zeroizing`] if the raw scalar will escape
/// the crate), and the compressed `secp256k1::PublicKey` matching
/// that private key. Kept in sync with the discovery scan and the
/// FFI-exposed preview so both code paths probe / surface identical
/// key material.
pub fn derive_identity_auth_keypair(
    wallet: &Wallet,
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<(DerivationPath, ExtendedPrivKey, PublicKey), PlatformWalletError> {
    use key_wallet::bip32::ExtendedPubKey;

    let full_path = identity_auth_derivation_path(network, identity_index, key_index)?;
    let auth_key = wallet
        .derive_extended_private_key(&full_path)
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive authentication key: {}",
                e
            ))
        })?;

    let extended_pub = ExtendedPubKey::from_priv(&auth_key);
    Ok((full_path, auth_key, extended_pub.public_key))
}

/// Derive the 20-byte RIPEMD160(SHA256) hash of the public key at the given
/// identity authentication path.
///
/// Path format: `base_path / key_type' / identity_index' / key_index'`
/// where `base_path` is `m/9'/COIN_TYPE'/5'/0'` (mainnet or testnet).
///
/// Thin wrapper over [`derive_identity_auth_keypair`] — shares the
/// path-building + derivation so the FFI-side preview and the live
/// scan can never drift from one another.
///
/// Requires a [`Wallet`] with resident private key material. For
/// wallets whose seed lives outside the in-process wallet manager
/// (`WalletType::ExternalSignable` — the iOS Keychain-backed shape),
/// this errors with `External signable wallet has no private key`; use
/// [`derive_identity_auth_key_hash_from_master`] instead, fed by a
/// master xpriv the caller resolved from the mnemonic on demand.
pub(crate) fn derive_identity_auth_key_hash(
    wallet: &Wallet,
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<[u8; 20], PlatformWalletError> {
    use dpp::util::hash::ripemd160_sha256;

    let (_path, _priv, public_key) =
        derive_identity_auth_keypair(wallet, network, identity_index, key_index)?;
    let public_key_bytes = public_key.to_compressed();
    let key_hash = ripemd160_sha256(&public_key_bytes);

    let mut key_hash_array = [0u8; 20];
    key_hash_array.copy_from_slice(&key_hash);

    Ok(key_hash_array)
}

/// Master-xpriv sibling of [`derive_identity_auth_key_hash`]: derive the
/// 20-byte RIPEMD160(SHA256) pubkey hash for the identity-authentication
/// slot `(identity_index, key_index)` directly from a master
/// `ExtendedPrivKey` instead of from an in-memory [`Wallet`].
///
/// Pure function — no `Wallet` required — so it works for the
/// `WalletType::ExternalSignable` shape where the seed lives outside the
/// in-process wallet manager (iOS Keychain). The caller resolves the
/// mnemonic into a master xpriv on demand (see the FFI resolver path)
/// and hands it in here.
///
/// Goes through [`derive_ecdsa_identity_auth_keypair_from_master`] so the
/// rescan scan, the registration derive path, and the in-creation key #0
/// can never drift on the path shape / secp256k1 derive: it derives the
/// same compressed pubkey the wallet-internal
/// [`derive_identity_auth_key_hash`] would for a key-resident wallet at
/// the same slot, then `ripemd160_sha256`-hashes it identically.
pub fn derive_identity_auth_key_hash_from_master(
    master: &ExtendedPrivKey,
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<[u8; 20], PlatformWalletError> {
    use dpp::util::hash::ripemd160_sha256;

    let derived =
        derive_ecdsa_identity_auth_keypair_from_master(master, network, identity_index, key_index)?;
    let key_hash = ripemd160_sha256(&derived.public_key);

    let mut key_hash_array = [0u8; 20];
    key_hash_array.copy_from_slice(&key_hash);

    Ok(key_hash_array)
}

/// Whether an identity key is the key the wallet derives at the key's
/// slot, from [`check_identity_key_from_master`].
pub enum IdentityKeyWalletCheck {
    /// The wallet derives no key at the key's slot, so there is nothing
    /// to compare the key with: it does not derive keys of this type at
    /// identity key slots, or the key's id is past the last hardened
    /// index, which no slot has.
    NotDerived,
    /// The key is in an encoding the check does not compare (an
    /// ECDSA_SECP256K1 key that is not 33 bytes), so it can't say
    /// whether the key is the wallet's.
    NotCompared,
    /// The wallet's key at the slot is this key.
    Matches {
        /// The slot's full DIP-9 path.
        derivation_path: DerivationPath,
        /// The key's 32-byte private key, wiped on drop.
        private_key: Zeroizing<[u8; 32]>,
        /// The key's public key hash (DPP `public_key_hash`).
        public_key_hash: [u8; 20],
    },
    /// The wallet derives a different key at the slot.
    Mismatch {
        /// The slot's full DIP-9 path.
        derivation_path: DerivationPath,
        /// The wallet's key at the slot in the key type's on-chain form.
        derived_public_key_data: Vec<u8>,
    },
}

// Manual `Debug` so the private key never reaches a log.
impl std::fmt::Debug for IdentityKeyWalletCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentityKeyWalletCheck::NotDerived => f.write_str("NotDerived"),
            IdentityKeyWalletCheck::NotCompared => f.write_str("NotCompared"),
            IdentityKeyWalletCheck::Matches {
                derivation_path,
                public_key_hash,
                ..
            } => f
                .debug_struct("Matches")
                .field("derivation_path", derivation_path)
                .field("public_key_hash", &hex::encode(public_key_hash))
                .finish_non_exhaustive(),
            IdentityKeyWalletCheck::Mismatch {
                derivation_path,
                derived_public_key_data,
            } => f
                .debug_struct("Mismatch")
                .field("derivation_path", derivation_path)
                .field(
                    "derived_public_key_data",
                    &hex::encode(derived_public_key_data),
                )
                .finish(),
        }
    }
}

/// What checking a key needs: nothing, when the outcome is known
/// without the seed, or the wallet's key at a slot.
enum KeyComparison {
    Known(IdentityKeyWalletCheck),
    AtSlot(DerivationPath),
}

fn key_comparison(
    network: Network,
    identity_index: u32,
    identity_public_key: &IdentityPublicKey,
) -> Result<KeyComparison, PlatformWalletError> {
    // The wallet derives at hardened slots only, so a key whose id is past
    // the last hardened index is at none of them. Such a key, enabled or
    // not, must not fail the check of the identity's other keys.
    if ChildNumber::from_hardened_idx(identity_public_key.id()).is_err() {
        return Ok(KeyComparison::Known(IdentityKeyWalletCheck::NotDerived));
    }
    let Some(derivation_path) =
        identity_key_derivation_path(network, identity_index, identity_public_key)?
    else {
        return Ok(KeyComparison::Known(IdentityKeyWalletCheck::NotDerived));
    };
    // DPP compares an ECDSA_SECP256K1 key with the derived key's
    // compressed 33-byte form only, and discovery and signing treat any
    // other encoding as not wallet-derived, so the check can't say
    // whether such a key is the wallet's.
    if identity_public_key.key_type() == KeyType::ECDSA_SECP256K1
        && identity_public_key.data().len() != KeyType::ECDSA_SECP256K1.default_size()
    {
        return Ok(KeyComparison::Known(IdentityKeyWalletCheck::NotCompared));
    }
    Ok(KeyComparison::AtSlot(derivation_path))
}

/// Check `identity_public_key` against the key the wallet derives from
/// `master` at the key's slot ([`identity_key_derivation_path`]): the
/// same path and bytes [`IdentityWallet::derive_identity_key_bytes`]
/// signs with, compared by DPP's `validate_private_key_bytes`. Pure,
/// like [`derive_ecdsa_identity_auth_keypair_from_master`], so it works
/// for wallets whose seed the FFI resolves on demand.
pub fn check_identity_key_from_master(
    master: &ExtendedPrivKey,
    network: Network,
    identity_index: u32,
    identity_public_key: &IdentityPublicKey,
) -> Result<IdentityKeyWalletCheck, PlatformWalletError> {
    let derivation_path = match key_comparison(network, identity_index, identity_public_key)? {
        KeyComparison::Known(check) => return Ok(check),
        KeyComparison::AtSlot(derivation_path) => derivation_path,
    };
    let key_id = identity_public_key.id();
    // The derived scalar and its exported byte copy zeroize on drop.
    let private_key = master
        .derive_priv(&derivation_path)
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive private key for identity key {key_id}: {e}"
            ))
        })?
        .private_key
        .to_bytes();

    let is_match = identity_public_key
        .validate_private_key_bytes(&private_key, network)
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to compare identity key {key_id} with the wallet's key: {e}"
            ))
        })?;
    if is_match {
        let public_key_hash = identity_public_key.public_key_hash().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to hash identity key {key_id}: {e}"
            ))
        })?;
        return Ok(IdentityKeyWalletCheck::Matches {
            derivation_path,
            private_key,
            public_key_hash,
        });
    }
    let derived_public_key_data = identity_public_key
        .key_type()
        .public_key_data_from_private_key_data(&private_key, network)
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to put the wallet's key for identity key {key_id} in on-chain form: {e}"
            ))
        })?;
    Ok(IdentityKeyWalletCheck::Mismatch {
        derivation_path,
        derived_public_key_data,
    })
}

/// Whether an identity is this wallet's, from its enabled keys. The
/// enabled master key decides; disabled keys never count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityWalletOwnership {
    /// An enabled master key is the wallet's key at its slot.
    Owned,
    /// The identity has an enabled master key, every enabled master key
    /// is another key than the wallet's at its slot, and no enabled key
    /// is or could be the wallet's. The identity is not this wallet's,
    /// so a client may offer to remove it.
    NotOwned,
    /// Neither: no enabled master key, an enabled master key the check
    /// can't compare, or a foreign master key next to a key that is or
    /// could be the wallet's (an enabled match, a key the check doesn't
    /// compare, or a key the caller could not describe). Not a reason
    /// to remove the identity.
    Undetermined,
}

/// Every key of an identity checked against the wallet, and what that
/// says about whose identity it is.
#[derive(Debug)]
pub struct IdentityWalletCheck {
    /// One check per key, in the order the keys were given.
    pub keys: Vec<IdentityKeyWalletCheck>,
    pub ownership: IdentityWalletOwnership,
}

/// The ownership `checks` (one per key in `keys`, in order) give, when
/// `unreadable_key_count` more keys of the identity could not be
/// described to the check.
fn identity_wallet_ownership(
    keys: &[IdentityPublicKey],
    checks: &[IdentityKeyWalletCheck],
    unreadable_key_count: usize,
) -> IdentityWalletOwnership {
    let mut has_master = false;
    let mut every_master_mismatches = true;
    // A key the caller could not describe could be the wallet's.
    let mut could_be_wallets = unreadable_key_count > 0;
    for (key, check) in keys.iter().zip(checks) {
        if key.is_disabled() {
            continue;
        }
        let matches = matches!(check, IdentityKeyWalletCheck::Matches { .. });
        if key.security_level() == SecurityLevel::MASTER {
            if matches {
                return IdentityWalletOwnership::Owned;
            }
            has_master = true;
            if !matches!(check, IdentityKeyWalletCheck::Mismatch { .. }) {
                every_master_mismatches = false;
            }
        }
        could_be_wallets |= matches || matches!(check, IdentityKeyWalletCheck::NotCompared);
    }
    if has_master && every_master_mismatches && !could_be_wallets {
        IdentityWalletOwnership::NotOwned
    } else {
        IdentityWalletOwnership::Undetermined
    }
}

/// Check every key of the identity at `identity_index`, enabled or not,
/// with [`check_identity_key_from_master`], and decide from them whether
/// the identity is this wallet's ([`IdentityWalletOwnership`]).
///
/// `unreadable_key_count` is how many more keys the identity has that
/// the caller could not describe (enabled or not). Any of them could be
/// the wallet's, so they rule out `NotOwned`.
pub fn check_identity_keys_from_master(
    master: &ExtendedPrivKey,
    network: Network,
    identity_index: u32,
    identity_public_keys: &[IdentityPublicKey],
    unreadable_key_count: usize,
) -> Result<IdentityWalletCheck, PlatformWalletError> {
    let keys = identity_public_keys
        .iter()
        .map(|key| check_identity_key_from_master(master, network, identity_index, key))
        .collect::<Result<Vec<_>, _>>()?;
    let ownership = identity_wallet_ownership(identity_public_keys, &keys, unreadable_key_count);
    Ok(IdentityWalletCheck { keys, ownership })
}

/// Identity + DashPay wallet facade.
///
/// A view onto the shared `PlatformWalletInfo` state inside the wallet
/// manager. Covers both the identity lifecycle (registration, discovery,
/// top-up, transfer, withdrawal) and the DashPay-contract operations that
/// live on the same identity (contact requests, contacts, payments,
/// profile, account labels).
///
/// Generic parameter `B` is the [`TransactionBroadcaster`] used by
/// payment paths (DashPay `send_payment`). Defaults to
/// [`SpvBroadcaster`] — the only production broadcaster. Asset-lock
/// funding uses its own pinned-to-`SpvBroadcaster` manager; the `B`
/// parameter is only for the DashPay payment broadcast surface.
pub struct IdentityWallet<B: TransactionBroadcaster + ?Sized = SpvBroadcaster> {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// Shared wallet manager holding key material and wallet info.
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifier for the wallet within the wallet manager.
    pub(crate) wallet_id: WalletId,
    /// Shared asset lock manager for building, broadcasting, and tracking
    /// asset lock transactions. Used by funding methods that build asset
    /// locks from wallet UTXOs.
    // Pinned to `SpvBroadcaster` to match the parent `PlatformWallet`.
    // If/when the outer broadcaster choice changes, flip this line
    // and the corresponding field in `PlatformWallet`.
    pub(crate) asset_locks: Arc<AssetLockManager<SpvBroadcaster>>,
    /// Per-wallet persistence handle for queuing changesets.
    pub(crate) persister: crate::wallet::persister::WalletPersister,
    /// The manager's persistence-fault latch (see
    /// [`PlatformWalletManager::sync_fault_detected`](crate::PlatformWalletManager::sync_fault_detected)).
    /// While it is set the wallet-event adapter holds the durable
    /// `synced_height` back at the last fully persisted height, and a
    /// direct cursor write from this side would advance it past rows that
    /// never landed. The DashPay rescan reconcile consults it before it
    /// stores a backfill record with a lowered cursor.
    pub(crate) sync_fault: Arc<std::sync::atomic::AtomicBool>,
    /// The manager's shared record of each wallet's durable sync cursor,
    /// also the lock that orders cursor writes between the wallet-event
    /// adapter and the DashPay rescan reconcile. See
    /// [`DurableCursors`](crate::changeset::DurableCursors).
    pub(crate) durable_cursors: crate::changeset::DurableCursors,
    /// Broadcaster for DashPay payment transactions. Distinct from the
    /// asset-lock broadcaster — the asset-lock manager is always
    /// `SpvBroadcaster`-pinned, while this one picks the broadcaster
    /// used by `send_payment` (static dispatch per call).
    pub(crate) broadcaster: Arc<B>,
    /// Concrete helper over the SDK's DashPay write operations
    /// (contact-request broadcast, document put), wrapping `sdk`. It
    /// erases the SDK's generic write signatures (`send_contact_request`
    /// is generic over seven type params; the document put rides the
    /// signer-generic `PutDocument` trait) behind two by-value methods
    /// so the call sites stay simple.
    pub(crate) sdk_writer: Arc<super::sdk_writer::SdkWriter>,
    /// Serializes DPNS marketplace mutations and sync reconciliation for
    /// this wallet. Every cloned handle shares the same gate.
    pub(crate) dpns_operation_gate: Arc<tokio::sync::Mutex<()>>,
    /// Bounded ownership-scan cursors, one per wallet identity. This is a
    /// short-lived in-memory optimization; durable marketplace rows remain
    /// the source rendered after process restart.
    pub(crate) dpns_sync_progress:
        Arc<StdMutex<BTreeMap<Identifier, super::dpns_marketplace::DpnsMarketplaceSyncProgress>>>,
}

// Manual `Debug`: the derive would require `B: Debug`, which is not part
// of the `TransactionBroadcaster` bound.
impl<B: TransactionBroadcaster + ?Sized> std::fmt::Debug for IdentityWallet<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityWallet").finish()
    }
}

// Manual `Clone`: the derive would add `where B: Clone`, but `Arc<B>`
// clones without cloning `B` itself, so the bound would be spurious.
// `B: ?Sized` is enough.
impl<B: TransactionBroadcaster + ?Sized> Clone for IdentityWallet<B> {
    fn clone(&self) -> Self {
        Self {
            sdk: Arc::clone(&self.sdk),
            wallet_manager: Arc::clone(&self.wallet_manager),
            wallet_id: self.wallet_id,
            asset_locks: Arc::clone(&self.asset_locks),
            persister: self.persister.clone(),
            sync_fault: Arc::clone(&self.sync_fault),
            durable_cursors: Arc::clone(&self.durable_cursors),
            broadcaster: Arc::clone(&self.broadcaster),
            sdk_writer: Arc::clone(&self.sdk_writer),
            dpns_operation_gate: Arc::clone(&self.dpns_operation_gate),
            dpns_sync_progress: Arc::clone(&self.dpns_sync_progress),
        }
    }
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Build the DIP-9 identity authentication derivation path.
    ///
    /// Path format: `m/9'/coin_type'/5'/0'/key_type'/identity_index'/key_id'`
    pub fn identity_auth_derivation_path(
        network: Network,
        key_derivation_type: KeyDerivationType,
        identity_index: u32,
        key_id: u32,
    ) -> Result<DerivationPath, PlatformWalletError> {
        let base_path: DerivationPath = match network {
            Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
            _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
        }
        .into();

        let key_type_index: u32 = key_derivation_type.into();

        Ok(base_path.extend([
            ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid key type index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid identity index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(key_id).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid key ID: {}", e))
            })?,
        ]))
    }

    /// Derive the raw private key bytes for an identity authentication key.
    ///
    /// Picks the DIP-9 path from the public key's [`KeyType`] and id
    /// ([`identity_key_derivation_path`]) and derives the private key
    /// from the wallet. Fails for a key type the wallet does not derive.
    ///
    /// Returns the bytes wrapped in [`Zeroizing`] so they are automatically
    /// wiped from memory when the value is dropped.
    pub fn derive_identity_key_bytes(
        wallet: &Wallet,
        network: Network,
        identity_index: u32,
        identity_public_key: &IdentityPublicKey,
    ) -> Result<Zeroizing<[u8; 32]>, PlatformWalletError> {
        let key_id = identity_public_key.id();
        let path = identity_key_derivation_path(network, identity_index, identity_public_key)?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "{} keys are not derived by the wallet",
                    identity_public_key.key_type()
                ))
            })?;

        let secret_key = wallet.derive_private_key(&path).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive private key for identity key {}: {}",
                key_id, e
            ))
        })?;

        Ok(secret_key.to_bytes())
    }

    /// Get a read-lock handle to the shared [`WalletManager`].
    ///
    /// Access wallet info via `wm.get_wallet_info(&wallet_id)` and key material
    /// via `wm.get_wallet(&wallet_id)` on the returned guard. The identity
    /// manager is on the wallet info: `info.identity_manager`.
    pub async fn wallet_manager_read(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, WalletManager<PlatformWalletInfo>> {
        self.wallet_manager.read().await
    }

    /// Get a write-lock handle to the shared [`WalletManager`].
    ///
    /// Access wallet info via `wm.get_wallet_info_mut(&wallet_id)` on the
    /// returned guard. This allows callers to mutate managed identities (e.g.
    /// adding or updating identities from an external persistence layer).
    pub async fn wallet_manager_write(
        &self,
    ) -> tokio::sync::RwLockWriteGuard<'_, WalletManager<PlatformWalletInfo>> {
        self.wallet_manager.write().await
    }

    /// Try to acquire a write-lock on the shared [`WalletManager`] without blocking.
    ///
    /// Returns `None` if the lock is currently held by another task.
    /// Useful for synchronous callers that cannot await.
    pub fn try_wallet_manager_write(
        &self,
    ) -> Option<tokio::sync::RwLockWriteGuard<'_, WalletManager<PlatformWalletInfo>>> {
        self.wallet_manager.try_write().ok()
    }

    /// The wallet ID for this identity wallet's underlying key material.
    pub fn wallet_id(&self) -> &WalletId {
        &self.wallet_id
    }

    /// The network this wallet's keys are derived on.
    async fn wallet_network(&self) -> Result<Network, PlatformWalletError> {
        let guard = self.wallet_manager.read().await;
        guard
            .get_wallet(&self.wallet_id)
            .map(|wallet| wallet.network)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))
    }

    /// Check every key of the identity at `identity_index`, enabled or
    /// not, against this wallet's keys, and decide whether the identity is
    /// this wallet's ([`check_identity_keys_from_master`]).
    ///
    /// `resolve_master` runs at most once, and only when a key needs
    /// comparing; an identity with none is answered without the seed. It
    /// is given the wallet's network to build the master key on. The
    /// master it returns must be this wallet's (it derives the wallet's
    /// account-0 xpub), or the check fails with
    /// [`PlatformWalletError::SeedMismatch`], so another wallet's seed can
    /// never make an identity look foreign. The master is wiped after the
    /// check.
    pub async fn check_identity_keys<E: From<PlatformWalletError>>(
        &self,
        identity_index: u32,
        identity_public_keys: &[IdentityPublicKey],
        unreadable_key_count: usize,
        resolve_master: impl FnOnce(Network) -> Result<ExtendedPrivKey, E>,
    ) -> Result<IdentityWalletCheck, E> {
        let network = self.wallet_network().await?;
        let comparisons = identity_public_keys
            .iter()
            .map(|key| key_comparison(network, identity_index, key))
            .collect::<Result<Vec<_>, _>>()?;
        if comparisons
            .iter()
            .all(|comparison| matches!(comparison, KeyComparison::Known(_)))
        {
            let keys: Vec<IdentityKeyWalletCheck> = comparisons
                .into_iter()
                .filter_map(|comparison| match comparison {
                    KeyComparison::Known(check) => Some(check),
                    KeyComparison::AtSlot(_) => None,
                })
                .collect();
            let ownership =
                identity_wallet_ownership(identity_public_keys, &keys, unreadable_key_count);
            return Ok(IdentityWalletCheck { keys, ownership });
        }

        let mut master = resolve_master(network)?;
        let check = match self.verify_master_binds(&master).await {
            Ok(()) => check_identity_keys_from_master(
                &master,
                network,
                identity_index,
                identity_public_keys,
                unreadable_key_count,
            ),
            Err(e) => Err(e),
        };
        master.private_key.zeroize();
        Ok(check?)
    }

    /// [`Self::check_identity_keys`] for one key: whether it is the key
    /// this wallet derives at its slot, from this wallet's seed only.
    pub async fn check_identity_key<E: From<PlatformWalletError>>(
        &self,
        identity_index: u32,
        identity_public_key: &IdentityPublicKey,
        resolve_master: impl FnOnce(Network) -> Result<ExtendedPrivKey, E>,
    ) -> Result<IdentityKeyWalletCheck, E> {
        let check = self
            .check_identity_keys(
                identity_index,
                std::slice::from_ref(identity_public_key),
                0,
                resolve_master,
            )
            .await?;
        check.keys.into_iter().next().ok_or_else(|| {
            E::from(PlatformWalletError::InvalidIdentityData(
                "the key check returned no result".to_string(),
            ))
        })
    }
}

// `refresh_identity` is defined for the production broadcaster only.
impl IdentityWallet {
    /// Re-fetch the identity from Platform ([`Self::refresh_identity`],
    /// which also stores the refreshed key rows) and check every key it
    /// holds afterwards with [`Self::check_identity_keys`], in key id
    /// order, at the identity's slot in this wallet. Ask this before
    /// removing an identity: the stored key rows can be stale (another
    /// device rotated the master key).
    ///
    /// Each refusal is an error, never `NotOwned`: an identity the
    /// manager doesn't hold (`IdentityNotFound`) or holds with no slot in
    /// this wallet (`IdentityIndexNotSet`), both before the fetch, and one
    /// Platform doesn't return. The seed is resolved only after the
    /// refresh.
    pub async fn check_identity_keys_on_platform<E: From<PlatformWalletError>>(
        &self,
        identity_id: &Identifier,
        resolve_master: impl FnOnce(Network) -> Result<ExtendedPrivKey, E>,
    ) -> Result<IdentityWalletCheck, E> {
        self.wallet_identity_slot(identity_id).await?;
        let identity = self.refresh_identity(identity_id).await?;
        // The slot as the manager holds it after the refresh.
        let identity_index = self.wallet_identity_slot(identity_id).await?;
        let identity_public_keys: Vec<IdentityPublicKey> =
            identity.public_keys().values().cloned().collect();
        self.check_identity_keys(identity_index, &identity_public_keys, 0, resolve_master)
            .await
    }

    /// The slot `identity_id` has in this wallet: `IdentityNotFound` when
    /// the manager doesn't hold it, `IdentityIndexNotSet` when it holds it
    /// with no slot in this wallet (observed, or another wallet's).
    async fn wallet_identity_slot(
        &self,
        identity_id: &Identifier,
    ) -> Result<u32, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let manager = &wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?
            .identity_manager;
        if manager.identity(identity_id).is_none() {
            return Err(PlatformWalletError::IdentityNotFound(*identity_id));
        }
        manager
            .wallet_identity(&self.wallet_id, identity_id)
            .and_then(|managed| managed.identity_index)
            .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))
    }
}

impl PlatformWallet {
    /// Wallet-level entry point for [`IdentityWallet::check_identity_keys`].
    pub async fn check_identity_keys<E: From<PlatformWalletError>>(
        &self,
        identity_index: u32,
        identity_public_keys: &[IdentityPublicKey],
        unreadable_key_count: usize,
        resolve_master: impl FnOnce(Network) -> Result<ExtendedPrivKey, E>,
    ) -> Result<IdentityWalletCheck, E> {
        self.identity()
            .check_identity_keys(
                identity_index,
                identity_public_keys,
                unreadable_key_count,
                resolve_master,
            )
            .await
    }

    /// Wallet-level entry point for
    /// [`IdentityWallet::check_identity_keys_on_platform`].
    pub async fn check_identity_keys_on_platform<E: From<PlatformWalletError>>(
        &self,
        identity_id: &Identifier,
        resolve_master: impl FnOnce(Network) -> Result<ExtendedPrivKey, E>,
    ) -> Result<IdentityWalletCheck, E> {
        self.identity()
            .check_identity_keys_on_platform(identity_id, resolve_master)
            .await
    }

    /// Wallet-level entry point for [`IdentityWallet::check_identity_key`].
    pub async fn check_identity_key<E: From<PlatformWalletError>>(
        &self,
        identity_index: u32,
        identity_public_key: &IdentityPublicKey,
        resolve_master: impl FnOnce(Network) -> Result<ExtendedPrivKey, E>,
    ) -> Result<IdentityKeyWalletCheck, E> {
        self.identity()
            .check_identity_key(identity_index, identity_public_key, resolve_master)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::util::hash::ripemd160_sha256;
    use key_wallet::mnemonic::Mnemonic;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use key_wallet::Network;

    /// English BIP-39 test vector (all-zero entropy). Same fixture the
    /// FFI-side derive tests use, so the derivations here can be
    /// cross-checked against those if a regression ever appears on one
    /// side only.
    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    /// Build a key-resident `WalletType::Mnemonic` wallet on `network`
    /// from [`TEST_MNEMONIC`]. `WalletAccountCreationOptions::None`
    /// skips the BLS/EdDSA provider accounts the discovery scan never
    /// touches — the identity-auth derivation walks the master xpriv,
    /// not the per-account collection, so no accounts are needed.
    fn mnemonic_wallet(network: Network) -> Wallet {
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC).expect("valid English test mnemonic");
        Wallet::from_mnemonic(mnemonic, network, WalletAccountCreationOptions::None)
            .expect("from_mnemonic should build a Mnemonic wallet")
    }

    /// The BIP-32 master node for [`TEST_MNEMONIC`] on `network` — the
    /// same node `derive_extended_private_key` reconstructs internally
    /// (`RootExtendedPrivKey::new_master(seed).to_extended_priv_key(network)`
    /// is byte-for-byte `ExtendedPrivKey::new_master(network, seed)`).
    fn master_for(network: Network) -> ExtendedPrivKey {
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC).expect("valid English test mnemonic");
        let seed = mnemonic.to_seed("");
        ExtendedPrivKey::new_master(network, &*seed).expect("master xpriv from test seed")
    }

    /// (a) The master-based key-hash helper must produce the SAME 20-byte
    /// hash as the wallet-internal `derive_identity_auth_key_hash` on a
    /// key-resident `WalletType::Mnemonic` wallet, for every slot the
    /// rescan probes. This is the core correctness guarantee: a rescan
    /// driven through the resolved master derives exactly what a
    /// key-resident wallet derives — same identity, same on-chain
    /// pubkey-hash lookup.
    #[test]
    fn master_hash_matches_resident_wallet_hash() {
        for network in [Network::Mainnet, Network::Testnet] {
            let wallet = mnemonic_wallet(network);
            let master = master_for(network);

            // Walk a window matching the discovery scan's MASTER slot
            // across several identity indices.
            for identity_index in 0..6u32 {
                let resident = derive_identity_auth_key_hash(
                    &wallet,
                    network,
                    identity_index,
                    MASTER_KEY_INDEX,
                )
                .expect("resident-wallet derive should succeed for a Mnemonic wallet");

                let from_master = derive_identity_auth_key_hash_from_master(
                    &master,
                    network,
                    identity_index,
                    MASTER_KEY_INDEX,
                )
                .expect("master derive should succeed");

                assert_eq!(
                    resident, from_master,
                    "master-based hash must equal resident-wallet hash \
                     (network={network:?}, identity_index={identity_index})"
                );
            }

            // Non-MASTER key indices must also agree — the helper is the
            // generic per-slot derive, not MASTER-pinned.
            let resident = derive_identity_auth_key_hash(&wallet, network, 3, 7)
                .expect("resident derive at (3,7)");
            let from_master = derive_identity_auth_key_hash_from_master(&master, network, 3, 7)
                .expect("master derive at (3,7)");
            assert_eq!(resident, from_master, "non-MASTER slot must also agree");
        }
    }

    /// (b) Pin the bug and its fix: `derive_identity_auth_key_hash` on a
    /// `WalletType::ExternalSignable` wallet ERRORS (the seed lives
    /// outside the in-process wallet — this is the exact failure the
    /// rescan UI surfaced), while the master-based helper SUCCEEDS for
    /// the same slot and yields the same hash a key-resident wallet
    /// would. Mirrors how registration derives on these wallets.
    #[test]
    fn external_signable_errors_but_master_succeeds() {
        let network = Network::Testnet;

        // Reference hash from a key-resident wallet at the probed slot.
        let resident_wallet = mnemonic_wallet(network);
        let expected =
            derive_identity_auth_key_hash(&resident_wallet, network, 0, MASTER_KEY_INDEX)
                .expect("resident derive should succeed");

        // Downgrade a clone to ExternalSignable: same wallet id, but the
        // key material is dropped — exactly the iOS Keychain-backed shape
        // loaded into the in-process `WalletManager`.
        let mut external = mnemonic_wallet(network);
        external.downgrade_to_external_signable();

        let resident_err = derive_identity_auth_key_hash(&external, network, 0, MASTER_KEY_INDEX);
        let err = resident_err
            .expect_err("ExternalSignable wallet has no resident key — derive must error");
        let msg = err.to_string();
        assert!(
            msg.contains("External signable wallet has no private key"),
            "error should be the External-signable no-private-key failure, got: {msg}"
        );

        // The master-based helper succeeds for the same slot and matches
        // the key-resident reference hash — this is the rescan fix.
        let master = master_for(network);
        let from_master =
            derive_identity_auth_key_hash_from_master(&master, network, 0, MASTER_KEY_INDEX)
                .expect("master derive must succeed where the resident derive failed");
        assert_eq!(
            from_master, expected,
            "master derive on an ExternalSignable wallet must reproduce the \
             key-resident hash for the same slot"
        );
    }

    /// (c) Parity with registration's in-creation key #0: the
    /// master-based hash at `(identity_index, MASTER_KEY_INDEX)` must
    /// equal `ripemd160_sha256` of the pubkey
    /// `derive_ecdsa_identity_auth_keypair_from_master` produces at the
    /// same slot — i.e. the rescan probes the hash of the very key
    /// registration publishes as MASTER auth key #0.
    #[test]
    fn master_hash_matches_registration_keypair_pubkey_hash() {
        let network = Network::Testnet;
        let master = master_for(network);

        for identity_index in 0..4u32 {
            let keypair = derive_ecdsa_identity_auth_keypair_from_master(
                &master,
                network,
                identity_index,
                MASTER_KEY_INDEX,
            )
            .expect("registration-shaped keypair derive should succeed");
            let expected = ripemd160_sha256(&keypair.public_key);

            let hash = derive_identity_auth_key_hash_from_master(
                &master,
                network,
                identity_index,
                MASTER_KEY_INDEX,
            )
            .expect("master hash derive should succeed");

            assert_eq!(
                hash.as_slice(),
                expected.as_slice(),
                "rescan hash must be ripemd160_sha256 of the registration \
                 keypair's pubkey (identity_index={identity_index})"
            );
        }
    }

    // ── DashPay Connect keys ──────────────────────────────────────────

    const CONNECT_IDENTITY: [u8; 32] = [0x35; 32];
    const CONNECT_LEAF: [u8; 32] = [0x6B; 32];

    fn session_key() -> ConnectKey {
        ConnectKey::SessionAuthentication {
            identity_id: Identifier::from(CONNECT_IDENTITY),
            request_id: CONNECT_LEAF,
        }
    }

    fn encryption_key(purpose: ApplicationKeyPurpose, rotation: u32) -> ConnectKey {
        ConnectKey::AppEncryption {
            identity_id: Identifier::from(CONNECT_IDENTITY),
            contract_id: CONNECT_LEAF,
            purpose,
            rotation,
        }
    }

    fn legacy_encryption_key(purpose: ApplicationKeyPurpose) -> ConnectKey {
        ConnectKey::AppEncryptionLegacy {
            identity_id: Identifier::from(CONNECT_IDENTITY),
            contract_id: CONNECT_LEAF,
            purpose,
        }
    }

    /// `00 01 .. 1f` and `ff fe .. e0`: asymmetric 256-bit children, so a
    /// swapped identity/contract level or a byte-order slip changes the key.
    fn ascending() -> [u8; 32] {
        std::array::from_fn(|i| i as u8)
    }

    fn descending() -> [u8; 32] {
        std::array::from_fn(|i| 0xFF - i as u8)
    }

    /// The keys DIP-13 pins for these inputs (BIP-39 `abandon x11 about`,
    /// empty passphrase); a change orphans every key already registered
    /// on-chain. The public key is the compressed point of the returned
    /// scalar.
    #[test]
    fn connect_keypairs_match_the_pinned_vectors() {
        use dpp::secp256k1::{PublicKey as SecpPublicKey, SecretKey};
        use ApplicationKeyPurpose::{Decryption, Encryption};

        let rotated = |identity: [u8; 32], contract, purpose, rotation| ConnectKey::AppEncryption {
            identity_id: Identifier::from(identity),
            contract_id: contract,
            purpose,
            rotation,
        };
        let legacy = |identity: [u8; 32], contract, purpose| ConnectKey::AppEncryptionLegacy {
            identity_id: Identifier::from(identity),
            contract_id: contract,
            purpose,
        };
        let (asc, desc) = (ascending(), descending());

        for (network, key, expected) in [
            (
                Network::Testnet,
                session_key(),
                "022c8b2e806244482374b1caf8306146dc03aad3b99a5954efd5e70a0eddd37a5d",
            ),
            (
                Network::Mainnet,
                rotated(CONNECT_IDENTITY, CONNECT_LEAF, Encryption, 0),
                "031a882c119e2b1f8e0349b444d877afa2599efeef49a26930fd25518cb6e9d57a",
            ),
            (
                Network::Mainnet,
                rotated(CONNECT_IDENTITY, CONNECT_LEAF, Encryption, 1),
                "03d0f6f0f48d968b9554d8e7fe467d7135bacf29d5c4994ca593bd10ebeed696f6",
            ),
            (
                Network::Mainnet,
                rotated(asc, desc, Encryption, 0),
                "02dfed57ddaaea8c0758e8da44513520038eb8d6b0b92109953c14d6ab1087259b",
            ),
            (
                Network::Mainnet,
                rotated(asc, desc, Decryption, 0),
                "020bcde800f41ad1f924477a9502f111445224cc9ab70114ae4e5099ba02bdd034",
            ),
            (
                Network::Mainnet,
                rotated(asc, desc, Decryption, 1),
                "021b97869c21933673076a938337c28848c8bf8094bbe6883e3a4acbf33c639072",
            ),
            (
                Network::Mainnet,
                legacy(CONNECT_IDENTITY, CONNECT_LEAF, Encryption),
                "03e989de1b62f137231cc06659c810c17100becd1f5e23d5710faa7602769fe434",
            ),
            (
                Network::Mainnet,
                legacy(asc, desc, Encryption),
                "03c4e462229f176595ec17e28e9096d3cfbc42496fb8ed4d48f8f58355d6105aa9",
            ),
            (
                Network::Mainnet,
                legacy(asc, desc, Decryption),
                "031fd0a3cfc01bebb89d43c2e12a6321c37ebdacb62c09495899e5b65a24fa5c99",
            ),
        ] {
            let derived = derive_connect_keypair_from_master(&master_for(network), network, key)
                .expect("connect derive");
            assert_eq!(hex::encode(derived.public_key), expected, "{key:?}");
            assert_eq!(
                derived.derivation_path,
                key.derivation_path(network).expect("connect path")
            );

            let secret: [u8; 32] = *derived.private_key;
            let sk = SecretKey::from_secret_bytes(secret).expect("valid scalar");
            assert_eq!(
                SecpPublicKey::from_secret_key(&sk).serialize(),
                derived.public_key
            );
        }
    }

    /// The rotated path is the legacy path plus one hardened `rotation'`
    /// level: nine levels against eight.
    #[test]
    fn app_encryption_path_appends_a_hardened_rotation_to_the_legacy_path() {
        let legacy = legacy_encryption_key(ApplicationKeyPurpose::Decryption)
            .derivation_path(Network::Testnet)
            .expect("legacy path");
        let rotated = encryption_key(ApplicationKeyPurpose::Decryption, 7)
            .derivation_path(Network::Testnet)
            .expect("rotated path");

        assert_eq!(legacy.len(), 8);
        assert_eq!(rotated.len(), 9);
        assert_eq!(rotated[..8], legacy[..]);
        assert_eq!(rotated[8], ChildNumber::Hardened { index: 7 });
    }

    /// A rotation is a 31-bit hardened index; anything wider is refused
    /// rather than wrapped onto another key.
    #[test]
    fn app_encryption_rotation_must_fit_a_hardened_index() {
        let key = |rotation| encryption_key(ApplicationKeyPurpose::Encryption, rotation);
        let master = master_for(Network::Testnet);

        assert!(
            derive_connect_keypair_from_master(&master, Network::Testnet, key(0x7FFF_FFFF)).is_ok()
        );
        for rotation in [0x8000_0000, u32::MAX] {
            let err = derive_connect_keypair_from_master(&master, Network::Testnet, key(rotation))
                .err()
                .expect("rotation >= 2^31 must be refused");
            assert!(
                matches!(err, PlatformWalletError::InvalidParameter(_)),
                "{err:?}"
            );
        }
    }

    /// The two halves of an encryption pair, each rotation, the legacy
    /// layout and the session key are all distinct.
    #[test]
    fn connect_keys_are_distinct_per_kind_purpose_and_rotation() {
        let master = master_for(Network::Testnet);
        let derive = |key| {
            derive_connect_keypair_from_master(&master, Network::Testnet, key)
                .expect("connect derive")
                .public_key
        };
        let all = [
            derive(session_key()),
            derive(encryption_key(ApplicationKeyPurpose::Encryption, 0)),
            derive(encryption_key(ApplicationKeyPurpose::Decryption, 0)),
            derive(encryption_key(ApplicationKeyPurpose::Encryption, 1)),
            derive(legacy_encryption_key(ApplicationKeyPurpose::Encryption)),
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b, "connect keys collided (index {i})");
            }
        }
    }

    /// The raw-level constructors accept exactly the DIP-13 combinations
    /// and build the matching variant.
    #[test]
    fn connect_key_from_parts_builds_the_valid_combinations() {
        let identity_id = Identifier::from(CONNECT_IDENTITY);
        assert_eq!(
            ConnectKey::from_parts(6, identity_id, CONNECT_LEAF, 0, 0).expect("session"),
            session_key()
        );
        assert_eq!(
            ConnectKey::from_parts(7, identity_id, CONNECT_LEAF, 1, 0).expect("encryption"),
            encryption_key(ApplicationKeyPurpose::Encryption, 0)
        );
        assert_eq!(
            ConnectKey::from_parts(7, identity_id, CONNECT_LEAF, 2, 0x7FFF_FFFF)
                .expect("decryption at the top rotation"),
            encryption_key(ApplicationKeyPurpose::Decryption, 0x7FFF_FFFF)
        );
        assert_eq!(
            ConnectKey::legacy_app_encryption_from_parts(identity_id, CONNECT_LEAF, 1)
                .expect("legacy encryption"),
            legacy_encryption_key(ApplicationKeyPurpose::Encryption)
        );
        assert_eq!(
            ConnectKey::legacy_app_encryption_from_parts(identity_id, CONNECT_LEAF, 2)
                .expect("legacy decryption"),
            legacy_encryption_key(ApplicationKeyPurpose::Decryption)
        );
    }

    #[test]
    fn connect_key_from_parts_refuses_other_combinations() {
        let identity_id = Identifier::from(CONNECT_IDENTITY);
        for (sub_feature, purpose, rotation, expected) in [
            (7, 3, 0, "purpose 3"),
            (7, 0, 0, "purpose 0"),
            (7, 1, 0x8000_0000, "rotation 2147483648"),
            (7, 2, u32::MAX, "rotation 4294967295"),
            (6, 1, 0, "sub_feature 6 with purpose 1 and rotation 0"),
            (6, 0, 1, "sub_feature 6 with purpose 0 and rotation 1"),
            (5, 0, 0, "sub_feature 5 with purpose 0 and rotation 0"),
        ] {
            let err =
                ConnectKey::from_parts(sub_feature, identity_id, CONNECT_LEAF, purpose, rotation)
                    .expect_err("invalid combination must be refused");
            match err {
                PlatformWalletError::InvalidParameter(message) => {
                    assert!(message.contains(expected), "{message}")
                }
                other => panic!("expected InvalidParameter, got {other:?}"),
            }
        }
        for purpose in [0, 3] {
            let err =
                ConnectKey::legacy_app_encryption_from_parts(identity_id, CONNECT_LEAF, purpose)
                    .expect_err("invalid legacy purpose must be refused");
            assert!(
                matches!(&err, PlatformWalletError::InvalidParameter(m) if m.contains(&format!("purpose {purpose}"))),
                "{err:?}"
            );
        }
    }

    #[test]
    fn should_say_which_identity_key_types_the_wallet_derives() {
        for (key_type, expected) in [
            (KeyType::ECDSA_SECP256K1, Some(KeyDerivationType::ECDSA)),
            (KeyType::BLS12_381, Some(KeyDerivationType::BLS)),
            (KeyType::ECDSA_HASH160, Some(KeyDerivationType::ECDSA)),
            (KeyType::BIP13_SCRIPT_HASH, None),
            (KeyType::EDDSA_25519_HASH160, Some(KeyDerivationType::ECDSA)),
            (KeyType::REDPALLAS_HECATE, None),
        ] {
            assert_eq!(
                identity_key_derivation_type(key_type),
                expected,
                "{key_type}"
            );
        }
    }

    #[test]
    fn should_refuse_to_derive_a_hecate_signer_key() {
        let wallet = mnemonic_wallet(Network::Testnet);
        let hecate_signer_key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: 3,
            purpose: Purpose::HECATE_SIGNER,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            key_type: KeyType::REDPALLAS_HECATE,
            read_only: false,
            data: BinaryData::new(vec![1; 64]),
            disabled_at: None,
        }
        .into();

        let err = IdentityWallet::<SpvBroadcaster>::derive_identity_key_bytes(
            &wallet,
            Network::Testnet,
            0,
            &hecate_signer_key,
        )
        .expect_err("the wallet does not derive Hecate signer keys");
        assert!(
            matches!(&err, PlatformWalletError::InvalidIdentityData(m) if m.contains("REDPALLAS_HECATE")),
            "{err:?}"
        );
    }

    // ── Wallet key check ──────────────────────────────────────────────

    fn identity_key(
        id: u32,
        key_type: KeyType,
        purpose: Purpose,
        data: Vec<u8>,
    ) -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id,
            purpose,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type,
            read_only: false,
            data: BinaryData::new(data),
            disabled_at: None,
        }
        .into()
    }

    /// The wallet's 32 bytes at the `key_derivation_type` slot
    /// `(identity_index, key_index)`.
    fn slot_bytes(
        master: &ExtendedPrivKey,
        network: Network,
        key_derivation_type: KeyDerivationType,
        identity_index: u32,
        key_index: u32,
    ) -> (DerivationPath, [u8; 32]) {
        let path = identity_auth_derivation_path_for_type(
            network,
            key_derivation_type,
            identity_index,
            key_index,
        )
        .expect("slot path");
        let bytes = *master
            .derive_priv(&path)
            .expect("slot derive")
            .private_key
            .to_bytes();
        (path, bytes)
    }

    fn check(key: &IdentityPublicKey, identity_index: u32) -> IdentityKeyWalletCheck {
        check_identity_key_from_master(
            &master_for(Network::Testnet),
            Network::Testnet,
            identity_index,
            key,
        )
        .expect("check")
    }

    #[test]
    fn should_not_check_a_key_of_a_type_the_wallet_does_not_derive() {
        let hecate_signer_key = identity_key(
            3,
            KeyType::REDPALLAS_HECATE,
            Purpose::HECATE_SIGNER,
            vec![1; 64],
        );
        let script_hash_key = identity_key(
            4,
            KeyType::BIP13_SCRIPT_HASH,
            Purpose::TRANSFER,
            vec![2; 20],
        );
        for key in [hecate_signer_key, script_hash_key] {
            assert!(
                matches!(check(&key, 0), IdentityKeyWalletCheck::NotDerived),
                "{key:?}"
            );
        }
    }

    #[test]
    fn should_match_an_ecdsa_key_the_wallet_derives_at_its_slot() {
        let network = Network::Testnet;
        let slot =
            derive_ecdsa_identity_auth_keypair_from_master(&master_for(network), network, 2, 3)
                .expect("slot keypair");
        let key = identity_key(
            3,
            KeyType::ECDSA_SECP256K1,
            Purpose::AUTHENTICATION,
            slot.public_key.to_vec(),
        );

        match check(&key, 2) {
            IdentityKeyWalletCheck::Matches {
                derivation_path,
                private_key,
                public_key_hash,
            } => {
                assert_eq!(derivation_path, slot.derivation_path);
                assert_eq!(*private_key, *slot.private_key);
                assert_eq!(
                    public_key_hash.as_slice(),
                    ripemd160_sha256(&slot.public_key).as_slice()
                );
            }
            other => panic!("expected Matches, got {other:?}"),
        }
    }

    #[test]
    fn should_match_an_ecdsa_hash160_key_the_wallet_derives_at_its_slot() {
        let network = Network::Testnet;
        let slot =
            derive_ecdsa_identity_auth_keypair_from_master(&master_for(network), network, 1, 2)
                .expect("slot keypair");
        let pubkey_hash = ripemd160_sha256(&slot.public_key).to_vec();
        let key = identity_key(
            2,
            KeyType::ECDSA_HASH160,
            Purpose::TRANSFER,
            pubkey_hash.clone(),
        );

        match check(&key, 1) {
            IdentityKeyWalletCheck::Matches {
                derivation_path,
                private_key,
                public_key_hash,
            } => {
                assert_eq!(derivation_path, slot.derivation_path);
                assert_eq!(*private_key, *slot.private_key);
                assert_eq!(public_key_hash.to_vec(), pubkey_hash);
            }
            other => panic!("expected Matches, got {other:?}"),
        }
    }

    #[test]
    fn should_report_a_mismatch_for_a_key_from_another_slot() {
        let network = Network::Testnet;
        let master = master_for(network);
        let other_slot = derive_ecdsa_identity_auth_keypair_from_master(&master, network, 2, 3)
            .expect("other slot keypair");
        let own_slot = derive_ecdsa_identity_auth_keypair_from_master(&master, network, 2, 4)
            .expect("own slot keypair");
        // Key 4 carrying key 3's public key.
        let key = identity_key(
            4,
            KeyType::ECDSA_SECP256K1,
            Purpose::AUTHENTICATION,
            other_slot.public_key.to_vec(),
        );

        match check(&key, 2) {
            IdentityKeyWalletCheck::Mismatch {
                derivation_path,
                derived_public_key_data,
            } => {
                assert_eq!(derivation_path, own_slot.derivation_path);
                assert_eq!(derived_public_key_data, own_slot.public_key.to_vec());
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }

    #[test]
    fn should_match_a_bls_key_the_wallet_derives_at_its_bls_slot() {
        let network = Network::Testnet;
        let (path, bytes) = slot_bytes(&master_for(network), network, KeyDerivationType::BLS, 0, 2);
        let data = KeyType::BLS12_381
            .public_key_data_from_private_key_data(&bytes, network)
            .expect("BLS public key");
        let key = identity_key(2, KeyType::BLS12_381, Purpose::AUTHENTICATION, data.clone());

        match check(&key, 0) {
            IdentityKeyWalletCheck::Matches {
                derivation_path,
                private_key,
                public_key_hash,
            } => {
                assert_eq!(derivation_path, path);
                assert_eq!(*private_key, bytes);
                assert_eq!(
                    public_key_hash.as_slice(),
                    ripemd160_sha256(&data).as_slice()
                );
            }
            other => panic!("expected Matches, got {other:?}"),
        }
    }

    /// A BLS key is compared with its own BLS slot, and the mismatch
    /// reports that slot's BLS public key.
    #[test]
    fn should_report_a_mismatch_for_a_bls_key_holding_another_slots_bls_key() {
        let network = Network::Testnet;
        let master = master_for(network);
        let (_, other_bytes) = slot_bytes(&master, network, KeyDerivationType::BLS, 0, 2);
        let (own_path, own_bytes) = slot_bytes(&master, network, KeyDerivationType::BLS, 0, 3);
        let bls_public_key = |bytes: &[u8; 32]| {
            KeyType::BLS12_381
                .public_key_data_from_private_key_data(bytes, network)
                .expect("BLS public key")
        };
        let key = identity_key(
            3,
            KeyType::BLS12_381,
            Purpose::AUTHENTICATION,
            bls_public_key(&other_bytes),
        );

        match check(&key, 0) {
            IdentityKeyWalletCheck::Mismatch {
                derivation_path,
                derived_public_key_data,
            } => {
                assert_eq!(derivation_path, own_path);
                assert_eq!(derived_public_key_data, bls_public_key(&own_bytes));
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }

    #[test]
    fn should_match_an_eddsa_key_the_wallet_derives_at_its_ecdsa_slot() {
        let network = Network::Testnet;
        let slot =
            derive_ecdsa_identity_auth_keypair_from_master(&master_for(network), network, 0, 5)
                .expect("slot keypair");
        let data = KeyType::EDDSA_25519_HASH160
            .public_key_data_from_private_key_data(&slot.private_key, network)
            .expect("EdDSA public key hash");
        let key = identity_key(5, KeyType::EDDSA_25519_HASH160, Purpose::TRANSFER, data);

        match check(&key, 0) {
            IdentityKeyWalletCheck::Matches {
                derivation_path,
                private_key,
                ..
            } => {
                assert_eq!(derivation_path, slot.derivation_path);
                assert_eq!(*private_key, *slot.private_key);
            }
            other => panic!("expected Matches, got {other:?}"),
        }
    }

    /// Signing derives with `derive_identity_key_bytes` on a key-resident
    /// wallet; the check derives from the master. Both must land on the
    /// same key for every key type the wallet derives.
    #[test]
    fn should_sign_with_the_key_the_check_matches() {
        let network = Network::Testnet;
        let master = master_for(network);
        let wallet = mnemonic_wallet(network);

        let ecdsa = derive_ecdsa_identity_auth_keypair_from_master(&master, network, 0, 1)
            .expect("slot keypair");
        let (_, bls_bytes) = slot_bytes(&master, network, KeyDerivationType::BLS, 0, 1);
        let keys = [
            identity_key(
                1,
                KeyType::ECDSA_SECP256K1,
                Purpose::AUTHENTICATION,
                ecdsa.public_key.to_vec(),
            ),
            identity_key(
                1,
                KeyType::ECDSA_HASH160,
                Purpose::TRANSFER,
                ripemd160_sha256(&ecdsa.public_key).to_vec(),
            ),
            identity_key(
                1,
                KeyType::EDDSA_25519_HASH160,
                Purpose::TRANSFER,
                KeyType::EDDSA_25519_HASH160
                    .public_key_data_from_private_key_data(&ecdsa.private_key, network)
                    .expect("EdDSA public key hash"),
            ),
            identity_key(
                1,
                KeyType::BLS12_381,
                Purpose::AUTHENTICATION,
                KeyType::BLS12_381
                    .public_key_data_from_private_key_data(&bls_bytes, network)
                    .expect("BLS public key"),
            ),
        ];

        for key in keys {
            let signing_bytes = IdentityWallet::<SpvBroadcaster>::derive_identity_key_bytes(
                &wallet, network, 0, &key,
            )
            .expect("resident derive");
            match check(&key, 0) {
                IdentityKeyWalletCheck::Matches { private_key, .. } => {
                    assert_eq!(*private_key, *signing_bytes, "{key:?}")
                }
                other => panic!("expected Matches for {key:?}, got {other:?}"),
            }
        }
    }
    // ── Identity ownership ────────────────────────────────────────────

    const DISABLED_AT: u64 = 1_700_000_000_000;

    fn ecdsa_key(
        id: u32,
        security_level: SecurityLevel,
        data: Vec<u8>,
        disabled_at: Option<u64>,
    ) -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(data),
            disabled_at,
        }
        .into()
    }

    /// The compressed public key the wallet derives at the ECDSA slot
    /// `(identity_index, key_index)` on testnet.
    fn slot_public_key(identity_index: u32, key_index: u32) -> Vec<u8> {
        derive_ecdsa_identity_auth_keypair_from_master(
            &master_for(Network::Testnet),
            Network::Testnet,
            identity_index,
            key_index,
        )
        .expect("slot keypair")
        .public_key
        .to_vec()
    }

    fn check_all(keys: &[IdentityPublicKey], identity_index: u32) -> IdentityWalletCheck {
        check_identity_keys_from_master(
            &master_for(Network::Testnet),
            Network::Testnet,
            identity_index,
            keys,
            0,
        )
        .expect("check")
    }

    fn a_match() -> IdentityKeyWalletCheck {
        IdentityKeyWalletCheck::Matches {
            derivation_path: DerivationPath::master(),
            private_key: Zeroizing::new([1; 32]),
            public_key_hash: [0; 20],
        }
    }

    fn a_mismatch() -> IdentityKeyWalletCheck {
        IdentityKeyWalletCheck::Mismatch {
            derivation_path: DerivationPath::master(),
            derived_public_key_data: vec![2; 33],
        }
    }

    fn rule_key(id: u32, security_level: SecurityLevel, disabled: bool) -> IdentityPublicKey {
        ecdsa_key(
            id,
            security_level,
            vec![2; 33],
            disabled.then_some(DISABLED_AT),
        )
    }

    /// A master key rotation leaves the former master disabled. A foreign
    /// former master must not make an identity whose enabled master is
    /// the wallet's look foreign.
    #[test]
    fn should_own_an_identity_whose_enabled_master_replaced_a_foreign_one() {
        let keys = [
            ecdsa_key(
                0,
                SecurityLevel::MASTER,
                slot_public_key(1, 9),
                Some(DISABLED_AT),
            ),
            ecdsa_key(1, SecurityLevel::MASTER, slot_public_key(1, 1), None),
        ];

        let check = check_all(&keys, 1);

        assert_eq!(check.ownership, IdentityWalletOwnership::Owned);
        assert!(matches!(
            check.keys[0],
            IdentityKeyWalletCheck::Mismatch { .. }
        ));
        assert!(matches!(
            check.keys[1],
            IdentityKeyWalletCheck::Matches { .. }
        ));
    }

    /// Keys added from elsewhere (ECDSA or BLS) do not make an identity
    /// whose master is the wallet's look foreign.
    #[test]
    fn should_own_an_identity_whose_master_matches_beside_foreign_keys() {
        let master = master_for(Network::Testnet);
        let (_, foreign_bls_bytes) =
            slot_bytes(&master, Network::Testnet, KeyDerivationType::BLS, 1, 8);
        let foreign_bls = IdentityPublicKeyV0 {
            id: 2,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::BLS12_381,
            read_only: false,
            data: BinaryData::new(
                KeyType::BLS12_381
                    .public_key_data_from_private_key_data(&foreign_bls_bytes, Network::Testnet)
                    .expect("BLS public key"),
            ),
            disabled_at: None,
        }
        .into();
        let keys = [
            ecdsa_key(0, SecurityLevel::MASTER, slot_public_key(1, 0), None),
            ecdsa_key(1, SecurityLevel::HIGH, slot_public_key(1, 7), None),
            foreign_bls,
        ];

        let check = check_all(&keys, 1);

        assert_eq!(check.ownership, IdentityWalletOwnership::Owned);
        assert!(matches!(
            check.keys[1],
            IdentityKeyWalletCheck::Mismatch { .. }
        ));
        assert!(matches!(
            check.keys[2],
            IdentityKeyWalletCheck::Mismatch { .. }
        ));
    }

    #[test]
    fn should_not_own_an_identity_whose_only_enabled_master_is_foreign() {
        let keys = [
            rule_key(0, SecurityLevel::MASTER, false),
            rule_key(1, SecurityLevel::HIGH, false),
        ];
        assert_eq!(
            identity_wallet_ownership(&keys, &[a_mismatch(), a_mismatch()], 0),
            IdentityWalletOwnership::NotOwned
        );
        assert_eq!(
            identity_wallet_ownership(
                &keys,
                &[a_mismatch(), IdentityKeyWalletCheck::NotDerived],
                0
            ),
            IdentityWalletOwnership::NotOwned
        );
    }

    #[test]
    fn should_leave_ownership_undetermined_when_another_enabled_key_is_the_wallets() {
        let keys = [
            rule_key(0, SecurityLevel::MASTER, false),
            rule_key(1, SecurityLevel::HIGH, false),
        ];
        assert_eq!(
            identity_wallet_ownership(&keys, &[a_mismatch(), a_match()], 0),
            IdentityWalletOwnership::Undetermined
        );
    }

    #[test]
    fn should_leave_ownership_undetermined_without_an_enabled_master() {
        let keys = [rule_key(0, SecurityLevel::MASTER, true)];
        assert_eq!(
            identity_wallet_ownership(&keys, &[a_mismatch()], 0),
            IdentityWalletOwnership::Undetermined
        );
        assert_eq!(
            identity_wallet_ownership(&[], &[], 0),
            IdentityWalletOwnership::Undetermined
        );
    }

    #[test]
    fn should_leave_ownership_undetermined_for_a_master_the_wallet_does_not_derive() {
        let keys = [rule_key(0, SecurityLevel::MASTER, false)];
        assert_eq!(
            identity_wallet_ownership(&keys, &[IdentityKeyWalletCheck::NotDerived], 0),
            IdentityWalletOwnership::Undetermined
        );
    }

    #[test]
    fn should_not_let_a_disabled_matching_key_keep_a_foreign_identity() {
        let keys = [
            rule_key(0, SecurityLevel::MASTER, false),
            rule_key(1, SecurityLevel::HIGH, true),
        ];
        assert_eq!(
            identity_wallet_ownership(&keys, &[a_mismatch(), a_match()], 0),
            IdentityWalletOwnership::NotOwned
        );
    }

    #[test]
    fn should_check_every_key_in_the_order_given() {
        let hash160_key = IdentityPublicKeyV0 {
            id: 2,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: BinaryData::new(ripemd160_sha256(&slot_public_key(0, 2)).to_vec()),
            disabled_at: None,
        }
        .into();
        let keys = [
            identity_key(
                3,
                KeyType::REDPALLAS_HECATE,
                Purpose::HECATE_SIGNER,
                vec![1; 64],
            ),
            ecdsa_key(0, SecurityLevel::MASTER, slot_public_key(0, 0), None),
            ecdsa_key(1, SecurityLevel::HIGH, slot_public_key(0, 4), None),
            hash160_key,
        ];

        let check = check_all(&keys, 0);

        assert_eq!(check.keys.len(), keys.len());
        assert!(matches!(check.keys[0], IdentityKeyWalletCheck::NotDerived));
        assert!(matches!(
            check.keys[1],
            IdentityKeyWalletCheck::Matches { .. }
        ));
        assert!(matches!(
            check.keys[2],
            IdentityKeyWalletCheck::Mismatch { .. }
        ));
        assert!(matches!(
            check.keys[3],
            IdentityKeyWalletCheck::Matches { .. }
        ));
        assert_eq!(check.ownership, IdentityWalletOwnership::Owned);
    }

    /// The uncompressed 65-byte encoding of the wallet's ECDSA key at
    /// `(identity_index, key_index)` on testnet.
    fn uncompressed_slot_public_key(identity_index: u32, key_index: u32) -> Vec<u8> {
        dpp::secp256k1::PublicKey::from_slice(&slot_public_key(identity_index, key_index))
            .expect("compressed slot key")
            .serialize_uncompressed()
            .to_vec()
    }

    /// No wallet slot has an id past the last hardened index, so such a
    /// key is not derived, and it doesn't fail the check of the identity's
    /// other keys whether it is disabled or enabled.
    #[test]
    fn should_not_derive_a_key_whose_id_is_past_the_last_hardened_index() {
        let past_last_hardened = 1u32 << 31;
        for disabled_at in [Some(DISABLED_AT), None] {
            let keys = [
                ecdsa_key(0, SecurityLevel::MASTER, slot_public_key(1, 0), None),
                ecdsa_key(
                    past_last_hardened,
                    SecurityLevel::HIGH,
                    slot_public_key(1, 1),
                    disabled_at,
                ),
            ];

            let check = check_all(&keys, 1);

            assert!(matches!(
                check.keys[0],
                IdentityKeyWalletCheck::Matches { .. }
            ));
            assert!(matches!(check.keys[1], IdentityKeyWalletCheck::NotDerived));
            assert_eq!(check.ownership, IdentityWalletOwnership::Owned);
        }

        // As the only master it leaves the identity undetermined, never
        // foreign.
        let keys = [ecdsa_key(
            past_last_hardened,
            SecurityLevel::MASTER,
            vec![2; 33],
            None,
        )];
        assert_eq!(
            check_all(&keys, 1).ownership,
            IdentityWalletOwnership::Undetermined
        );
    }

    /// A key the caller could not describe could be the one that is the
    /// wallet's, so it rules out NotOwned; it never undoes Owned.
    #[test]
    fn should_not_call_an_identity_foreign_beside_a_key_it_could_not_read() {
        let foreign_master = [rule_key(0, SecurityLevel::MASTER, false)];
        assert_eq!(
            identity_wallet_ownership(&foreign_master, &[a_mismatch()], 1),
            IdentityWalletOwnership::Undetermined
        );
        assert_eq!(
            identity_wallet_ownership(&foreign_master, &[a_mismatch()], 0),
            IdentityWalletOwnership::NotOwned
        );
        assert_eq!(
            identity_wallet_ownership(&foreign_master, &[a_match()], 1),
            IdentityWalletOwnership::Owned
        );

        let keys = [ecdsa_key(
            0,
            SecurityLevel::MASTER,
            slot_public_key(1, 5),
            None,
        )];
        let check = |unreadable_key_count| {
            check_identity_keys_from_master(
                &master_for(Network::Testnet),
                Network::Testnet,
                1,
                &keys,
                unreadable_key_count,
            )
            .expect("check")
            .ownership
        };
        assert_eq!(check(1), IdentityWalletOwnership::Undetermined);
        assert_eq!(check(0), IdentityWalletOwnership::NotOwned);
    }

    /// DPP compares an ECDSA key in its compressed form only, so the
    /// wallet's own master encoded uncompressed is not compared, and it
    /// can't make the identity look foreign.
    #[test]
    fn should_not_compare_an_uncompressed_ecdsa_master() {
        let uncompressed_master = ecdsa_key(
            0,
            SecurityLevel::MASTER,
            uncompressed_slot_public_key(1, 0),
            None,
        );

        assert!(matches!(
            check(&uncompressed_master, 1),
            IdentityKeyWalletCheck::NotCompared
        ));
        let check = check_all(std::slice::from_ref(&uncompressed_master), 1);
        assert!(matches!(
            check.keys[..],
            [IdentityKeyWalletCheck::NotCompared]
        ));
        assert_eq!(check.ownership, IdentityWalletOwnership::Undetermined);
    }

    #[test]
    fn should_not_call_an_identity_foreign_beside_an_uncompressed_key() {
        let keys = [
            ecdsa_key(0, SecurityLevel::MASTER, slot_public_key(1, 5), None),
            ecdsa_key(
                1,
                SecurityLevel::HIGH,
                uncompressed_slot_public_key(1, 1),
                None,
            ),
        ];

        let check = check_all(&keys, 1);

        assert!(matches!(
            check.keys[..],
            [
                IdentityKeyWalletCheck::Mismatch { .. },
                IdentityKeyWalletCheck::NotCompared
            ]
        ));
        assert_eq!(check.ownership, IdentityWalletOwnership::Undetermined);
        assert_eq!(
            identity_wallet_ownership(
                &[
                    rule_key(0, SecurityLevel::MASTER, false),
                    rule_key(1, SecurityLevel::HIGH, false)
                ],
                &[a_mismatch(), IdentityKeyWalletCheck::NotCompared],
                0
            ),
            IdentityWalletOwnership::Undetermined
        );
    }

    // ── Wallet-scoped check ───────────────────────────────────────────

    /// A seed other than the test wallet's.
    const OTHER_MNEMONIC: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";

    fn other_master() -> ExtendedPrivKey {
        let seed = Mnemonic::from_phrase(OTHER_MNEMONIC)
            .expect("valid other mnemonic")
            .to_seed("");
        ExtendedPrivKey::new_master(Network::Testnet, &*seed).expect("other master")
    }

    /// The testnet wallet `test_platform_wallet_manager` creates from
    /// [`TEST_MNEMONIC`], and its manager (keep it alive).
    async fn test_wallet() -> (
        Arc<crate::PlatformWalletManager<crate::test_support::NoopTestPersister>>,
        Arc<PlatformWallet>,
    ) {
        let (manager, wallet_id) = crate::test_support::test_platform_wallet_manager().await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("test wallet");
        (manager, wallet)
    }

    /// Checked with another wallet's seed, the wallet's own identity
    /// would look foreign, so the wallet refuses that seed instead.
    #[tokio::test]
    async fn should_refuse_another_wallets_seed_instead_of_calling_the_identity_foreign() {
        let (_manager, wallet) = test_wallet().await;
        let keys = [ecdsa_key(
            0,
            SecurityLevel::MASTER,
            slot_public_key(0, 0),
            None,
        )];
        // What the comparator alone says with the other seed.
        let unbound =
            check_identity_keys_from_master(&other_master(), Network::Testnet, 0, &keys, 0)
                .expect("comparator");
        assert_eq!(unbound.ownership, IdentityWalletOwnership::NotOwned);

        let err = wallet
            .check_identity_keys(
                0,
                &keys,
                0,
                |_| Ok::<_, PlatformWalletError>(other_master()),
            )
            .await
            .expect_err("another wallet's seed must be refused");
        assert!(
            matches!(err, PlatformWalletError::SeedMismatch { .. }),
            "{err:?}"
        );

        let check = wallet
            .check_identity_keys(0, &keys, 0, |_| {
                Ok::<_, PlatformWalletError>(master_for(Network::Testnet))
            })
            .await
            .expect("the wallet's own seed");
        assert_eq!(check.ownership, IdentityWalletOwnership::Owned);
    }

    /// Keys that need no comparison (not derived, not compared) are
    /// answered without resolving the seed at all.
    #[tokio::test]
    async fn should_answer_without_the_seed_when_no_key_needs_comparing() {
        let (_manager, wallet) = test_wallet().await;
        let keys = [
            identity_key(
                3,
                KeyType::REDPALLAS_HECATE,
                Purpose::HECATE_SIGNER,
                vec![1; 64],
            ),
            ecdsa_key(
                0,
                SecurityLevel::MASTER,
                uncompressed_slot_public_key(0, 0),
                None,
            ),
        ];
        let mut resolves = 0;

        let check = wallet
            .check_identity_keys(0, &keys, 0, |_| {
                resolves += 1;
                Ok::<_, PlatformWalletError>(other_master())
            })
            .await
            .expect("no seed needed");

        assert_eq!(resolves, 0);
        assert!(matches!(
            check.keys[..],
            [
                IdentityKeyWalletCheck::NotDerived,
                IdentityKeyWalletCheck::NotCompared
            ]
        ));
        assert_eq!(check.ownership, IdentityWalletOwnership::Undetermined);
    }

    #[tokio::test]
    async fn should_check_one_key_with_the_wallets_seed_only() {
        let (_manager, wallet) = test_wallet().await;
        let key = ecdsa_key(1, SecurityLevel::HIGH, slot_public_key(0, 1), None);

        let err = wallet
            .check_identity_key(0, &key, |_| Ok::<_, PlatformWalletError>(other_master()))
            .await
            .expect_err("another wallet's seed must be refused");
        assert!(
            matches!(err, PlatformWalletError::SeedMismatch { .. }),
            "{err:?}"
        );
        assert!(matches!(
            wallet
                .check_identity_key(0, &key, |_| {
                    Ok::<_, PlatformWalletError>(master_for(Network::Testnet))
                })
                .await
                .expect("the wallet's own seed"),
            IdentityKeyWalletCheck::Matches { .. }
        ));

        let uncompressed = ecdsa_key(
            1,
            SecurityLevel::HIGH,
            uncompressed_slot_public_key(0, 1),
            None,
        );
        assert!(matches!(
            wallet
                .check_identity_key(0, &uncompressed, |_| {
                    Err(PlatformWalletError::InvalidParameter(
                        "the seed must not be read".to_string(),
                    ))
                })
                .await
                .expect("no seed needed"),
            IdentityKeyWalletCheck::NotCompared
        ));
    }
    // ── Check after a Platform refresh ────────────────────────────────

    const PLATFORM_IDENTITY: [u8; 32] = [0x11; 32];

    fn platform_identity(keys: Vec<IdentityPublicKey>, revision: u64) -> dpp::identity::Identity {
        dpp::identity::Identity::V0(dpp::identity::v0::IdentityV0 {
            id: Identifier::from(PLATFORM_IDENTITY),
            public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
            balance: 0,
            revision,
        })
    }

    /// Where the test wallet's identity manager holds the identity.
    enum Held {
        /// At this slot of the test wallet.
        AtSlot(u32),
        /// Observed, with no wallet slot.
        OutOfWallet,
        /// At slot 1 of another wallet's bucket.
        InAnotherWallet,
    }

    /// The testnet wallet of [`TEST_MNEMONIC`], holding `stored` as given,
    /// over a mock SDK that answers the identity's fetch with
    /// `on_platform` when given.
    async fn wallet_on_platform(
        stored: Option<(dpp::identity::Identity, Held)>,
        on_platform: Option<Option<dpp::identity::Identity>>,
    ) -> (
        Arc<crate::PlatformWalletManager<crate::test_support::NoopTestPersister>>,
        Arc<PlatformWallet>,
    ) {
        struct Events;
        impl crate::events::EventHandler for Events {}
        impl crate::events::PlatformEventHandler for Events {}

        let mut sdk = dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk");
        if let Some(answer) = on_platform {
            sdk.mock()
                .expect_fetch(Identifier::from(PLATFORM_IDENTITY), answer)
                .await
                .expect("identity fetch expectation");
        }
        let manager = Arc::new(crate::PlatformWalletManager::new(
            Arc::new(sdk),
            Arc::new(crate::test_support::NoopTestPersister),
            Arc::new(Events),
        ));
        let seed = Mnemonic::from_phrase(TEST_MNEMONIC)
            .expect("valid test mnemonic")
            .to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        if let Some((stored, held)) = stored {
            let mut wm = manager.wallet_manager.write().await;
            let identity_manager = &mut wm
                .get_wallet_info_mut(&wallet.wallet_id())
                .expect("wallet info")
                .identity_manager;
            let persister = &wallet.identity().persister;
            match held {
                Held::AtSlot(slot) => {
                    identity_manager.add_identity(stored, slot, wallet.wallet_id(), persister)
                }
                Held::OutOfWallet => identity_manager.add_out_of_wallet_identity(stored, persister),
                Held::InAnotherWallet => {
                    identity_manager.add_identity(stored, 1, [0x99; 32], persister)
                }
            }
            .expect("store the identity");
        }
        (manager, wallet)
    }

    /// The stored rows still hold a foreign master that another device
    /// has since replaced: they say NotOwned, but the identity Platform
    /// returns has the wallet's master enabled, so the check after the
    /// refresh says Owned.
    #[tokio::test]
    async fn should_check_the_identity_platform_returns_not_the_stale_rows() {
        let stale_master = ecdsa_key(0, SecurityLevel::MASTER, slot_public_key(1, 9), None);
        let stored = platform_identity(vec![stale_master.clone()], 0);
        let on_platform = platform_identity(
            vec![
                ecdsa_key(
                    0,
                    SecurityLevel::MASTER,
                    slot_public_key(1, 9),
                    Some(DISABLED_AT),
                ),
                ecdsa_key(1, SecurityLevel::MASTER, slot_public_key(1, 1), None),
            ],
            1,
        );
        let (_manager, wallet) =
            wallet_on_platform(Some((stored, Held::AtSlot(1))), Some(Some(on_platform))).await;

        let stale = wallet
            .check_identity_keys(1, &[stale_master], 0, |network| {
                Ok::<_, PlatformWalletError>(master_for(network))
            })
            .await
            .expect("check the stored rows");
        assert_eq!(stale.ownership, IdentityWalletOwnership::NotOwned);

        let refreshed = wallet
            .check_identity_keys_on_platform(&Identifier::from(PLATFORM_IDENTITY), |network| {
                Ok::<_, PlatformWalletError>(master_for(network))
            })
            .await
            .expect("check after the refresh");
        assert_eq!(refreshed.ownership, IdentityWalletOwnership::Owned);
        assert!(matches!(
            refreshed.keys[..],
            [
                IdentityKeyWalletCheck::Mismatch { .. },
                IdentityKeyWalletCheck::Matches { .. }
            ]
        ));
    }

    /// An identity the wallet does not hold can't be refreshed, so the
    /// check fails, before reading the seed, instead of saying NotOwned.
    #[tokio::test]
    async fn should_refuse_an_identity_the_wallet_does_not_hold_without_the_seed() {
        let (_manager, wallet) = wallet_on_platform(None, None).await;

        let err = wallet
            .check_identity_keys_on_platform(&Identifier::from(PLATFORM_IDENTITY), |_| {
                Err(PlatformWalletError::InvalidParameter(
                    "the seed must not be read".to_string(),
                ))
            })
            .await
            .expect_err("an identity the wallet does not hold");
        assert!(
            matches!(err, PlatformWalletError::IdentityNotFound(id) if id == Identifier::from(PLATFORM_IDENTITY)),
            "{err:?}"
        );
    }

    /// An identity Platform doesn't return fails the refresh, so the check
    /// fails, before reading the seed, instead of saying NotOwned.
    #[tokio::test]
    async fn should_refuse_an_identity_platform_does_not_return() {
        let stored = platform_identity(
            vec![ecdsa_key(
                0,
                SecurityLevel::MASTER,
                slot_public_key(1, 9),
                None,
            )],
            0,
        );
        let (_manager, wallet) =
            wallet_on_platform(Some((stored, Held::AtSlot(1))), Some(None)).await;

        let err = wallet
            .check_identity_keys_on_platform(&Identifier::from(PLATFORM_IDENTITY), |_| {
                Err(PlatformWalletError::InvalidParameter(
                    "the seed must not be read".to_string(),
                ))
            })
            .await
            .expect_err("an identity Platform does not return");
        assert!(
            matches!(&err, PlatformWalletError::InvalidIdentityData(message) if message.contains("not found on Platform")),
            "{err:?}"
        );
    }

    /// The check uses the identity's own slot in the wallet, so a caller
    /// can't ask about a contradictory one: checked at slot 0, the
    /// identity at slot 1 would look foreign.
    #[tokio::test]
    async fn should_check_a_held_identity_at_its_own_wallet_slot() {
        let master = ecdsa_key(0, SecurityLevel::MASTER, slot_public_key(1, 0), None);
        let identity = platform_identity(vec![master.clone()], 0);
        let (_manager, wallet) = wallet_on_platform(
            Some((identity.clone(), Held::AtSlot(1))),
            Some(Some(identity)),
        )
        .await;

        let wrong_slot = wallet
            .check_identity_keys(0, std::slice::from_ref(&master), 0, |network| {
                Ok::<_, PlatformWalletError>(master_for(network))
            })
            .await
            .expect("check at slot 0");
        assert_eq!(wrong_slot.ownership, IdentityWalletOwnership::NotOwned);

        let check = wallet
            .check_identity_keys_on_platform(&Identifier::from(PLATFORM_IDENTITY), |network| {
                Ok::<_, PlatformWalletError>(master_for(network))
            })
            .await
            .expect("check after the refresh");
        assert_eq!(check.ownership, IdentityWalletOwnership::Owned);
        assert!(matches!(
            check.keys[..],
            [IdentityKeyWalletCheck::Matches { .. }]
        ));
    }

    /// An identity the manager holds with no slot in this wallet
    /// (observed, or another wallet's) is refused before the fetch and
    /// before the seed is read, never called foreign.
    #[tokio::test]
    async fn should_refuse_a_held_identity_with_no_wallet_slot_without_the_seed() {
        for held in [Held::OutOfWallet, Held::InAnotherWallet] {
            let identity = platform_identity(
                vec![ecdsa_key(
                    0,
                    SecurityLevel::MASTER,
                    slot_public_key(1, 9),
                    None,
                )],
                0,
            );
            // No fetch expectation: a fetch would fail with another error.
            let (_manager, wallet) = wallet_on_platform(Some((identity, held)), None).await;

            let err = wallet
                .check_identity_keys_on_platform(&Identifier::from(PLATFORM_IDENTITY), |_| {
                    Err(PlatformWalletError::InvalidParameter(
                        "the seed must not be read".to_string(),
                    ))
                })
                .await
                .expect_err("an identity with no slot in this wallet");
            assert!(
                matches!(err, PlatformWalletError::IdentityIndexNotSet(id) if id == Identifier::from(PLATFORM_IDENTITY)),
                "{err:?}"
            );
        }
    }
}
