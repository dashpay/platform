//! DashPay `contactInfo` document sync + publish (M3 task 13 / G10 +
//! G5 stage 2).
//!
//! `contactInfo` carries the owner's PRIVATE per-contact metadata
//! (alias, note, `displayHidden`) self-encrypted per
//! [`crate::wallet::identity::crypto::contact_info`] — publishing it
//! is what makes alias/note/hide survive restore-from-seed and sync
//! across devices (the M2 app stored these device-locally).
//!
//! Document identity: the unique index is
//! `($ownerId, rootEncryptionKeyIndex, derivationEncryptionKeyIndex)`
//! — one document per contact, distinguished by the (sequential)
//! derivation index. The contact ↔ document mapping is intentionally
//! **stateless**: resolving which doc belongs to which contact means
//! decrypting each doc's `encToUserId` with the keys its own indices
//! select. No extra local schema, and restore-from-seed recovers
//! everything from chain.

use dpp::document::{Document, DocumentV0};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::identity::signer::Signer;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::identity::crypto::contact_info::{
    decode_private_data, derive_contact_info_keys, encode_private_data, ContactInfoPrivateData,
};

/// One decrypted `contactInfo` document.
struct DecryptedContactInfo {
    doc_id: Identifier,
    revision: u64,
    derivation_index: u32,
    contact_id: Identifier,
    data: ContactInfoPrivateData,
}

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Fetch + decrypt every `contactInfo` document owned by
    /// `identity_id`. Documents whose keys we can't derive (foreign
    /// root index) or whose payload doesn't decrypt are skipped with a
    /// warning — a malformed doc must not abort the sync pass.
    async fn fetch_decrypted_contact_infos(
        &self,
        identity_id: &Identifier,
    ) -> Result<Vec<DecryptedContactInfo>, PlatformWalletError> {
        use dash_sdk::drive::query::{OrderClause, WhereClause, WhereOperator};
        use dash_sdk::platform::FetchMany;
        use dpp::document::DocumentV0Getters;
        use dpp::platform_value::platform_value;

        let dashpay_contract = super::dashpay_contract()?;

        let query = dash_sdk::platform::DocumentQuery {
            select: dash_sdk::drive::query::SelectProjection::documents(),
            data_contract: dashpay_contract,
            document_type_name: "contactInfo".to_string(),
            where_clauses: vec![WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: platform_value!(identity_id),
            }],
            group_by: vec![],
            having: vec![],
            // Load-bearing, not cosmetic: drive answers a bare
            // secondary-index equality with a verified proof of
            // ABSENCE (same trap the contact-request queries hit —
            // see fetch_received_contact_requests). The order-by
            // binds the query to the ownerIdAndUpdatedAt index.
            order_by_clauses: vec![OrderClause {
                field: "$updatedAt".to_string(),
                ascending: true,
            }],
            limit: 100,
            start: None,
        };

        let docs = Document::fetch_many(&self.sdk, query)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        // Resolve the wallet HD slot once; decryption is per-doc.
        let (identity_index, wallet_snapshot) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let Some(identity_index) = managed.identity_index else {
                // Watch-only / out-of-wallet identity — no HD slot to
                // derive the self-encryption keys from (G4 hook later).
                return Ok(Vec::new());
            };
            let wallet = wm
                .get_wallet(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            (identity_index, wallet.clone())
        };

        let mut out = Vec::new();
        for (doc_id, maybe_doc) in docs.iter() {
            let Some(doc) = maybe_doc else { continue };
            let props = doc.properties();
            let (Some(root_index), Some(derivation_index)) = (
                props
                    .get("rootEncryptionKeyIndex")
                    .and_then(|v: &Value| v.to_integer::<u32>().ok()),
                props
                    .get("derivationEncryptionKeyIndex")
                    .and_then(|v: &Value| v.to_integer::<u32>().ok()),
            ) else {
                tracing::warn!(owner = %identity_id, doc = %doc_id, "contactInfo missing key indices");
                continue;
            };
            let (Some(enc_to_user_id), Some(private_data)) = (
                props
                    .get("encToUserId")
                    .and_then(|v: &Value| v.to_binary_bytes().ok()),
                props
                    .get("privateData")
                    .and_then(|v: &Value| v.to_binary_bytes().ok()),
            ) else {
                tracing::warn!(owner = %identity_id, doc = %doc_id, "contactInfo missing payload fields");
                continue;
            };
            let Ok(enc_to_user_id): Result<[u8; 32], _> = enc_to_user_id.as_slice().try_into()
            else {
                tracing::warn!(owner = %identity_id, doc = %doc_id, "contactInfo encToUserId is not 32 bytes");
                continue;
            };

            let keys = match derive_contact_info_keys(
                &wallet_snapshot,
                self.sdk.network,
                identity_index,
                root_index,
                derivation_index,
            ) {
                Ok(k) => k,
                Err(e) => {
                    tracing::warn!(owner = %identity_id, doc = %doc_id, error = %e, "contactInfo key derivation failed");
                    continue;
                }
            };

            let contact_id = Identifier::from(platform_encryption::decrypt_enc_to_user_id(
                &keys.enc_to_user_id_key,
                &enc_to_user_id,
            ));
            let data = match platform_encryption::decrypt_private_data(
                &keys.private_data_key,
                &private_data,
            )
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!("privateData decrypt: {e}"))
            })
            .and_then(|plain| decode_private_data(&plain))
            {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(owner = %identity_id, doc = %doc_id, error = %e, "contactInfo privateData decode failed");
                    continue;
                }
            };

            out.push(DecryptedContactInfo {
                doc_id: *doc_id,
                revision: doc.revision().unwrap_or(dpp::document::INITIAL_REVISION),
                derivation_index,
                contact_id,
                data,
            });
        }
        Ok(out)
    }

    /// Sync `contactInfo` documents for every wallet-owned identity:
    /// fetch, decrypt, and apply alias/note/hidden onto the matching
    /// established contacts. Remote state wins — local edits publish
    /// immediately (see [`Self::set_contact_info_with_external_signer`]),
    /// so convergence is last-writer through Platform.
    ///
    /// Returns the number of contacts whose metadata was applied.
    pub async fn sync_contact_infos(&self) -> Result<u32, PlatformWalletError> {
        let identity_ids: Vec<Identifier> = {
            let wm = self.wallet_manager.read().await;
            let Some(info) = wm.get_wallet_info(&self.wallet_id) else {
                return Ok(0);
            };
            info.identity_manager
                .wallet_identities
                .values()
                .flat_map(|inner| inner.values().map(|m| m.id()))
                .collect()
        };

        let mut applied = 0u32;
        for identity_id in identity_ids {
            // Log-and-continue per identity, matching the other sync steps.
            let infos = match self.fetch_decrypted_contact_infos(&identity_id).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        identity = %identity_id,
                        error = %e,
                        "contactInfo sync failed for identity; continuing"
                    );
                    continue;
                }
            };
            if infos.is_empty() {
                continue;
            }
            let mut wm = self.wallet_manager.write().await;
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                continue;
            };
            let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) else {
                continue;
            };
            for decrypted in infos {
                if managed.set_contact_metadata(
                    &decrypted.contact_id,
                    decrypted.data.alias_name.clone(),
                    decrypted.data.note.clone(),
                    decrypted.data.display_hidden,
                    &self.persister,
                ) {
                    applied += 1;
                }
            }
        }
        Ok(applied)
    }

    /// Set alias / note / hidden for an established contact, persist
    /// locally, and publish (create or update) the corresponding
    /// `contactInfo` document on Platform.
    ///
    /// DIP-15 privacy rule: with fewer than two established contacts
    /// the document write is skipped (a single contactInfo would be
    /// trivially linkable to the pair's contactRequest); the local
    /// state still updates and the next edit after the second contact
    /// is established publishes normally.
    pub async fn set_contact_info_with_external_signer<S>(
        &self,
        identity_id: &Identifier,
        contact_id: &Identifier,
        alias: Option<String>,
        note: Option<String>,
        display_hidden: bool,
        signer: &S,
    ) -> Result<(), PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        use dashcore::secp256k1::rand::{thread_rng, RngCore};
        use dpp::data_contract::accessors::v0::DataContractV0Getters;


        // 1. Local state first — works offline and feeds SwiftData.
        let (established_count, identity_index, signing_key, root_key_id, wallet_snapshot) = {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity_mut(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            if !managed.set_contact_metadata(
                contact_id,
                alias.clone(),
                note.clone(),
                display_hidden,
                &self.persister,
            ) {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Contact {contact_id} is not established for identity {identity_id}"
                )));
            }
            let established_count = managed.established_contacts.len();
            let identity_index = managed.identity_index;
            let signing_key = managed
                .identity
                .get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    [SecurityLevel::HIGH, SecurityLevel::CRITICAL].into(),
                    [KeyType::ECDSA_SECP256K1].into(),
                    false,
                )
                .cloned();
            let root_key_id = managed
                .identity
                .public_keys()
                .iter()
                .find(|(_, k)| {
                    k.purpose() == Purpose::ENCRYPTION && k.key_type() == KeyType::ECDSA_SECP256K1
                })
                .map(|(_, k)| k.id());
            drop(wm);
            let wm = self.wallet_manager.read().await;
            let wallet = wm
                .get_wallet(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?
                .clone();
            (
                established_count,
                identity_index,
                signing_key,
                root_key_id,
                wallet,
            )
        };

        // 2. DIP-15 privacy gate.
        if established_count < 2 {
            tracing::info!(
                identity = %identity_id,
                contact = %contact_id,
                established_count,
                "contactInfo publish deferred (DIP-15: needs ≥2 established contacts); local state updated"
            );
            return Ok(());
        }

        let Some(identity_index) = identity_index else {
            tracing::info!(
                identity = %identity_id,
                "contactInfo publish skipped for watch-only/seedless identity (G4 pending); local state updated"
            );
            return Ok(());
        };
        let signing_key = signing_key.ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "No HIGH or CRITICAL authentication key found on identity \
                 (required for document state transitions)"
                    .to_string(),
            )
        })?;
        let root_key_id = root_key_id.ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Identity has no ECDSA_SECP256K1 encryption key (required for contactInfo)"
                    .to_string(),
            )
        })?;

        // 3. Resolve the existing doc for this contact (stateless: by
        //    decrypting encToUserId of each owned doc) or pick the next
        //    sequential derivation index for a fresh one.
        let existing = self.fetch_decrypted_contact_infos(identity_id).await?;
        let (doc_id, revision, derivation_index) = match existing
            .iter()
            .find(|d| d.contact_id == *contact_id)
        {
            Some(d) => (Some(d.doc_id), d.revision + 1, d.derivation_index),
            None => {
                let next_index = existing
                    .iter()
                    .map(|d| d.derivation_index + 1)
                    .max()
                    .unwrap_or(0);
                (None, dpp::document::INITIAL_REVISION, next_index)
            }
        };

        // 4. Encrypt the payload.
        let keys = derive_contact_info_keys(
            &wallet_snapshot,
            self.sdk.network,
            identity_index,
            root_key_id,
            derivation_index,
        )?;
        let enc_to_user_id =
            platform_encryption::encrypt_enc_to_user_id(&keys.enc_to_user_id_key, &contact_id.to_buffer());
        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);
        let private_data = platform_encryption::encrypt_private_data(
            &keys.private_data_key,
            &iv,
            &encode_private_data(&ContactInfoPrivateData {
                alias_name: alias,
                note,
                display_hidden,
            }),
        );

        // 5. Build + put the document through the write seam.
        let mut properties = std::collections::BTreeMap::new();
        properties.insert(
            "encToUserId".to_string(),
            Value::Bytes32(enc_to_user_id),
        );
        properties.insert("rootEncryptionKeyIndex".to_string(), Value::U32(root_key_id));
        properties.insert(
            "derivationEncryptionKeyIndex".to_string(),
            Value::U32(derivation_index),
        );
        properties.insert("privateData".to_string(), Value::Bytes(private_data));

        let document = Document::V0(DocumentV0 {
            id: doc_id.unwrap_or_else(|| Identifier::from([0u8; 32])),
            owner_id: *identity_id,
            properties,
            revision: if doc_id.is_some() {
                Some(revision)
            } else {
                None
            },
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

        let dashpay_contract = super::dashpay_contract()?;
        let document_type = dashpay_contract
            .document_type_for_name("contactInfo")
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to get contactInfo document type: {e}"
                ))
            })?
            .to_owned_document_type();

        self.sdk_writer
            .put_document(super::sdk_writer::PutDocumentParams {
                document,
                document_type,
                signing_public_key: signing_key,
                signer: signer as &(dyn Signer<IdentityPublicKey> + Send + Sync),
            })
            .await?;

        tracing::info!(
            identity = %identity_id,
            contact = %contact_id,
            derivation_index,
            updated = doc_id.is_some(),
            "Published contactInfo document"
        );
        Ok(())
    }
}
