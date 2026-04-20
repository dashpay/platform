//! DashPay wallet for contact requests and payments.
//!
//! Provides methods for the DashPay contact lifecycle: sending contact
//! requests, syncing incoming requests from the platform, accepting
//! incoming requests (establishing contacts), and listing established contacts.

use std::sync::Arc;

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType};
use key_wallet::wallet::Wallet;
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::signer::IdentitySigner;

/// DashPay wallet providing contact request and payment functionality.
///
/// Shares the same `WalletManager` lock as all other sub-wallets.
///
/// `B` is the concrete broadcaster type; every `send_payment` call
/// dispatches statically instead of through a `dyn` vtable.
pub struct DashPayWallet<B: TransactionBroadcaster + ?Sized> {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// The shared wallet manager lock for all mutable wallet state.
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet within the manager this sub-wallet operates on.
    pub(crate) wallet_id: WalletId,
    /// Per-wallet persistence handle for queuing changesets.
    pub(crate) persister: crate::wallet::persister::WalletPersister,
    /// Transaction broadcaster (SPV or DAPI) for `send_payment`.
    pub(crate) broadcaster: Arc<B>,
}

impl<B: TransactionBroadcaster + ?Sized> std::fmt::Debug for DashPayWallet<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DashPayWallet").finish()
    }
}

// Manual `Clone` impl: the derive would add a `where B: Clone`
// bound, but `Arc<B>` clones without cloning `B` itself, so we
// don't want that bound. `B: ?Sized` is enough.
impl<B: TransactionBroadcaster + ?Sized> Clone for DashPayWallet<B> {
    fn clone(&self) -> Self {
        Self {
            sdk: Arc::clone(&self.sdk),
            wallet_manager: Arc::clone(&self.wallet_manager),
            wallet_id: self.wallet_id,
            persister: self.persister.clone(),
            broadcaster: Arc::clone(&self.broadcaster),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayWallet<B> {
    /// Derive the ECDH private key for the given identity's encryption key.
    ///
    /// Uses the same DIP-9 derivation as `IdentitySigner` but returns the raw
    /// `secp256k1::SecretKey` needed for ECDH.
    ///
    /// The encryption key must be of type ECDSA_SECP256K1 or ECDSA_HASH160;
    /// other key types are not supported for ECDH derivation.
    fn derive_encryption_private_key(
        wallet: &Wallet,
        network: key_wallet::Network,
        identity_index: u32,
        encryption_key: &IdentityPublicKey,
    ) -> Result<dashcore::secp256k1::SecretKey, PlatformWalletError> {
        use key_wallet::bip32::{ChildNumber, DerivationPath, KeyDerivationType};
        use key_wallet::dip9::{
            IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
        };

        // Validate that the encryption key type is compatible with ECDH derivation.
        match encryption_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {}
            other => {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Unsupported key type {:?} for ECDH derivation; expected ECDSA_SECP256K1 or ECDSA_HASH160",
                    other
                )));
            }
        }

        let base_path: DerivationPath = match network {
            key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
            _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
        }
        .into();

        let key_type_index: u32 = KeyDerivationType::ECDSA.into();

        let full_path = base_path.extend([
            ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid key type index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid identity index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(encryption_key.id()).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid key ID: {}", e))
            })?,
        ]);

        let ext_priv = wallet
            .derive_extended_private_key(&full_path)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to derive encryption private key: {}",
                    e
                ))
            })?;

        // Wrap intermediate private key bytes in Zeroizing so they are wiped on drop.
        let secret_bytes = zeroize::Zeroizing::new(ext_priv.private_key.secret_bytes());

        dashcore::secp256k1::SecretKey::from_slice(&*secret_bytes).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Invalid derived encryption private key: {}",
                e
            ))
        })
    }
}

// ---------------------------------------------------------------------------
// Comprehensive DashPay sync
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayWallet<B> {
    /// Comprehensive DashPay sync: contact requests followed by profiles.
    ///
    /// Call this on wallet open and on periodic refresh. Failures in either
    /// step are propagated immediately; partial progress is not rolled back.
    pub async fn sync(&self) -> Result<(), PlatformWalletError> {
        // Contact requests first — may establish new contacts.
        self.sync_contact_requests().await?;
        // Then profiles for all managed identities.
        self.sync_profiles().await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Sub-modules — each groups a coherent set of DashPay operations. See the
// per-file doc comment for what lives where.
// ---------------------------------------------------------------------------

mod account_labels;
mod contact_requests;
mod contacts;
mod payments;
mod profile;
