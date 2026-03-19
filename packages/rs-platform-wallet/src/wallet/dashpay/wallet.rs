//! DashPay wallet for contact requests and payments.
//!
//! Provides methods for the DashPay contact lifecycle: sending contact
//! requests, syncing incoming requests from the platform, accepting
//! incoming requests (establishing contacts), and listing established contacts.

use std::sync::Arc;

use dpp::document::DocumentV0Getters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::Purpose;
use dpp::identity::{Identity, IdentityPublicKey, KeyType};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use key_wallet::account::AccountType;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use tokio::sync::RwLock;

use dash_sdk::platform::dashpay::{EcdhProvider, SendContactRequestInput};

use crate::error::PlatformWalletError;
use crate::wallet::dashpay::contact_request::ContactRequest;
use crate::wallet::dashpay::established_contact::EstablishedContact;
use crate::wallet::identity::IdentityManager;
use crate::wallet::signer::IdentitySigner;

/// DashPay wallet providing contact request and payment functionality.
///
/// Shares the same `identity_manager` Arc as `IdentityWallet`.
#[derive(Clone)]
pub struct DashPayWallet {
    pub(crate) sdk: dash_sdk::Sdk,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    pub(crate) identity_manager: Arc<RwLock<IdentityManager>>,
    pub(crate) network: key_wallet::Network,
}

impl std::fmt::Debug for DashPayWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DashPayWallet").finish()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

impl DashPayWallet {
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
// Send contact request
// ---------------------------------------------------------------------------

impl DashPayWallet {
    /// Send a contact request to another identity.
    ///
    /// All parameters that can be resolved internally are resolved automatically:
    /// - **identity_index**: looked up from the local `ManagedIdentity`
    /// - **sender_key_index**: first key with `Purpose::ENCRYPTION` on the sender
    /// - **recipient_key_index**: first key with `Purpose::DECRYPTION` on the recipient
    /// - **account_index**: defaults to `0`
    /// - **ECDH**: performed SDK-side using the sender's derived encryption private key
    ///
    /// # Arguments
    ///
    /// * `sender_identity_id`    - Identity that owns the contact request.
    /// * `recipient_identity_id` - Identity the request is sent to.
    pub async fn send_contact_request(
        &self,
        sender_identity_id: &Identifier,
        recipient_identity_id: &Identifier,
    ) -> Result<(), PlatformWalletError> {
        // 1. Retrieve the sender identity and its HD index from the local manager
        //    via a single managed_identity() call.
        let (sender_identity, identity_index) = {
            let manager = self.identity_manager.read().await;
            let managed = manager
                .managed_identity(sender_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*sender_identity_id))?;
            let index = Some(managed.identity_index).ok_or(
                PlatformWalletError::IdentityIndexNotSet(*sender_identity_id),
            )?;
            (managed.identity.clone(), index)
        };

        // 2. Fetch the recipient identity from Platform.
        let recipient_identity = {
            use dash_sdk::platform::Fetch;
            Identity::fetch(&self.sdk, *recipient_identity_id)
                .await
                .map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to fetch recipient identity: {}",
                        e
                    ))
                })?
                .ok_or_else(|| PlatformWalletError::IdentityNotFound(*recipient_identity_id))?
        };

        // 3. Resolve key indices — sender ENCRYPTION, recipient DECRYPTION.
        let sender_encryption_key = sender_identity
            .public_keys()
            .iter()
            .find(|(_, k)| k.purpose() == Purpose::ENCRYPTION)
            .map(|(_, k)| k.clone())
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Sender identity has no encryption key".to_string(),
                )
            })?;
        let sender_key_index = sender_encryption_key.id();

        let recipient_key_index = recipient_identity
            .public_keys()
            .iter()
            .find(|(_, k)| k.purpose() == Purpose::DECRYPTION)
            .map(|(id, _)| *id)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Recipient identity has no decryption key".to_string(),
                )
            })?;

        // 4. Derive both the DashPay receiving-account xpub and the ECDH
        //    private key under a single wallet read lock.
        let account_index: u32 = 0;
        let (xpub_bytes, ecdh_private_key) = {
            let wallet = self.wallet.read().await;

            let account_type = AccountType::DashpayReceivingFunds {
                index: account_index,
                user_identity_id: sender_identity_id.to_buffer(),
                friend_identity_id: recipient_identity_id.to_buffer(),
            };
            let account_path = account_type.derivation_path(self.network).map_err(|err| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to derive DashPay receiving account path: {err}"
                ))
            })?;
            let account_xpub = wallet
                .derive_extended_public_key(&account_path)
                .map_err(|err| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to derive DashPay receiving account xpub: {err}"
                    ))
                })?;
            let xpub = account_xpub.encode();

            let ecdh_key = Self::derive_encryption_private_key(
                &wallet,
                self.network,
                identity_index,
                &sender_encryption_key,
            )?;

            (xpub, ecdh_key)
        };

        // 5. Build the signing key and signer.
        let signer = IdentitySigner::new(self.wallet.clone(), self.network, identity_index);
        let identity_public_key = sender_identity
            .public_keys()
            .values()
            .find(|k| k.purpose() == Purpose::AUTHENTICATION)
            .cloned()
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Sender identity has no authentication key for signing".to_string(),
                )
            })?;

        // 6. Prepare SDK input and submit.
        let contact_request_input = dash_sdk::platform::dashpay::ContactRequestInput {
            sender_identity: sender_identity.clone(),
            recipient: dash_sdk::platform::dashpay::RecipientIdentity::Identity(recipient_identity),
            sender_key_index,
            recipient_key_index,
            account_reference: account_index,
            account_label: None,
            auto_accept_proof: None,
        };

        let send_input = SendContactRequestInput {
            contact_request: contact_request_input,
            identity_public_key,
            signer,
        };

        let expected_key_id = sender_key_index;
        let ecdh_provider: EcdhProvider<
            _,
            _,
            fn(
                &dashcore::secp256k1::PublicKey,
            ) -> std::future::Ready<Result<[u8; 32], dash_sdk::Error>>,
            _,
        > = EcdhProvider::SdkSide {
            get_private_key: move |key: &IdentityPublicKey, _index: u32| {
                let pk = ecdh_private_key;
                let actual_key_id = key.id();
                async move {
                    if actual_key_id != expected_key_id {
                        return Err(dash_sdk::Error::Generic(format!(
                            "ECDH key mismatch: expected key {}, got {}",
                            expected_key_id, actual_key_id
                        )));
                    }
                    Ok(pk)
                }
            },
        };

        let xpub_bytes_clone = xpub_bytes.clone();
        let result = self
            .sdk
            .send_contact_request(send_input, ecdh_provider, |_account_ref: u32| async move {
                Ok::<Vec<u8>, dash_sdk::Error>(xpub_bytes_clone)
            })
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to send contact request: {e}"
                ))
            })?;

        // 7. Store the sent request in the local manager.
        let contact_request = ContactRequest::new(
            *sender_identity_id,
            result.recipient_id,
            sender_key_index,
            recipient_key_index,
            result.account_reference,
            // The encrypted xpub was already submitted to Platform as part of the
            // contact request document. We don't store the real ciphertext locally
            // because it is only needed by the recipient. A zeroed placeholder of the
            // correct length (96 bytes) is kept so the struct remains consistently
            // sized. Changing this field to Option<Vec<u8>> would be more precise but
            // requires updating all constructors and serialization code.
            vec![0u8; 96],
            result.document.created_at_core_block_height().unwrap_or(0),
            result.document.created_at().unwrap_or(0),
        );

        {
            let mut manager = self.identity_manager.write().await;
            let managed = manager
                .managed_identity_mut(sender_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*sender_identity_id))?;
            managed.add_sent_contact_request(contact_request);
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Sync contact requests from platform
// ---------------------------------------------------------------------------

impl DashPayWallet {
    /// Fetch and process contact requests from the platform for all local identities.
    ///
    /// For every identity in the local manager this method:
    /// 1. Fetches received contact-request documents from Platform.
    /// 2. Converts them into [`ContactRequest`] structs.
    /// 3. Adds each as an incoming request to the corresponding
    ///    `ManagedIdentity` (which may auto-establish a contact when a
    ///    matching outgoing request already exists).
    ///
    /// Returns all newly discovered incoming contact requests.
    pub async fn sync_contact_requests(&self) -> Result<Vec<ContactRequest>, PlatformWalletError> {
        let identity_ids: Vec<Identifier> = {
            let manager = self.identity_manager.read().await;
            manager.identities().keys().copied().collect()
        };

        let mut all_requests = Vec::new();

        for identity_id in identity_ids {
            let received_docs = self
                .sdk
                .fetch_received_contact_requests(identity_id, None)
                .await
                .map_err(|e| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to fetch received contact requests: {e}"
                    ))
                })?;

            let mut manager = self.identity_manager.write().await;
            let managed = match manager.managed_identity_mut(&identity_id) {
                Some(m) => m,
                None => continue,
            };

            for (_doc_id, maybe_doc) in received_docs.iter() {
                let doc = match maybe_doc {
                    Some(d) => d,
                    None => continue,
                };

                let sender_id = doc.owner_id();

                // Skip if already tracked (sent, incoming, or established).
                if managed.sent_contact_requests.contains_key(&sender_id)
                    || managed.incoming_contact_requests.contains_key(&sender_id)
                    || managed.established_contacts.contains_key(&sender_id)
                {
                    continue;
                }

                let props = doc.properties();

                let sender_key_index = match props
                    .get("senderKeyIndex")
                    .and_then(|v: &Value| v.to_integer::<u32>().ok())
                {
                    Some(v) => v,
                    None => {
                        tracing::warn!(
                            sender = %sender_id,
                            recipient = %identity_id,
                            "Skipping contact request document: missing senderKeyIndex"
                        );
                        continue;
                    }
                };
                let recipient_key_index = match props
                    .get("recipientKeyIndex")
                    .and_then(|v: &Value| v.to_integer::<u32>().ok())
                {
                    Some(v) => v,
                    None => {
                        tracing::warn!(
                            sender = %sender_id,
                            recipient = %identity_id,
                            "Skipping contact request document: missing recipientKeyIndex"
                        );
                        continue;
                    }
                };
                let account_reference = match props
                    .get("accountReference")
                    .and_then(|v: &Value| v.to_integer::<u32>().ok())
                {
                    Some(v) => v,
                    None => {
                        tracing::warn!(
                            sender = %sender_id,
                            recipient = %identity_id,
                            "Skipping contact request document: missing accountReference"
                        );
                        continue;
                    }
                };
                let encrypted_public_key = match props
                    .get("encryptedPublicKey")
                    .and_then(|v: &Value| v.as_bytes())
                    .cloned()
                {
                    Some(v) => v,
                    None => {
                        tracing::warn!(
                            sender = %sender_id,
                            recipient = %identity_id,
                            "Skipping contact request document: missing encryptedPublicKey"
                        );
                        continue;
                    }
                };

                let contact_request = ContactRequest::new(
                    sender_id,
                    identity_id,
                    sender_key_index,
                    recipient_key_index,
                    account_reference,
                    encrypted_public_key,
                    doc.created_at_core_block_height().unwrap_or(0),
                    doc.created_at().unwrap_or(0),
                );

                managed.add_incoming_contact_request(contact_request.clone());
                all_requests.push(contact_request);
            }
        }

        Ok(all_requests)
    }
}

// ---------------------------------------------------------------------------
// Accept an incoming contact request
// ---------------------------------------------------------------------------

impl DashPayWallet {
    /// Accept an incoming contact request by sending a reciprocal request and
    /// establishing the contact locally.
    ///
    /// All parameters are resolved internally from the incoming [`ContactRequest`]:
    /// - The recipient of the reciprocal request is derived from `request.sender_id`.
    /// - Our identity ID is `request.recipient_id`.
    /// - ECDH, signing key, identity index, and account index are resolved the
    ///   same way as [`send_contact_request`].
    ///
    /// # Arguments
    ///
    /// * `request` - The incoming [`ContactRequest`] to accept.
    pub async fn accept_contact_request(
        &self,
        request: &ContactRequest,
    ) -> Result<EstablishedContact, PlatformWalletError> {
        let our_identity_id = request.recipient_id;
        let sender_id = request.sender_id;

        // 1. Verify the incoming request is known.
        {
            let manager = self.identity_manager.read().await;
            let managed = manager
                .managed_identity(&our_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(our_identity_id))?;
            if !managed.incoming_contact_requests.contains_key(&sender_id) {
                return Err(PlatformWalletError::ContactRequestNotFound(sender_id));
            }
        }

        // 2. Send reciprocal request (this also stores it as a sent request
        //    in the managed identity which, combined with the existing
        //    incoming request, will auto-establish the contact).
        self.send_contact_request(&our_identity_id, &sender_id)
            .await?;

        // 3. The auto-establish logic in ManagedIdentity should have created
        //    the established contact. Retrieve and return it.
        let manager = self.identity_manager.read().await;
        let managed = manager
            .managed_identity(&our_identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(our_identity_id))?;

        managed
            .established_contacts
            .get(&sender_id)
            .cloned()
            .ok_or(PlatformWalletError::ContactRequestNotFound(sender_id))
    }
}

// ---------------------------------------------------------------------------
// Established contacts accessor
// ---------------------------------------------------------------------------

impl DashPayWallet {
    /// Get all established contacts across every identity managed by this wallet.
    ///
    /// Returns a flat list; each element includes the contact's identity ID.
    pub async fn established_contacts(&self) -> Vec<EstablishedContact> {
        let manager = self.identity_manager.read().await;
        manager
            .identities
            .values()
            .flat_map(|managed| managed.established_contacts.values().cloned())
            .collect()
    }
}
