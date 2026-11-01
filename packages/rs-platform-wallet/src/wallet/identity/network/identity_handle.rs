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
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType};
use dpp::prelude::Identifier;
use key_wallet::bip32::{
    ApplicationKeyPurpose, ChildNumber, DerivationPath, ExtendedPrivKey, KeyDerivationType,
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
use zeroize::Zeroizing;

use crate::broadcaster::{SpvBroadcaster, TransactionBroadcaster};
use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::manager::AssetLockManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

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
            } => {
                let rotation = hardened_rotation(rotation)?;
                // Temporary: the pinned rust-dashcore builds only the
                // pre-rotation path, so the rotation level is appended here.
                // dashpay/rust-dashcore#1250 adds the level to key-wallet;
                // once platform bumps to a rev with it, replace this arm with
                // `DerivationPath::application_encryption_path(network,
                // identity_id, contract_id, purpose, rotation)` and the
                // legacy arm with `application_encryption_path_without_rotation`.
                // `connect_keypairs_match_the_pinned_vectors` keeps that swap
                // a no-op.
                Ok(DerivationPath::application_encryption_path(
                    network,
                    identity_id.to_buffer(),
                    contract_id,
                    purpose,
                )
                .extend([rotation]))
            }
            ConnectKey::AppEncryptionLegacy {
                identity_id,
                contract_id,
                purpose,
            } => Ok(DerivationPath::application_encryption_path(
                network,
                identity_id.to_buffer(),
                contract_id,
                purpose,
            )),
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
    ChildNumber::from_hardened_idx(rotation).map_err(|e| {
        PlatformWalletError::InvalidParameter(format!(
            "Invalid app encryption rotation {rotation}: {e}"
        ))
    })
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
    /// Orders moves of a wallet identity between slots (discovery, loading
    /// and `PlatformWallet::apply` take it for writing) against operations
    /// that act on the account a slot selects (tip preparation, tip sends and
    /// tip address publication take it for reading). Taken before
    /// `wallet_manager`.
    pub(crate) identity_placement: Arc<tokio::sync::RwLock<()>>,
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
            identity_placement: Arc::clone(&self.identity_placement),
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
    /// Determines the correct [`KeyDerivationType`] from the public key's
    /// [`KeyType`], builds the DIP-9 derivation path, and derives the
    /// private key from the wallet.
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
        let key_derivation_type = match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => KeyDerivationType::ECDSA,
            KeyType::BLS12_381 => KeyDerivationType::BLS,
            // EdDSA uses the ECDSA derivation path; the raw bytes are
            // reinterpreted as an Ed25519 seed.
            KeyType::EDDSA_25519_HASH160 => KeyDerivationType::ECDSA,
            KeyType::BIP13_SCRIPT_HASH => {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "BIP13_SCRIPT_HASH keys are not supported for signing".to_string(),
                ));
            }
        };

        let path = Self::identity_auth_derivation_path(
            network,
            key_derivation_type,
            identity_index,
            key_id,
        )?;

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
}

#[cfg(test)]
mod tests {
    use super::*;
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
        ExtendedPrivKey::new_master(network, &seed).expect("master xpriv from test seed")
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
}
