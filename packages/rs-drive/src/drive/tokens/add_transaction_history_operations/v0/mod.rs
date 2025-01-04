use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::tokens::token_event::TokenEvent;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeMap, HashMap};

impl Drive {
    /// Adds token transaction history
    pub(super) fn add_token_transaction_history_operations_v0(
        &self,
        token_id: Identifier,
        owner_id: Identifier,
        owner_nonce: IdentityNonce,
        event: TokenEvent,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let operations;

        let contract = self.cache.system_data_contracts.load_token_history();

        match event {
            TokenEvent::Mint(mint_amount, recipient_id, public_note) => {
                let document_type = contract.document_type_for_name("mint")?;
                let document_id = Document::generate_document_id_v0(
                    &contract.id(),
                    &owner_id,
                    "mint",
                    owner_nonce.to_be_bytes().as_slice(),
                );
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("recipientId".to_string(), recipient_id.into()),
                    ("amount".to_string(), mint_amount.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                let document: Document = DocumentV0 {
                    id: document_id,
                    owner_id,
                    properties,
                    revision: None,
                    created_at: Some(block_info.time_ms),
                    updated_at: None,
                    transferred_at: None,
                    created_at_block_height: Some(block_info.height),
                    updated_at_block_height: None,
                    transferred_at_block_height: None,
                    created_at_core_block_height: Some(block_info.core_height),
                    updated_at_core_block_height: None,
                    transferred_at_core_block_height: None,
                }
                .into();
                operations = self.add_document_for_contract_operations(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentOwnedInfo((document, None)),
                            owner_id: Some(owner_id.to_buffer()),
                        },
                        contract: &contract,
                        document_type,
                    },
                    true,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
            }
            TokenEvent::Burn(burn_amount, public_note) => {
                let document_type = contract.document_type_for_name("burn")?;
                let document_id = Document::generate_document_id_v0(
                    &contract.id(),
                    &owner_id,
                    "burn",
                    owner_nonce.to_be_bytes().as_slice(),
                );
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("amount".to_string(), burn_amount.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("note".to_string(), note.into());
                }
                let document: Document = DocumentV0 {
                    id: document_id,
                    owner_id,
                    properties,
                    revision: None,
                    created_at: Some(block_info.time_ms),
                    updated_at: None,
                    transferred_at: None,
                    created_at_block_height: Some(block_info.height),
                    updated_at_block_height: None,
                    transferred_at_block_height: None,
                    created_at_core_block_height: Some(block_info.core_height),
                    updated_at_core_block_height: None,
                    transferred_at_core_block_height: None,
                }
                .into();
                operations = self.add_document_for_contract_operations(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentOwnedInfo((document, None)),
                            owner_id: Some(owner_id.to_buffer()),
                        },
                        contract: &contract,
                        document_type,
                    },
                    true,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
            }
            TokenEvent::Transfer(
                to,
                public_note,
                token_event_shared_encrypted_note,
                token_event_personal_encrypted_note,
                amount,
            ) => {
                let document_type = contract.document_type_for_name("transfer")?;
                let document_id = Document::generate_document_id_v0(
                    &contract.id(),
                    &owner_id,
                    "transfer",
                    owner_nonce.to_be_bytes().as_slice(),
                );
                let mut properties = BTreeMap::from([
                    ("tokenId".to_string(), token_id.into()),
                    ("amount".to_string(), amount.into()),
                    ("toIdentityId".to_string(), to.into()),
                ]);
                if let Some(note) = public_note {
                    properties.insert("publicNote".to_string(), note.into());
                }
                if let Some((sender_key_index, recipient_key_index, note)) =
                    token_event_shared_encrypted_note
                {
                    properties.insert("encryptedSharedNote".to_string(), note.into());
                    properties.insert("senderKeyIndex".to_string(), sender_key_index.into());
                    properties.insert("recipientKeyIndex".to_string(), recipient_key_index.into());
                }

                if let Some((root_encryption_key_index, derivation_encryption_key_index, note)) =
                    token_event_personal_encrypted_note
                {
                    properties.insert("encryptedPersonalNote".to_string(), note.into());
                    properties.insert(
                        "rootEncryptionKeyIndex".to_string(),
                        root_encryption_key_index.into(),
                    );
                    properties.insert(
                        "derivationEncryptionKeyIndex".to_string(),
                        derivation_encryption_key_index.into(),
                    );
                }
                let document: Document = DocumentV0 {
                    id: document_id,
                    owner_id,
                    properties,
                    revision: None,
                    created_at: Some(block_info.time_ms),
                    updated_at: None,
                    transferred_at: None,
                    created_at_block_height: Some(block_info.height),
                    updated_at_block_height: None,
                    transferred_at_block_height: None,
                    created_at_core_block_height: Some(block_info.core_height),
                    updated_at_core_block_height: None,
                    transferred_at_core_block_height: None,
                }
                .into();
                operations = self.add_document_for_contract_operations(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentOwnedInfo((document, None)),
                            owner_id: Some(owner_id.to_buffer()),
                        },
                        contract: &contract,
                        document_type,
                    },
                    true,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
            }
        }

        Ok(operations)
    }
}
