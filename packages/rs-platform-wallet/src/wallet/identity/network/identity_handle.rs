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

use dashcore::secp256k1::PublicKey;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType};
use dpp::prelude::Identifier;
use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, KeyDerivationType};
use key_wallet::dip9::{
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
    use dashcore::secp256k1::Secp256k1;
    use key_wallet::bip32::ExtendedPubKey;

    let path = identity_auth_derivation_path_for_type(
        network,
        KeyDerivationType::ECDSA,
        identity_index,
        key_index,
    )?;
    let secp = Secp256k1::new();
    // `ExtendedPrivKey` doesn't implement `Zeroize`, so we can't
    // wrap it in `Zeroizing` directly — but its inner
    // `secp256k1::SecretKey` does implement `Drop` with a memzero,
    // so the secret scalar is scrubbed when `derived` falls out of
    // scope. The surrounding `chain_code` / `depth` /
    // `parent_fingerprint` / `child_number` are non-secret BIP-32
    // metadata; leaking them on the stack is a non-event. The
    // returned `private_key` is wrapped in `Zeroizing` below so
    // the 32-byte scalar copy crossing the function boundary is
    // also scrubbed on the caller's drop.
    let derived = master.derive_priv(&secp, &path).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to derive private key at (identity={identity_index}, key={key_index}): {e}"
        ))
    })?;
    let extended_pub = ExtendedPubKey::from_priv(&secp, &derived);

    Ok(DerivedIdentityAuthKey {
        derivation_path: path,
        private_key: Zeroizing::new(derived.private_key.secret_bytes()),
        public_key: extended_pub.public_key.serialize(),
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
    use dashcore::secp256k1::Secp256k1;
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

    let secp = Secp256k1::new();
    let extended_pub = ExtendedPubKey::from_priv(&secp, &auth_key);
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
    let public_key_bytes = public_key.serialize();
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

        Ok(Zeroizing::new(secret_key.secret_bytes()))
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
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC)
            .expect("valid English test mnemonic");
        Wallet::from_mnemonic(mnemonic, network, WalletAccountCreationOptions::None)
            .expect("from_mnemonic should build a Mnemonic wallet")
    }

    /// The BIP-32 master node for [`TEST_MNEMONIC`] on `network` — the
    /// same node `derive_extended_private_key` reconstructs internally
    /// (`RootExtendedPrivKey::new_master(seed).to_extended_priv_key(network)`
    /// is byte-for-byte `ExtendedPrivKey::new_master(network, seed)`).
    fn master_for(network: Network) -> ExtendedPrivKey {
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC)
            .expect("valid English test mnemonic");
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
}
