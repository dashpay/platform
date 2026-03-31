//! Identity wallet for managing Platform identities.
//!
//! Provides methods for the full identity lifecycle: registration, discovery
//! (gap-limit scan), top-up, withdrawal, and credit transfer.

use std::collections::BTreeMap;
use std::sync::Arc;

use dashcore::Address as DashAddress;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use tokio::sync::RwLock;

use dpp::identity::signer::Signer;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;
use dash_sdk::platform::transition::top_up_identity_from_addresses::TopUpIdentityFromAddresses;
use dash_sdk::platform::transition::transfer::TransferToIdentity;
use dash_sdk::platform::transition::transfer_to_addresses::TransferToAddresses;
use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

use crate::error::PlatformWalletError;
use crate::wallet::core::CoreWallet;
use crate::wallet::platform_addresses::PlatformAddressWallet;
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

    /// Withdraw credits using an externally-provided identity and signer.
    ///
    /// Unlike [`withdraw_credits`](Self::withdraw_credits), this method does
    /// **not** look up the identity in the internal `IdentityManager`. Instead,
    /// the caller supplies the `Identity` object and a `Signer` implementation
    /// directly. This is useful when the caller manages identities outside of
    /// the platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the remaining credit balance after the withdrawal.
    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw_credits_with_signer<S: Signer<IdentityPublicKey> + Send>(
        &self,
        identity: &Identity,
        to_address: Option<DashAddress>,
        amount: u64,
        signing_withdrawal_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
    ) -> Result<u64, dash_sdk::Error> {
        identity
            .withdraw(
                &self.sdk,
                to_address,
                amount,
                Some(1), // core_fee_per_byte
                signing_withdrawal_key_to_use,
                signer,
                None, // settings
            )
            .await
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

    /// Transfer credits using an externally-provided identity and signer.
    ///
    /// Unlike [`transfer_credits`](Self::transfer_credits), this method does
    /// **not** look up the identity in the internal `IdentityManager`. The
    /// caller supplies the `Identity` and a `Signer` directly.
    ///
    /// Returns `(sender_balance, receiver_balance)` after the transfer.
    pub async fn transfer_credits_with_signer<S: Signer<IdentityPublicKey> + Send>(
        &self,
        identity: &Identity,
        to_id: Identifier,
        amount: u64,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
    ) -> Result<(u64, u64), dash_sdk::Error> {
        identity
            .transfer_credits(
                &self.sdk,
                to_id,
                amount,
                signing_transfer_key_to_use,
                signer,
                None, // settings
            )
            .await
    }
}

// ---------------------------------------------------------------------------
// Identity update (add/disable keys)
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Update an identity by adding or disabling public keys.
    ///
    /// Builds an `IdentityUpdateTransition`, signs it with the identity's
    /// master key, and broadcasts it to Platform.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to update.
    /// * `add_public_keys` - New keys to add (key IDs are auto-assigned).
    /// * `disable_public_keys` - Key IDs to disable.
    pub async fn update_identity(
        &self,
        identity_id: &Identifier,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<u32>,
    ) -> Result<(), PlatformWalletError> {
        use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
        use dpp::state_transition::proof_result::StateTransitionProofResult;
        use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;

        let (mut identity, identity_index) = {
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

        // Increment revision for the update transition.
        let original_revision = identity.revision();
        identity.set_revision(original_revision + 1);

        // Find a master key that the signer can use.
        let signer = self.signer_for_identity(identity_index);

        let master_key_id = identity
            .public_keys()
            .iter()
            .find(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION
                    && key.security_level() == SecurityLevel::MASTER
                    && key.key_type() == KeyType::ECDSA_SECP256K1
            })
            .map(|(id, _)| *id)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "No signable master key found on identity".to_string(),
                )
            })?;

        // Get identity nonce from Platform.
        let identity_nonce = self
            .sdk
            .get_identity_nonce(identity.id(), true, None)
            .await?;

        // Build the update transition.
        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            &identity,
            &master_key_id,
            add_public_keys,
            disable_public_keys,
            identity_nonce,
            0, // user_fee_increase
            &signer,
            self.sdk.version(),
            None,
        )
        .map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to create identity update transition: {}",
                e
            ))
        })?;

        // Broadcast and wait for confirmation.
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&self.sdk, None)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to broadcast identity update: {}",
                    e
                ))
            })?;

        Ok(())
    }

    /// Update an identity using an externally-provided identity and signer.
    ///
    /// Unlike [`update_identity`](Self::update_identity), this method does
    /// **not** look up the identity in the internal `IdentityManager`. The
    /// caller supplies the `Identity`, master key ID, and a `Signer` directly.
    ///
    /// Returns the [`StateTransitionProofResult`] from the broadcast so callers
    /// can inspect proof-verified outcomes (e.g. updated keys, balance).
    pub async fn update_identity_with_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity: &Identity,
        master_key_id: &u32,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<u32>,
        signer: &S,
    ) -> Result<dpp::state_transition::proof_result::StateTransitionProofResult, dash_sdk::Error>
    {
        use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
        use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
        use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;

        // Get identity nonce from Platform.
        let identity_nonce = self
            .sdk
            .get_identity_nonce(identity.id(), true, None)
            .await?;

        // Build the update transition.
        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            identity,
            master_key_id,
            add_public_keys,
            disable_public_keys,
            identity_nonce,
            0, // user_fee_increase
            signer,
            self.sdk.version(),
            None,
        )
        .map_err(|e| dash_sdk::Error::Protocol(e))?;

        // Broadcast and wait for confirmation.
        let result = state_transition
            .broadcast_and_wait(&self.sdk, None)
            .await?;

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Top-up from platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Top up an identity by spending platform address balances.
    ///
    /// Uses the `TopUpIdentityFromAddresses` SDK trait. Address nonces are
    /// looked up automatically.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to top up.
    /// * `inputs` - Map of platform addresses to credit amounts to spend.
    /// * `platform_address_wallet` - The platform address wallet (provides signing).
    pub async fn top_up_from_addresses(
        &self,
        identity_id: &Identifier,
        inputs: BTreeMap<PlatformAddress, Credits>,
        platform_address_wallet: &PlatformAddressWallet,
    ) -> Result<Credits, PlatformWalletError> {
        let identity = {
            let manager = self.identity_manager.read().await;
            manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
        };

        let (_address_infos, new_balance) = identity
            .top_up_from_addresses(
                &self.sdk,
                inputs,
                platform_address_wallet,
                None, // settings
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to top up identity from addresses: {}",
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

        Ok(new_balance)
    }
}

// ---------------------------------------------------------------------------
// Transfer credits to platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Transfer credits from an identity to multiple platform addresses.
    ///
    /// Uses the `TransferToAddresses` SDK trait.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The sending identity (must be owned by this wallet).
    /// * `recipient_addresses` - Map of platform addresses to credit amounts.
    pub async fn transfer_credits_to_addresses(
        &self,
        identity_id: &Identifier,
        recipient_addresses: BTreeMap<PlatformAddress, Credits>,
    ) -> Result<Credits, PlatformWalletError> {
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

        let (_address_infos, new_balance) = identity
            .transfer_credits_to_addresses(
                &self.sdk,
                recipient_addresses,
                None, // signing_transfer_key_to_use
                &signer,
                None, // settings
            )
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to transfer credits to addresses: {}",
                    e
                ))
            })?;

        // Update the sender's balance in the local manager.
        {
            let mut manager = self.identity_manager.write().await;
            if let Some(identity) = manager.identity_mut(identity_id) {
                identity.set_balance(new_balance);
            }
        }

        Ok(new_balance)
    }
}

// ---------------------------------------------------------------------------
// DPNS name operations
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Register a DPNS name for an identity.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to register the name for.
    /// * `name` - The desired username label (e.g., "alice").
    pub async fn register_name(
        &self,
        identity_id: &Identifier,
        name: &str,
    ) -> Result<String, PlatformWalletError> {
        use dash_sdk::platform::dpns_usernames::RegisterDpnsNameInput;

        let (identity, identity_index, auth_key) = {
            let manager = self.identity_manager.read().await;
            let identity = manager
                .identity(identity_id)
                .cloned()
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager.identity_index(identity_id).ok_or(
                PlatformWalletError::IdentityIndexNotSet(*identity_id),
            )?;
            // Use the first authentication key (key_id 0).
            let key = identity
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    [SecurityLevel::MASTER, SecurityLevel::HIGH].into(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "No authentication key found on identity".to_string(),
                    )
                })?
                .clone();
            (identity, index, key)
        };

        let signer = self.signer_for_identity(identity_index);

        let input = RegisterDpnsNameInput {
            label: name.to_string(),
            identity,
            identity_public_key: auth_key,
            signer,
            preorder_callback: None,
        };

        let result = self.sdk.register_dpns_name(input).await.map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to register DPNS name '{}': {}",
                name, e
            ))
        })?;

        Ok(result.full_domain_name)
    }

    /// Register a DPNS name using an externally-provided identity and signer.
    ///
    /// Unlike [`register_name`](Self::register_name), this method does **not**
    /// look up the identity in the internal `IdentityManager`. The caller
    /// supplies the `Identity`, the signing key, and a `Signer` directly.
    ///
    /// Returns the full domain name (e.g. "alice.dash").
    pub async fn register_name_with_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity: Identity,
        name: &str,
        identity_public_key: IdentityPublicKey,
        signer: S,
    ) -> Result<String, dash_sdk::Error> {
        use dash_sdk::platform::dpns_usernames::RegisterDpnsNameInput;

        let input = RegisterDpnsNameInput {
            label: name.to_string(),
            identity,
            identity_public_key,
            signer,
            preorder_callback: None,
        };

        let result = self.sdk.register_dpns_name(input).await?;
        Ok(result.full_domain_name)
    }

    /// Resolve a DPNS name to an identity identifier.
    ///
    /// Accepts both "alice" and "alice.dash" formats.
    pub async fn resolve_name(
        &self,
        name: &str,
    ) -> Result<Option<Identifier>, PlatformWalletError> {
        self.sdk
            .resolve_dpns_name(name)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to resolve DPNS name '{}': {}",
                    name, e
                ))
            })
    }

    /// Search for DPNS names by prefix.
    pub async fn search_names(
        &self,
        prefix: &str,
        limit: Option<u32>,
    ) -> Result<Vec<dash_sdk::platform::dpns_usernames::DpnsUsername>, PlatformWalletError> {
        self.sdk
            .search_dpns_names(prefix, limit)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to search DPNS names with prefix '{}': {}",
                    prefix, e
                ))
            })
    }
}
