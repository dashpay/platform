//! Identity wallet for managing Platform identities.
//!
//! Provides methods for the full identity lifecycle: registration, discovery
//! (gap-limit scan), top-up, withdrawal, and credit transfer.

use std::collections::BTreeMap;
use std::sync::Arc;

use dashcore::Address as DashAddress;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use tokio::sync::RwLock;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;
use dash_sdk::platform::transition::transfer::TransferToIdentity;
use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;

use crate::error::PlatformWalletError;
use crate::wallet::core::CoreWallet;
use crate::wallet::signer::IdentitySigner;

use super::manager::IdentityManager;

/// Default gap limit for identity discovery scanning.
const IDENTITY_GAP_LIMIT: u32 = 5;

/// Derive the 20-byte RIPEMD160(SHA256) hash of the public key at the given
/// identity authentication path.
///
/// Path format: `base_path / key_type' / identity_index' / key_index'`
/// where `base_path` is `m/9'/COIN_TYPE'/5'/0'` (mainnet or testnet).
fn derive_identity_auth_key_hash(
    wallet: &Wallet,
    network: key_wallet::Network,
    identity_index: u32,
    key_index: u32,
) -> Result<[u8; 20], PlatformWalletError> {
    use dashcore::secp256k1::Secp256k1;
    use dpp::util::hash::ripemd160_sha256;
    use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPubKey, KeyDerivationType};
    use key_wallet::dip9::{
        IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
    };

    let base_path = match network {
        key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
        _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
    };

    let key_type_index: u32 = KeyDerivationType::ECDSA.into();

    let mut full_path = DerivationPath::from(base_path);
    full_path = full_path.extend([
        ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid key type index: {}", e))
        })?,
        ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid identity index: {}", e))
        })?,
        ChildNumber::from_hardened_idx(key_index).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid key index: {}", e))
        })?,
    ]);

    let auth_key = wallet
        .derive_extended_private_key(&full_path)
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive authentication key: {}",
                e
            ))
        })?;

    let secp = Secp256k1::new();
    let public_key = ExtendedPubKey::from_priv(&secp, &auth_key);
    let public_key_bytes = public_key.public_key.serialize();
    let key_hash = ripemd160_sha256(&public_key_bytes);

    let mut key_hash_array = [0u8; 20];
    key_hash_array.copy_from_slice(&key_hash);

    Ok(key_hash_array)
}

/// Identity wallet providing identity management functionality.
#[derive(Clone)]
pub struct IdentityWallet {
    pub(crate) sdk: dash_sdk::Sdk,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    pub(crate) identity_manager: Arc<RwLock<IdentityManager>>,
    pub(crate) network: key_wallet::Network,
}

impl IdentityWallet {
    /// Create an [`IdentitySigner`] for the given identity index.
    ///
    /// The returned signer implements `Signer<IdentityPublicKey>` and derives
    /// private keys on-the-fly from the wallet using the DIP-9 identity
    /// authentication path.
    pub fn signer_for_identity(&self, identity_index: u32) -> IdentitySigner {
        IdentitySigner::new(self.wallet.clone(), self.network, identity_index)
    }
}

impl std::fmt::Debug for IdentityWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityWallet").finish()
    }
}

// ---------------------------------------------------------------------------
// Identity registration
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Register a new identity on Platform.
    ///
    /// High-level flow:
    /// 1. Build an asset lock proof via the core wallet (funds the identity).
    /// 2. Generate `key_count` identity authentication keys at DIP-9 paths
    ///    for the given `identity_index`.
    /// 3. Call the SDK's `Identity::put_to_platform_and_wait_for_response()`
    ///    to broadcast the identity-create state transition.
    /// 4. Add the new identity to the local `identity_manager`.
    ///
    /// # Arguments
    ///
    /// * `core_wallet` - The core wallet used to build the asset lock transaction.
    /// * `amount_duffs` - Amount of Dash (in duffs) to lock for the identity's
    ///   initial credit balance.
    /// * `identity_index` - BIP-9 identity index (hardened) in the key tree.
    /// * `key_count` - Number of authentication keys to register with the
    ///   identity (must be >= 1).
    pub async fn register_identity(
        &self,
        core_wallet: &CoreWallet,
        amount_duffs: u64,
        identity_index: u32,
        key_count: u32,
    ) -> Result<Identity, PlatformWalletError> {
        if key_count == 0 {
            return Err(PlatformWalletError::InvalidIdentityData(
                "key_count must be at least 1".to_string(),
            ));
        }

        // Step 1: Build and broadcast the asset lock transaction, then wait
        // for the instant-send lock proof.
        let (asset_lock_proof, asset_lock_private_key) = core_wallet
            .create_registration_asset_lock_proof(amount_duffs, identity_index)
            .await?;

        // Step 2: Derive identity authentication keys at DIP-9 paths.
        let mut keys_map: BTreeMap<u32, IdentityPublicKey> = BTreeMap::new();
        {
            use dashcore::secp256k1::Secp256k1;
            use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPubKey, KeyDerivationType};
            use key_wallet::dip9::{
                IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
            };

            let wallet = self.wallet.read().await;
            let base_path: DerivationPath = match self.network {
                key_wallet::Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
                _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
            }
            .into();

            let key_type_index: u32 = KeyDerivationType::ECDSA.into();

            let secp = Secp256k1::new();

            for key_index in 0..key_count {
                let full_path = base_path.extend([
                    ChildNumber::from_hardened_idx(key_type_index).map_err(|e| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "Invalid key type index: {}",
                            e
                        ))
                    })?,
                    ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "Invalid identity index: {}",
                            e
                        ))
                    })?,
                    ChildNumber::from_hardened_idx(key_index).map_err(|e| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "Invalid key index: {}",
                            e
                        ))
                    })?,
                ]);

                let ext_priv = wallet.derive_extended_private_key(&full_path).map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to derive authentication key: {}",
                        e
                    ))
                })?;

                let ext_pub = ExtendedPubKey::from_priv(&secp, &ext_priv);
                let compressed_pubkey = ext_pub.public_key.serialize();

                // First key is MASTER, remaining keys are HIGH.
                let security_level = if key_index == 0 {
                    SecurityLevel::MASTER
                } else {
                    SecurityLevel::HIGH
                };

                let identity_public_key =
                    IdentityPublicKey::V0(IdentityPublicKeyV0 {
                        id: key_index,
                        purpose: Purpose::AUTHENTICATION,
                        security_level,
                        contract_bounds: None,
                        key_type: KeyType::ECDSA_SECP256K1,
                        read_only: false,
                        data: BinaryData::new(compressed_pubkey.to_vec()),
                        disabled_at: None,
                    });

                keys_map.insert(key_index, identity_public_key);
            }
        }

        // Step 3: Build the Identity object and submit it to Platform.
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::default(), // SDK fills this from the asset lock
            public_keys: keys_map,
            balance: 0,
            revision: 0,
        });

        let signer = self.signer_for_identity(identity_index);

        let identity = identity
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                asset_lock_proof,
                &asset_lock_private_key,
                &signer,
                None,
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to register identity on Platform: {}",
                    e
                ))
            })?;

        // Step 4: Add the identity to the local manager (with its HD index).
        let mut manager = self.identity_manager.write().await;
        manager.add_identity(identity.clone(), identity_index)?;

        Ok(identity)
    }
}

// ---------------------------------------------------------------------------
// Identity discovery (gap-limit scan)
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Discover identities owned by this wallet via gap-limit scanning.
    ///
    /// Starting from the last scanned index stored in the identity manager,
    /// derives consecutive ECDSA authentication keys from the wallet's BIP-32
    /// tree and queries Platform for registered identities. Scanning stops
    /// after `IDENTITY_GAP_LIMIT` (5) consecutive misses.
    ///
    /// Any discovered identities are added to the local identity manager and
    /// returned. The `last_scanned_index` is updated so subsequent calls
    /// resume where this one left off.
    pub async fn sync(&self) -> Result<Vec<Identity>, PlatformWalletError> {
        use dash_sdk::platform::types::identity::PublicKeyHash;
        use dash_sdk::platform::Fetch;

        let network = {
            let wallet = self.wallet.read().await;
            wallet.network
        };

        let start_index = {
            let manager = self.identity_manager.read().await;
            manager.last_scanned_index()
        };

        let mut consecutive_misses = 0u32;
        let mut identity_index = start_index;
        let mut discovered: Vec<Identity> = Vec::new();

        while consecutive_misses < IDENTITY_GAP_LIMIT {
            // Derive the authentication key hash for this identity index
            // (key_index 0 is the primary authentication key).
            let key_hash_array = {
                let wallet = self.wallet.read().await;
                derive_identity_auth_key_hash(&wallet, network, identity_index, 0)?
            };

            // Query Platform for an identity registered with this key hash.
            // No locks are held during this network call.
            match Identity::fetch(&self.sdk, PublicKeyHash(key_hash_array)).await {
                Ok(Some(identity)) => {
                    let identity_id = identity.id();

                    // Acquire write lock only when adding an identity.
                    let mut manager = self.identity_manager.write().await;
                    if manager.identity(&identity_id).is_none() {
                        manager.add_identity(identity.clone(), identity_index)?;
                    }
                    drop(manager);

                    discovered.push(identity);
                    consecutive_misses = 0;
                }
                Ok(None) => {
                    consecutive_misses += 1;
                }
                Err(e) => {
                    // Log the error but treat it as a miss so scanning
                    // continues. A transient network error should not
                    // silently stop discovery.
                    tracing::warn!(
                        "Failed to query identity at index {}: {}",
                        identity_index,
                        e
                    );
                    consecutive_misses += 1;
                }
            }

            identity_index += 1;
        }

        // Update the last scanned index so the next sync resumes here.
        let mut manager = self.identity_manager.write().await;
        manager.set_last_scanned_index(identity_index);

        Ok(discovered)
    }
}

// ---------------------------------------------------------------------------
// Top-up
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Top up an existing identity's credit balance.
    ///
    /// Builds an asset lock transaction for the given amount and submits an
    /// `IdentityTopUpTransition` to Platform.
    ///
    /// # Arguments
    ///
    /// * `core_wallet` - The core wallet used to fund the top-up.
    /// * `identity_id` - The identifier of the identity to top up.
    /// * `topup_index` - An incrementing index distinguishing successive
    ///   top-ups for the same identity.
    /// * `amount_duffs` - Amount of Dash (in duffs) to add.
    pub async fn top_up_identity(
        &self,
        core_wallet: &CoreWallet,
        identity_id: &Identifier,
        topup_index: u32,
        amount_duffs: u64,
    ) -> Result<(), PlatformWalletError> {
        // Retrieve the identity and its HD index from the manager.
        let (identity, identity_index) = {
            let manager = self.identity_manager.read().await;
            let identity = manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager.identity_index(identity_id).ok_or(
                PlatformWalletError::IdentityIndexNotSet(*identity_id),
            )?;
            (identity, index)
        };

        // Step 1: Build and broadcast the top-up asset lock transaction,
        // then wait for the instant-send lock proof.
        let (asset_lock_proof, asset_lock_private_key) = core_wallet
            .create_topup_asset_lock_proof(amount_duffs, identity_index, topup_index)
            .await?;

        // Step 2: Submit the top-up state transition.
        let new_balance = identity
            .top_up_identity(
                &self.sdk,
                asset_lock_proof,
                &asset_lock_private_key,
                None, // user_fee_increase
                None, // settings
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to top up identity: {}",
                    e
                ))
            })?;

        // Update the identity's balance in the local manager.
        {
            let mut manager = self.identity_manager.write().await;
            if let Some(identity) = manager.identity_mut(identity_id) {
                identity.set_balance(new_balance);
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Withdrawal
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Withdraw credits from an identity to a Dash address.
    ///
    /// Submits an `IdentityCreditWithdrawalTransition` to Platform that moves
    /// the specified amount (in platform credits) from the identity back to
    /// a Core chain address.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identifier of the identity to withdraw from.
    /// * `amount` - Amount of credits to withdraw.
    /// * `to_address` - The Dash P2PKH address to receive the withdrawal.
    pub async fn withdraw_credits(
        &self,
        identity_id: &Identifier,
        amount: u64,
        to_address: &DashAddress,
    ) -> Result<(), PlatformWalletError> {
        // Retrieve the identity and its HD index from the manager.
        let (identity, identity_index) = {
            let manager = self.identity_manager.read().await;
            let identity = manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager.identity_index(identity_id).ok_or(
                PlatformWalletError::IdentityIndexNotSet(*identity_id),
            )?;
            (identity, index)
        };

        let signer = self.signer_for_identity(identity_index);

        let new_balance = identity
            .withdraw(
                &self.sdk,
                Some(to_address.clone()),
                amount,
                None, // core_fee_per_byte
                None, // signing_withdrawal_key_to_use
                signer,
                None, // settings
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to withdraw credits: {}",
                    e
                ))
            })?;

        // Update the identity's balance in the local manager.
        {
            let mut manager = self.identity_manager.write().await;
            if let Some(identity) = manager.identity_mut(identity_id) {
                identity.set_balance(new_balance);
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Credit transfer
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Transfer credits from one identity to another.
    ///
    /// Submits an `IdentityCreditTransferTransition` to Platform that moves
    /// `amount` credits from `from_id` to `to_id`.
    ///
    /// # Arguments
    ///
    /// * `from_id` - The identifier of the sending identity (must be owned
    ///   by this wallet).
    /// * `to_id` - The identifier of the receiving identity.
    /// * `amount` - Amount of credits to transfer.
    pub async fn transfer_credits(
        &self,
        from_id: &Identifier,
        to_id: &Identifier,
        amount: u64,
    ) -> Result<(), PlatformWalletError> {
        // Retrieve the sending identity and its HD index from the manager.
        let (identity, identity_index) = {
            let manager = self.identity_manager.read().await;
            let identity = manager
                .identity(from_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*from_id))?;
            let index = manager.identity_index(from_id).ok_or(
                PlatformWalletError::IdentityIndexNotSet(*from_id),
            )?;
            (identity, index)
        };

        let signer = self.signer_for_identity(identity_index);

        let (sender_balance, _receiver_balance) = identity
            .transfer_credits(
                &self.sdk,
                *to_id,
                amount,
                None, // signing_transfer_key_to_use
                signer,
                None, // settings
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to transfer credits: {}",
                    e
                ))
            })?;

        // Update the sender's balance in the local manager.
        {
            let mut manager = self.identity_manager.write().await;
            if let Some(identity) = manager.identity_mut(from_id) {
                identity.set_balance(sender_balance);
            }
        }

        Ok(())
    }
}
