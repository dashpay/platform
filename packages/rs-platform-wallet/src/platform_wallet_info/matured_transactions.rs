//! Processing asset lock transactions for identity registration detection
//!
//! This module handles the detection and fetching of identities created from
//! asset lock transactions.

use super::PlatformWalletInfo;
use crate::error::PlatformWalletError;
#[allow(unused_imports)]
use crate::ContactRequest;
use dpp::prelude::Identifier;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::Network;

use dpp::identity::accessors::IdentityGettersV0;

impl PlatformWalletInfo {
    /// Discover identity and fetch contact requests for a single asset lock transaction
    ///
    /// This is called automatically when an asset lock transaction is detected.
    ///
    /// # Arguments
    ///
    /// * `wallet` - The wallet to derive authentication keys from
    /// * `tx` - The asset lock transaction
    ///
    /// # Returns
    ///
    /// Returns Ok(Some(identity_id)) if found, Ok(None) if not found
    pub async fn fetch_identity_and_contacts_for_asset_lock(
        &mut self,
        wallet: &key_wallet::Wallet,
        tx: &dashcore::Transaction,
    ) -> Result<Option<Identifier>, PlatformWalletError> {
        let result = self
            .fetch_contact_requests_for_identities_after_asset_locks(
                wallet,
                std::slice::from_ref(tx),
            )
            .await?;

        Ok(result.first().copied())
    }

    /// Discover identities and fetch contact requests after asset locks
    ///
    /// When asset lock transactions are seen (added as immature), identities may have been registered.
    /// This searches for the first identity key to discover newly registered identities
    /// and fetches their DashPay contact requests.
    ///
    /// # Arguments
    ///
    /// * `wallet` - The wallet to derive authentication keys from
    /// * `asset_lock_transactions` - List of asset lock transactions
    ///
    /// # Returns
    ///
    /// Returns a list of identity IDs for which contact requests were fetched
    pub async fn fetch_contact_requests_for_identities_after_asset_locks(
        &mut self,
        wallet: &key_wallet::Wallet,
        asset_lock_transactions: &[dashcore::Transaction],
    ) -> Result<Vec<Identifier>, PlatformWalletError> {
        use dash_sdk::platform::types::identity::PublicKeyHash;
        use dash_sdk::platform::Fetch;
        use dpp::util::hash::ripemd160_sha256;

        let mut identities_processed = Vec::new();

        // Early return if no asset lock transactions
        if asset_lock_transactions.is_empty() {
            return Ok(identities_processed);
        }

        // Get SDK from identity manager
        let sdk = self
            .identity_manager()
            .sdk
            .as_ref()
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "SDK not configured in identity manager".to_string(),
                )
            })?
            .clone();

        // Derive the first authentication key (identity_index 0, key_index 0)
        let identity_index = 0u32;
        let key_index = 0u32;

        // Build identity authentication derivation path
        // Path format: m/9'/COIN_TYPE'/5'/0'/identity_index'/key_index'
        use key_wallet::bip32::{ChildNumber, DerivationPath};
        use key_wallet::dip9::{
            IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
        };

        let base_path = match self.network() {
            Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
            Network::Testnet => IDENTITY_AUTHENTICATION_PATH_TESTNET,
            _ => {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "Unsupported network for identity derivation".to_string(),
                ));
            }
        };

        // Create full derivation path: base path + identity_index' + key_index'
        let mut full_path = DerivationPath::from(base_path);
        full_path = full_path.extend([
            ChildNumber::from_hardened_idx(identity_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid identity index: {}", e))
            })?,
            ChildNumber::from_hardened_idx(key_index).map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("Invalid key index: {}", e))
            })?,
        ]);

        // Derive the extended private key at this path
        let auth_key = wallet
            .derive_extended_private_key(&full_path)
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to derive authentication key: {}",
                    e
                ))
            })?;

        // Get public key bytes and hash them
        use dashcore::secp256k1::Secp256k1;
        use key_wallet::bip32::ExtendedPubKey;
        let secp = Secp256k1::new();
        let public_key = ExtendedPubKey::from_priv(&secp, &auth_key);
        let public_key_bytes = public_key.public_key.serialize();
        let key_hash = ripemd160_sha256(&public_key_bytes);

        // Create a fixed-size array from the hash
        let mut key_hash_array = [0u8; 20];
        key_hash_array.copy_from_slice(&key_hash);

        // Query Platform for identity by public key hash
        match dpp::identity::Identity::fetch(&sdk, PublicKeyHash(key_hash_array)).await {
            Ok(Some(identity)) => {
                let identity_id = identity.id();

                // Add identity to manager if not already present
                if !self
                    .identity_manager()
                    .identities()
                    .contains_key(&identity_id)
                {
                    self.identity_manager_mut().add_identity(identity.clone())?;
                }

                // Fetch DashPay contact requests for this identity
                match sdk
                    .fetch_all_contact_requests_for_identity(&identity, Some(100))
                    .await
                {
                    Ok((sent_docs, received_docs)) => {
                        // Process sent contact requests
                        for (_doc_id, maybe_doc) in sent_docs {
                            if let Some(doc) = maybe_doc {
                                if let Ok(contact_request) = parse_contact_request_document(&doc) {
                                    // Add to managed identity
                                    if let Some(managed_identity) = self
                                        .identity_manager_mut()
                                        .managed_identity_mut(&identity_id)
                                    {
                                        managed_identity.add_sent_contact_request(contact_request);
                                    }
                                }
                            }
                        }

                        // Process received contact requests
                        for (_doc_id, maybe_doc) in received_docs {
                            if let Some(doc) = maybe_doc {
                                if let Ok(contact_request) = parse_contact_request_document(&doc) {
                                    // Add to managed identity
                                    if let Some(managed_identity) = self
                                        .identity_manager_mut()
                                        .managed_identity_mut(&identity_id)
                                    {
                                        managed_identity
                                            .add_incoming_contact_request(contact_request);
                                    }
                                }
                            }
                        }

                        identities_processed.push(identity_id);
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to fetch contact requests for identity {}: {}",
                            identity_id, e
                        );
                    }
                }
            }
            Ok(None) => {
                // No identity found for this key - that's ok, may not be registered yet
            }
            Err(e) => {
                eprintln!("Failed to query identity by public key hash: {}", e);
            }
        }

        Ok(identities_processed)
    }
}

/// Parse a contact request document into a ContactRequest struct
fn parse_contact_request_document(
    doc: &dpp::document::Document,
) -> Result<ContactRequest, PlatformWalletError> {
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::Value;

    // Extract fields from the document
    let properties = doc.properties();

    let to_user_id = properties
        .get("toUserId")
        .and_then(|v| match v {
            Value::Identifier(id) => Some(Identifier::from(*id)),
            _ => None,
        })
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Missing or invalid toUserId in contact request".to_string(),
            )
        })?;

    let sender_key_index = properties
        .get("senderKeyIndex")
        .and_then(|v| match v {
            Value::U32(i) => Some(*i),
            _ => None,
        })
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Missing or invalid senderKeyIndex in contact request".to_string(),
            )
        })?;

    let recipient_key_index = properties
        .get("recipientKeyIndex")
        .and_then(|v| match v {
            Value::U32(i) => Some(*i),
            _ => None,
        })
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Missing or invalid recipientKeyIndex in contact request".to_string(),
            )
        })?;

    let account_reference = properties
        .get("accountReference")
        .and_then(|v| match v {
            Value::U32(i) => Some(*i),
            _ => None,
        })
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Missing or invalid accountReference in contact request".to_string(),
            )
        })?;

    let encrypted_public_key = properties
        .get("encryptedPublicKey")
        .and_then(|v| match v {
            Value::Bytes(b) => Some(b.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Missing or invalid encryptedPublicKey in contact request".to_string(),
            )
        })?;

    let created_at_core_block_height = doc.created_at_core_block_height().unwrap_or(0);

    let created_at = doc.created_at().unwrap_or(0);

    let sender_id = doc.owner_id();

    Ok(ContactRequest::new(
        sender_id,
        to_user_id,
        sender_key_index,
        recipient_key_index,
        account_reference,
        encrypted_public_key,
        created_at_core_block_height,
        created_at,
    ))
}
