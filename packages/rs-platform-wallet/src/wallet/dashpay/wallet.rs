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
use key_wallet::wallet::Wallet;
use key_wallet_manager::WalletManager;
use platform_encryption::CryptoError;
use tokio::sync::RwLock;

use dash_sdk::platform::dashpay::{EcdhProvider, SendContactRequestInput};

use crate::error::PlatformWalletError;
use crate::wallet::dashpay::contact_request::ContactRequest;
use crate::wallet::dashpay::established_contact::EstablishedContact;
use crate::wallet::dashpay::payment::DashpayAddressMatch;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::signer::IdentitySigner;

/// DashPay wallet providing contact request and payment functionality.
///
/// Shares the same `WalletManager` lock as all other sub-wallets.
#[derive(Clone)]
pub struct DashPayWallet {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// The shared wallet manager lock for all mutable wallet state.
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet within the manager this sub-wallet operates on.
    pub(crate) wallet_id: WalletId,
    /// Per-wallet persistence handle for queuing changesets.
    pub(crate) persister: crate::wallet::persister::WalletPersister,
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
    /// * `account_label`         - Optional account label (plaintext; encrypted by SDK).
    /// * `auto_accept_proof`     - Optional auto-accept proof bytes (38-102 bytes).
    pub async fn send_contact_request(
        &self,
        sender_identity_id: &Identifier,
        recipient_identity_id: &Identifier,
        account_label: Option<String>,
        auto_accept_proof: Option<Vec<u8>>,
    ) -> Result<ContactRequest, PlatformWalletError> {
        // 1. Retrieve the sender identity and its HD index from the local manager
        //    via a single managed_identity() call.
        let (sender_identity, identity_index) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
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
        //    private key under a single read lock.
        let account_index: u32 = 0;
        let (xpub_bytes, ecdh_private_key) = {
            let wm = self.wallet_manager.read().await;
            let wallet = wm
                .get_wallet(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

            let account_type = AccountType::DashpayReceivingFunds {
                index: account_index,
                user_identity_id: sender_identity_id.to_buffer(),
                friend_identity_id: recipient_identity_id.to_buffer(),
            };
            let account_path = account_type
                .derivation_path(self.sdk.network)
                .map_err(|err| {
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
                wallet,
                self.sdk.network,
                identity_index,
                &sender_encryption_key,
            )?;

            (xpub, ecdh_key)
        };

        // 5. Build the signing key and signer.
        let signer = IdentitySigner::new(
            self.wallet_manager.clone(),
            self.wallet_id,
            self.sdk.network,
            identity_index,
        );
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
            account_label,
            auto_accept_proof,
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
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity_mut(sender_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*sender_identity_id))?;
            managed.add_sent_contact_request(contact_request.clone(), &self.persister);
        }

        // Register the contact account in ManagedWalletInfo so SPV monitors
        // incoming payment addresses from this contact.
        self.register_contact_account(sender_identity_id, recipient_identity_id, account_index)
            .await?;

        Ok(contact_request)
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
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.identity_manager.identities().keys().copied().collect()
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

            let mut wm = self.wallet_manager.write().await;
            let info = match wm.get_wallet_info_mut(&self.wallet_id) {
                Some(i) => i,
                None => continue,
            };
            let managed = match info.identity_manager.managed_identity_mut(&identity_id) {
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

                managed.add_incoming_contact_request(contact_request.clone(), &self.persister);
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
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(&our_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(our_identity_id))?;
            if !managed.incoming_contact_requests.contains_key(&sender_id) {
                return Err(PlatformWalletError::ContactRequestNotFound(sender_id));
            }
        }

        // 2. Send reciprocal request (this also stores it as a sent request
        //    in the managed identity which, combined with the existing
        //    incoming request, will auto-establish the contact).
        self.send_contact_request(&our_identity_id, &sender_id, None, None)
            .await?;

        // 3. The auto-establish logic in ManagedIdentity should have created
        //    the established contact. Retrieve and return it.
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let managed = info
            .identity_manager
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
    // TODO: We don't want to clone all contacts on get - it's terrible.
    /// Get all established contacts across every identity managed by this wallet.
    ///
    /// Returns a flat list; each element includes the contact's identity ID.
    pub async fn established_contacts(&self) -> Vec<EstablishedContact> {
        let wm = self.wallet_manager.read().await;
        let Some(info) = wm.get_wallet_info(&self.wallet_id) else {
            return Vec::new();
        };
        info.identity_manager
            .identities
            .values()
            .flat_map(|managed| managed.established_contacts.values().cloned())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Contact xpub and payment address derivation (DIP-14 / DIP-15)
// ---------------------------------------------------------------------------

impl DashPayWallet {
    /// Get the contact xpub data for a specific contact relationship.
    ///
    /// Derives the extended public key along path:
    /// `m/9'/coin'/15'/account'/(sender_id)/(recipient_id)`
    ///
    /// The last two segments use DIP-14 256-bit non-hardened derivation.
    ///
    /// # Arguments
    ///
    /// * `account_index` - Account index (hardened) in the derivation path.
    /// * `sender_id`     - Our identity identifier.
    /// * `recipient_id`  - The contact's identity identifier.
    pub async fn contact_xpub(
        &self,
        account_index: u32,
        sender_id: &Identifier,
        recipient_id: &Identifier,
    ) -> Result<super::dip14::ContactXpubData, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let wallet = wm
            .get_wallet(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        super::dip14::derive_contact_xpub(
            wallet,
            self.sdk.network,
            account_index,
            sender_id,
            recipient_id,
        )
    }

    // TODO: Isn't this something what should be done internally?
    /// Derive payment addresses for a contact (for receiving payments from them).
    ///
    /// Returns `count` addresses starting from `start_index`, derived via
    /// standard BIP32 from the contact xpub.
    ///
    /// Register a DashPay contact account in the wallet's `ManagedWalletInfo`.
    ///
    /// Creates a `DashpayReceivingFunds` managed account with address pools
    /// so the SPV adapter monitors incoming payments from this contact.
    /// Call this when a contact is established (mutual requests exist).
    ///
    /// No-op if the account already exists for this contact relationship.
    pub async fn register_contact_account(
        &self,
        our_identity_id: &Identifier,
        contact_identity_id: &Identifier,
        account_index: u32,
    ) -> Result<(), PlatformWalletError> {
        let account_type = AccountType::DashpayReceivingFunds {
            index: account_index,
            user_identity_id: our_identity_id.to_buffer(),
            friend_identity_id: contact_identity_id.to_buffer(),
        };

        // Derive the account xpub and add to both Wallet and ManagedWalletInfo
        let mut wm = self.wallet_manager.write().await;
        let wallet = wm
            .get_wallet(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let path = account_type
            .derivation_path(self.sdk.network)
            .map_err(|err| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to derive DashPay contact account path: {err}"
                ))
            })?;
        let account_xpub = wallet.derive_extended_public_key(&path).map_err(|err| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to derive DashPay contact xpub: {err}"
            ))
        })?;

        let account = key_wallet::Account {
            parent_wallet_id: Some(wallet.wallet_id),
            account_type,
            network: self.sdk.network,
            account_xpub,
            is_watch_only: false,
        };

        // Add managed wrapper to ManagedWalletInfo (address pools, state tracking)
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let managed = key_wallet::managed_account::ManagedCoreAccount::from_account(&account);
        info.core_wallet.accounts.insert(managed).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to register contact account: {e}"
            ))
        })?;

        Ok(())
    }

    /// Match an on-chain address against this wallet's registered
    /// DashPay contact receival accounts.
    ///
    /// Iterates every `DashpayReceivingFunds` account in this
    /// wallet's [`key_wallet::managed_account::ManagedAccountCollection`]
    /// and checks whether the address belongs to any of their
    /// address pools. Returns the first match as a
    /// [`DashpayAddressMatch`], or `None` if the address is not
    /// a DashPay contact address for this wallet.
    ///
    /// Used by the SPV / backend task layer to classify observed
    /// transaction outputs as DashPay incoming payments from a
    /// specific contact — replaces the redundant
    /// `dashpay_address_mappings` reverse-lookup table the UI
    /// layer used to maintain. The authoritative state is already
    /// tracked by `register_contact_account`, which inserts the
    /// account into the wallet's `ManagedAccountCollection` so
    /// key-wallet manages the address pool (derivation + gap limit
    /// + used tracking).
    ///
    /// Only the external pool of each receival account is
    /// searched: DashPay uses a single-pool account type so all
    /// contact payment addresses live on that one pool.
    pub async fn match_incoming_dashpay_address(
        &self,
        address: &dashcore::Address,
    ) -> Option<DashpayAddressMatch> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id)?;
        Self::match_in_collection(info, address)
    }

    /// Blocking variant of [`match_incoming_dashpay_address`] for
    /// sync callers (SPV transaction-processing frame loop). Uses
    /// `tokio::sync::RwLock::blocking_read` — must NOT be called
    /// from within a tokio async context.
    pub fn match_incoming_dashpay_address_blocking(
        &self,
        address: &dashcore::Address,
    ) -> Option<DashpayAddressMatch> {
        let wm = self.wallet_manager.blocking_read();
        let info = wm.get_wallet_info(&self.wallet_id)?;
        Self::match_in_collection(info, address)
    }

    /// Non-blocking variant of [`match_incoming_dashpay_address`].
    /// Returns `Err(())` if the wallet-manager lock is currently
    /// contended (e.g. SPV is processing a block). Returns `Ok(None)`
    /// if the address does not belong to any DashPay receiving
    /// account. Safe to call from any thread, including tokio runtime
    /// threads, where the blocking variant would panic.
    pub fn try_match_incoming_dashpay_address(
        &self,
        address: &dashcore::Address,
    ) -> Result<Option<DashpayAddressMatch>, ()> {
        let wm = self.wallet_manager.try_read().map_err(|_| ())?;
        let Some(info) = wm.get_wallet_info(&self.wallet_id) else {
            return Ok(None);
        };
        Ok(Self::match_in_collection(info, address))
    }

    /// Shared implementation that iterates
    /// `info.core_wallet.accounts.dashpay_receival_accounts` and
    /// checks each account's address pool for a match.
    fn match_in_collection(
        info: &PlatformWalletInfo,
        address: &dashcore::Address,
    ) -> Option<DashpayAddressMatch> {
        use key_wallet::managed_account::managed_account_type::ManagedAccountType;

        for (key, account) in &info.core_wallet.accounts.dashpay_receival_accounts {
            let ManagedAccountType::DashpayReceivingFunds {
                user_identity_id,
                friend_identity_id,
                ..
            } = &account.account_type
            else {
                // Routing invariant: dashpay_receival_accounts must
                // only contain DashpayReceivingFunds. If this ever
                // trips, it's a key-wallet bug.
                debug_assert!(
                    false,
                    "non-DashpayReceivingFunds in dashpay_receival_accounts"
                );
                continue;
            };
            let Some(info) = account.get_address_info(address) else {
                continue;
            };
            // Sanity check — the collection key should match the
            // account type's own identity ids.
            debug_assert_eq!(&key.user_identity_id, user_identity_id);
            debug_assert_eq!(&key.friend_identity_id, friend_identity_id);
            return Some(DashpayAddressMatch {
                user_identity_id: Identifier::from(*user_identity_id),
                friend_identity_id: Identifier::from(*friend_identity_id),
                address_index: info.index,
            });
        }
        None
    }

    /// # Arguments
    ///
    /// * `account_index` - Account index (hardened) in the derivation path.
    /// * `sender_id`     - Our identity identifier.
    /// * `recipient_id`  - The contact's identity identifier.
    /// * `start_index`   - First payment address index.
    /// * `count`         - Number of addresses to derive.
    pub async fn contact_payment_addresses(
        &self,
        account_index: u32,
        sender_id: &Identifier,
        recipient_id: &Identifier,
        start_index: u32,
        count: u32,
    ) -> Result<Vec<dashcore::Address>, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let wallet = wm
            .get_wallet(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let data = super::dip14::derive_contact_xpub(
            wallet,
            self.sdk.network,
            account_index,
            sender_id,
            recipient_id,
        )?;
        super::dip14::derive_contact_payment_addresses(
            &data.xpub,
            start_index,
            count,
            self.sdk.network,
        )
    }
}

// ---------------------------------------------------------------------------
// Account label encryption / decryption (DIP-15)
// ---------------------------------------------------------------------------

impl DashPayWallet {
    /// Encrypt an account label using CBC-AES-256 with a shared ECDH key.
    ///
    /// Uses the `platform_encryption` crate which prepends a random 16-byte IV
    /// to the ciphertext.
    ///
    /// # Arguments
    ///
    /// * `label`      - The account label to encrypt.
    /// * `shared_key` - 32-byte shared secret derived via ECDH.
    ///
    /// # Returns
    ///
    /// Encrypted label bytes (48-80 bytes: 16-byte IV + 32-64 byte ciphertext).
    pub fn encrypt_account_label(
        label: &str,
        shared_key: &[u8; 32],
    ) -> Result<Vec<u8>, PlatformWalletError> {
        use dashcore::secp256k1::rand::{thread_rng, RngCore};

        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);

        let encrypted = platform_encryption::encrypt_account_label(shared_key, &iv, label);

        Ok(encrypted)
    }

    /// Decrypt an account label using CBC-AES-256 with a shared ECDH key.
    ///
    /// The first 16 bytes of `encrypted` are taken as the IV.
    ///
    /// # Arguments
    ///
    /// * `encrypted`  - Encrypted label bytes (48-80 bytes).
    /// * `shared_key` - 32-byte shared secret derived via ECDH.
    ///
    /// # Returns
    ///
    /// The decrypted label string.
    pub fn decrypt_account_label(
        encrypted: &[u8],
        shared_key: &[u8; 32],
    ) -> Result<String, PlatformWalletError> {
        platform_encryption::decrypt_account_label(shared_key, encrypted).map_err(|e| match e {
            CryptoError::DecryptionFailed => {
                PlatformWalletError::InvalidIdentityData("Account label decryption failed".into())
            }
            CryptoError::InvalidUtf8 => PlatformWalletError::InvalidIdentityData(
                "Decrypted account label is not valid UTF-8".into(),
            ),
            CryptoError::InvalidCiphertextLength => PlatformWalletError::InvalidIdentityData(
                "Invalid encrypted account label length".into(),
            ),
        })
    }
}

// ---------------------------------------------------------------------------
// Sent contact requests query
// ---------------------------------------------------------------------------

impl DashPayWallet {
    /// Fetch sent contact requests for a specific identity from Platform.
    ///
    /// Queries the DashPay contract for `contactRequest` documents where
    /// `$ownerId == identity_id`, ordered by `$createdAt`.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity whose sent requests to fetch.
    ///
    /// # Returns
    ///
    /// A list of [`ContactRequest`] structs representing the sent requests.
    pub async fn sent_contact_requests(
        &self,
        identity_id: &Identifier,
    ) -> Result<Vec<ContactRequest>, PlatformWalletError> {
        let sent_docs = self
            .sdk
            .fetch_sent_contact_requests(*identity_id, None)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to fetch sent contact requests: {e}"
                ))
            })?;

        let mut requests = Vec::new();

        for (_doc_id, maybe_doc) in sent_docs.iter() {
            let doc = match maybe_doc {
                Some(d) => d,
                None => continue,
            };

            let sender_id = doc.owner_id();

            let props = doc.properties();

            let to_user_id = match props
                .get("toUserId")
                .and_then(|v: &Value| v.to_identifier().ok())
            {
                Some(v) => v,
                None => continue,
            };
            let sender_key_index = match props
                .get("senderKeyIndex")
                .and_then(|v: &Value| v.to_integer::<u32>().ok())
            {
                Some(v) => v,
                None => continue,
            };
            let recipient_key_index = match props
                .get("recipientKeyIndex")
                .and_then(|v: &Value| v.to_integer::<u32>().ok())
            {
                Some(v) => v,
                None => continue,
            };
            let account_reference = match props
                .get("accountReference")
                .and_then(|v: &Value| v.to_integer::<u32>().ok())
            {
                Some(v) => v,
                None => continue,
            };
            let encrypted_public_key = match props
                .get("encryptedPublicKey")
                .and_then(|v: &Value| v.as_bytes())
                .cloned()
            {
                Some(v) => v,
                None => continue,
            };

            let mut contact_request = ContactRequest::new(
                sender_id,
                to_user_id,
                sender_key_index,
                recipient_key_index,
                account_reference,
                encrypted_public_key,
                doc.created_at_core_block_height().unwrap_or(0),
                doc.created_at().unwrap_or(0),
            );

            // Attach optional encrypted account label if present.
            contact_request.encrypted_account_label = props
                .get("encryptedAccountLabel")
                .and_then(|v: &Value| v.as_bytes())
                .cloned();

            // Attach optional auto-accept proof if present.
            contact_request.auto_accept_proof = props
                .get("autoAcceptProof")
                .and_then(|v: &Value| v.as_bytes())
                .cloned();

            requests.push(contact_request);
        }

        // Sort by creation time ascending.
        requests.sort_by_key(|r| r.created_at);

        Ok(requests)
    }
}

// ---------------------------------------------------------------------------
// Reject contact request
// ---------------------------------------------------------------------------

impl DashPayWallet {
    /// Reject a contact request by hiding the contact.
    ///
    /// This marks the contact as hidden in the local identity manager so that
    /// the UI no longer surfaces it. A full DashPay implementation would also
    /// create or update a `contactInfo` document on Platform with
    /// `display_hidden: true`; that part requires SDK support for document
    /// creation on arbitrary contracts which is not yet available here.
    ///
    /// # Arguments
    ///
    /// * `identity_id`         - Our identity.
    /// * `contact_identity_id` - The identity whose request we reject.
    pub async fn reject_contact_request(
        &self,
        identity_id: &Identifier,
        contact_identity_id: &Identifier,
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let managed = info
            .identity_manager
            .managed_identity_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        // Remove from incoming requests (if present).
        if managed
            .incoming_contact_requests
            .remove(contact_identity_id)
            .is_none()
        {
            return Err(PlatformWalletError::ContactRequestNotFound(
                *contact_identity_id,
            ));
        }

        // TODO: When the SDK supports creating/updating arbitrary DashPay
        // documents (contactInfo), submit a `display_hidden: true` document to
        // Platform here so the rejection is persisted across devices.

        tracing::info!(
            identity = %identity_id,
            rejected_contact = %contact_identity_id,
            "Contact request rejected (hidden locally)"
        );

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Sync profiles
// ---------------------------------------------------------------------------

impl DashPayWallet {
    /// Fetch DashPay profile documents from Platform for all managed
    /// identities and cache them on [`ManagedIdentity`].
    ///
    /// Returns the number of profiles that were successfully synced.
    pub(crate) async fn sync_profiles(&self) -> Result<u32, PlatformWalletError> {
        // 1. Collect all managed identity IDs under a short read lock.
        let identity_ids: Vec<Identifier> = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.identity_manager.identities().keys().copied().collect()
        };

        if identity_ids.is_empty() {
            return Ok(0);
        }

        // 2. Load the DashPay contract locally (no network round-trip needed).
        let dashpay_contract = Arc::new(
            dpp::system_data_contracts::load_system_data_contract(
                dpp::data_contracts::SystemDataContract::Dashpay,
                dpp::version::PlatformVersion::latest(),
            )
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to load DashPay contract: {e}"
                ))
            })?,
        );

        let mut profiles_synced = 0u32;

        // 3. For each identity fetch the profile document, then cache it.
        for identity_id in &identity_ids {
            match self
                .fetch_profile_document(&dashpay_contract, identity_id)
                .await
            {
                Ok(Some(profile)) => {
                    let mut wm = self.wallet_manager.write().await;
                    if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                        if let Some(managed) =
                            info.identity_manager.managed_identity_mut(identity_id)
                        {
                            managed.set_dashpay_profile(Some(profile), &self.persister);
                            profiles_synced += 1;
                        }
                    }
                }
                Ok(None) => {
                    // No profile on Platform — clear local cache only when one
                    // is currently stored, to avoid needless writes.
                    let mut wm = self.wallet_manager.write().await;
                    if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                        if let Some(managed) =
                            info.identity_manager.managed_identity_mut(identity_id)
                        {
                            if managed.dashpay_profile.is_some() {
                                managed.set_dashpay_profile(None, &self.persister);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to fetch DashPay profile"
                    );
                }
            }
        }

        Ok(profiles_synced)
    }

    /// Fetch a single `profile` document from the DashPay contract for
    /// `identity_id` and convert it into a [`DashPayProfile`].
    ///
    /// Returns `Ok(None)` when no profile document exists on Platform.
    async fn fetch_profile_document(
        &self,
        dashpay_contract: &Arc<dpp::data_contract::DataContract>,
        identity_id: &Identifier,
    ) -> Result<Option<crate::wallet::dashpay::DashPayProfile>, PlatformWalletError> {
        use dash_sdk::drive::query::{WhereClause, WhereOperator};
        use dash_sdk::platform::FetchMany;
        use dpp::document::Document;
        use dpp::platform_value::platform_value;

        // Build query: profile documents WHERE $ownerId = identity_id.
        let query = dash_sdk::platform::DocumentQuery {
            data_contract: Arc::clone(dashpay_contract),
            document_type_name: "profile".to_string(),
            where_clauses: vec![WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: platform_value!(identity_id),
            }],
            order_by_clauses: vec![],
            limit: 1,
            start: None,
        };

        let docs = Document::fetch_many(&self.sdk, query)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        // Take the first result (profile is unique per $ownerId).
        let doc = match docs.into_values().next() {
            Some(Some(d)) => d,
            _ => return Ok(None),
        };

        let props = doc.properties();

        let display_name = props
            .get("displayName")
            .and_then(|v: &Value| v.as_str().map(ToString::to_string))
            .filter(|s| !s.is_empty());

        let public_message = props
            .get("publicMessage")
            .and_then(|v: &Value| v.as_str().map(ToString::to_string))
            .filter(|s| !s.is_empty());

        let avatar_url = props
            .get("avatarUrl")
            .and_then(|v: &Value| v.as_str().map(ToString::to_string))
            .filter(|s| !s.is_empty());

        let avatar_hash = props
            .get("avatarHash")
            .and_then(|v: &Value| v.as_bytes())
            .and_then(|bytes| <[u8; 32]>::try_from(bytes.as_slice()).ok());

        let avatar_fingerprint = props
            .get("avatarFingerprint")
            .and_then(|v: &Value| v.as_bytes())
            .and_then(|bytes| <[u8; 8]>::try_from(bytes.as_slice()).ok());

        Ok(Some(crate::wallet::dashpay::DashPayProfile {
            display_name,
            // `publicMessage` from the contract is the bio/about-me field.
            bio: public_message.clone(),
            avatar_url,
            avatar_hash,
            avatar_fingerprint,
            public_message,
        }))
    }
}

// ---------------------------------------------------------------------------
// Comprehensive DashPay sync
// ---------------------------------------------------------------------------

impl DashPayWallet {
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
// Profile create / update
// ---------------------------------------------------------------------------

impl DashPayWallet {
    /// Create a new DashPay profile document on Platform for `identity_id`.
    ///
    /// Steps:
    /// 1. Load the DashPay contract.
    /// 2. Compute `avatarHash` (SHA-256) and `avatarFingerprint` (dHash)
    ///    from `input.avatar_bytes` when present.
    /// 3. Build a `profile` document with the supplied fields.
    /// 4. Retrieve the identity and signing key from the wallet manager.
    /// 5. Broadcast the document creation via the SDK.
    /// 6. Cache the resulting [`DashPayProfile`] on [`ManagedIdentity`].
    /// 7. Return the cached profile.
    pub async fn create_profile(
        &self,
        identity_id: &Identifier,
        input: crate::wallet::dashpay::ProfileUpdate,
    ) -> Result<crate::wallet::dashpay::DashPayProfile, PlatformWalletError> {
        use dash_sdk::platform::transition::put_document::PutDocument;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::document::{Document, DocumentV0};
        use dpp::platform_value::Value;

        // 1. Load the DashPay data contract.
        let dashpay_contract = Arc::new(
            dpp::system_data_contracts::load_system_data_contract(
                dpp::data_contracts::SystemDataContract::Dashpay,
                dpp::version::PlatformVersion::latest(),
            )
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to load DashPay contract: {e}"
                ))
            })?,
        );

        // 2. Compute avatar hashes when raw bytes are provided.
        let (avatar_hash, avatar_fingerprint) = if let Some(ref bytes) = input.avatar_bytes {
            let hash = crate::wallet::dashpay::calculate_avatar_hash(bytes);
            let fingerprint = crate::wallet::dashpay::calculate_dhash_fingerprint(bytes)
                .map_err(|e| PlatformWalletError::InvalidIdentityData(e))?;
            (Some(hash), Some(fingerprint))
        } else {
            (None, None)
        };

        // 3. Build the document property map.
        let mut properties = std::collections::BTreeMap::new();
        if let Some(ref name) = input.display_name {
            properties.insert("displayName".to_string(), Value::Text(name.clone()));
        }
        if let Some(ref msg) = input.public_message {
            properties.insert("publicMessage".to_string(), Value::Text(msg.clone()));
        }
        if let Some(ref url) = input.avatar_url {
            properties.insert("avatarUrl".to_string(), Value::Text(url.clone()));
        }
        if let Some(hash) = avatar_hash {
            properties.insert("avatarHash".to_string(), Value::Bytes32(hash));
        }
        if let Some(fp) = avatar_fingerprint {
            properties.insert("avatarFingerprint".to_string(), Value::Bytes(fp.to_vec()));
        }

        // 4. Retrieve identity, identity_index, and signing key.
        let (_identity, identity_index, signing_key) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let idx = managed.identity_index;
            let key = managed
                .identity
                .public_keys()
                .values()
                .find(|k| k.purpose() == Purpose::AUTHENTICATION)
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "Identity has no authentication key for signing".to_string(),
                    )
                })?;
            (managed.identity.clone(), idx, key)
        };

        // Build a stub document — the SDK will assign the real ID during
        // `put_to_platform_and_wait_for_response` (entropy-based generation).
        let stub_document = Document::V0(DocumentV0 {
            id: Identifier::from([0u8; 32]),
            owner_id: *identity_id,
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        // 5. Broadcast via PutDocument trait (handles ID + entropy generation).
        let signer = IdentitySigner::new(
            self.wallet_manager.clone(),
            self.wallet_id,
            self.sdk.network,
            identity_index,
        );

        let profile_document_type = dashpay_contract
            .document_type_for_name("profile")
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to get profile document type: {e}"
                ))
            })?
            .to_owned_document_type();

        let _result_doc = stub_document
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                profile_document_type,
                None, // entropy auto-generated
                signing_key,
                None, // no token payment
                &signer,
                None, // default settings
            )
            .await
            .map_err(PlatformWalletError::Sdk)?;

        // 6. Build and cache the profile locally.
        let profile = crate::wallet::dashpay::DashPayProfile {
            display_name: input.display_name,
            bio: input.public_message.clone(),
            avatar_url: input.avatar_url,
            avatar_hash,
            avatar_fingerprint,
            public_message: input.public_message,
        };

        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                    managed.set_dashpay_profile(Some(profile.clone()), &self.persister);
                }
            }
        }

        Ok(profile)
    }

    /// Update an existing DashPay profile on Platform for `identity_id`.
    ///
    /// Fetches the current profile document to obtain its ID and revision,
    /// applies the fields from `input`, then broadcasts a document replace
    /// transition. The local cache is updated on success.
    pub async fn update_profile(
        &self,
        identity_id: &Identifier,
        input: crate::wallet::dashpay::ProfileUpdate,
    ) -> Result<crate::wallet::dashpay::DashPayProfile, PlatformWalletError> {
        use dash_sdk::platform::transition::put_document::PutDocument;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::document::{Document, DocumentV0, INITIAL_REVISION};
        use dpp::platform_value::Value;

        // 1. Load the DashPay contract.
        let dashpay_contract = Arc::new(
            dpp::system_data_contracts::load_system_data_contract(
                dpp::data_contracts::SystemDataContract::Dashpay,
                dpp::version::PlatformVersion::latest(),
            )
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to load DashPay contract: {e}"
                ))
            })?,
        );

        // 2. Fetch the existing profile document to get its Platform ID and
        //    current revision. We must query the raw Document rather than the
        //    parsed DashPayProfile because we need the document ID field.
        let (existing_doc_id, current_revision) = {
            use dash_sdk::drive::query::{WhereClause, WhereOperator};
            use dash_sdk::platform::FetchMany;
            use dpp::platform_value::platform_value;

            let query = dash_sdk::platform::DocumentQuery {
                data_contract: Arc::clone(&dashpay_contract),
                document_type_name: "profile".to_string(),
                where_clauses: vec![WhereClause {
                    field: "$ownerId".to_string(),
                    operator: WhereOperator::Equal,
                    value: platform_value!(identity_id),
                }],
                order_by_clauses: vec![],
                limit: 1,
                start: None,
            };

            let docs = Document::fetch_many(&self.sdk, query)
                .await
                .map_err(PlatformWalletError::Sdk)?;

            match docs.into_values().next() {
                Some(Some(doc)) => {
                    let id = doc.id();
                    let rev = doc.revision().unwrap_or(INITIAL_REVISION);
                    (id, rev)
                }
                _ => {
                    return Err(PlatformWalletError::InvalidIdentityData(
                        "No existing profile document found to update".to_string(),
                    ));
                }
            }
        };

        // 3. Compute avatar hashes when raw bytes are provided.
        let (avatar_hash, avatar_fingerprint) = if let Some(ref bytes) = input.avatar_bytes {
            let hash = crate::wallet::dashpay::calculate_avatar_hash(bytes);
            let fingerprint = crate::wallet::dashpay::calculate_dhash_fingerprint(bytes)
                .map_err(|e| PlatformWalletError::InvalidIdentityData(e))?;
            (Some(hash), Some(fingerprint))
        } else {
            // Preserve existing avatar fields from the local cache.
            let wm = self.wallet_manager.read().await;
            let (h, f) = wm
                .get_wallet_info(&self.wallet_id)
                .and_then(|info| info.identity_manager.managed_identity(identity_id))
                .and_then(|m| m.dashpay_profile.as_ref())
                .map(|p| (p.avatar_hash, p.avatar_fingerprint))
                .unwrap_or((None, None));
            (h, f)
        };

        // 4. Build the updated property map.
        let mut properties = std::collections::BTreeMap::new();
        if let Some(ref name) = input.display_name {
            properties.insert("displayName".to_string(), Value::Text(name.clone()));
        }
        if let Some(ref msg) = input.public_message {
            properties.insert("publicMessage".to_string(), Value::Text(msg.clone()));
        }
        if let Some(ref url) = input.avatar_url {
            properties.insert("avatarUrl".to_string(), Value::Text(url.clone()));
        }
        if let Some(hash) = avatar_hash {
            properties.insert("avatarHash".to_string(), Value::Bytes32(hash));
        }
        if let Some(fp) = avatar_fingerprint {
            properties.insert("avatarFingerprint".to_string(), Value::Bytes(fp.to_vec()));
        }

        // 5. Retrieve identity_index and signing key.
        let (identity_index, signing_key) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let key = managed
                .identity
                .public_keys()
                .values()
                .find(|k| k.purpose() == Purpose::AUTHENTICATION)
                .cloned()
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(
                        "Identity has no authentication key for signing".to_string(),
                    )
                })?;
            (managed.identity_index, key)
        };

        // 6. Build the document with the existing ID and bumped revision.
        let updated_document = Document::V0(DocumentV0 {
            id: existing_doc_id,
            owner_id: *identity_id,
            properties,
            // Bumping revision signals to `put_to_platform` that this is a
            // replace transition (revision > INITIAL_REVISION).
            revision: Some(current_revision + 1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        // 7. Broadcast the replace transition.
        let signer = IdentitySigner::new(
            self.wallet_manager.clone(),
            self.wallet_id,
            self.sdk.network,
            identity_index,
        );

        let profile_document_type = dashpay_contract
            .document_type_for_name("profile")
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to get profile document type: {e}"
                ))
            })?
            .to_owned_document_type();

        let _result_doc = updated_document
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                profile_document_type,
                None, // entropy not used for replace
                signing_key,
                None, // no token payment
                &signer,
                None, // default settings
            )
            .await
            .map_err(PlatformWalletError::Sdk)?;

        // 8. Build and cache the updated profile.
        let profile = crate::wallet::dashpay::DashPayProfile {
            display_name: input.display_name,
            bio: input.public_message.clone(),
            avatar_url: input.avatar_url,
            avatar_hash,
            avatar_fingerprint,
            public_message: input.public_message,
        };

        {
            let mut wm = self.wallet_manager.write().await;
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                    managed.set_dashpay_profile(Some(profile.clone()), &self.persister);
                }
            }
        }

        Ok(profile)
    }
}
