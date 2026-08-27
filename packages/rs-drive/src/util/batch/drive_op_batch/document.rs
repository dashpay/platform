use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::util::object_size_info::DocumentInfo::{DocumentRefAndSerialization, DocumentRefInfo};
use crate::util::object_size_info::{
    DataContractInfo, DocumentAndContractInfo, DocumentTypeInfo, OwnedDocumentInfo,
};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::document_event::DocumentEvent;
use dpp::document::Document;
use dpp::prelude::{Identifier, IdentityNonce};

use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

/// A wrapper for a document operation
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum DocumentOperation<'a> {
    /// An add operation
    AddOperation {
        /// Document info with maybe the owner id
        owned_document_info: OwnedDocumentInfo<'a>,
        /// Should we override the document if one already exists?
        override_document: bool,
    },
    /// An update operation
    UpdateOperation(UpdateOperationInfo<'a>),
}

/// Document and contract info
#[derive(Clone, Debug)]
pub struct DocumentOperationsForContractDocumentType<'a> {
    /// Document info
    pub operations: Vec<DocumentOperation<'a>>,
    ///DataContract
    pub contract: &'a DataContract,
    /// Document type
    pub document_type: DocumentTypeRef<'a>,
}

/// Operations on Documents
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum DocumentOperationType<'a> {
    /// Adds a document to a contract matching the desired info.
    AddDocument {
        /// The document and contract info, also may contain the owner_id
        owned_document_info: OwnedDocumentInfo<'a>,
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Document type
        document_type_info: DocumentTypeInfo<'a>,
        /// Should we override the document if one already exists?
        override_document: bool,
    },
    /// Adds a contested document to a contract matching the desired info.
    /// A contested document is a document that is trying to a acquire a
    /// unique index that has a conflict resolution mechanism
    AddContestedDocument {
        /// The document and contract info, also may contain the owner_id
        owned_document_info: OwnedDocumentInfo<'a>,
        /// The vote poll in question that will should be created
        contested_document_resource_vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Document type
        document_type_info: DocumentTypeInfo<'a>,
        /// Should we insert without verifying first that the document doesn't already exist
        insert_without_check: bool,
        /// Should we also insert the vote poll stored info
        also_insert_vote_poll_stored_info: Option<ContestedDocumentVotePollStoredInfo>,
    },
    /// Updates a document and returns the associated fee.
    UpdateDocument {
        /// The document and contract info, also may contain the owner_id
        owned_document_info: OwnedDocumentInfo<'a>,
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Document type
        document_type_info: DocumentTypeInfo<'a>,
    },
    /// Deletes a document
    DeleteDocument {
        /// The document id
        document_id: Identifier,
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Document type
        document_type_info: DocumentTypeInfo<'a>,
    },
    /// Deletes an indexOnly document from its property values — there is
    /// no primary-storage row to fetch, so the values (plus the owner)
    /// are what every index entry is recomputed from. `$createdAt` may
    /// ride in `data` under its system key when the type indexes it.
    DeleteIndexOnlyDocument {
        /// The document id (deterministic; never stored)
        document_id: Identifier,
        /// The owner whose entries are being removed
        owner_id: Identifier,
        /// The document's property values
        data: BTreeMap<String, Value>,
        /// Data Contract info to potentially be resolved if needed
        contract_info: DataContractInfo<'a>,
        /// Document type
        document_type_info: DocumentTypeInfo<'a>,
    },
    /// Convenience method to add a withdrawal document.
    AddWithdrawalDocument {
        /// The document and contract info, also may contain the owner_id
        owned_document_info: OwnedDocumentInfo<'a>,
    },
    /// Adds a document to a contract.
    MultipleDocumentOperationsForSameContractDocumentType {
        /// The document operations
        document_operations: DocumentOperationsForContractDocumentType<'a>,
    },
    /// Adds a historical document to the document history system contract,
    /// recording a transfer, purchase, or price update of a document whose
    /// document type subscribed to history.
    DocumentHistory {
        /// The data contract of the source document
        source_data_contract_id: Identifier,
        /// The document type name of the source document
        source_document_type_name: String,
        /// The source document
        source_document_id: Identifier,
        /// The identity making the event
        owner_id: Identifier,
        /// The nonce
        nonce: IdentityNonce,
        /// The document event
        event: DocumentEvent,
    },
}

impl DriveLowLevelOperationConverter for DocumentOperationType<'_> {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            DocumentOperationType::AddDocument {
                owned_document_info,
                contract_info,
                document_type_info,
                override_document,
            } => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let contract_resolved_info = contract_info.resolve(
                    drive,
                    block_info,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                let contract = contract_resolved_info.as_ref();
                let document_type = document_type_info.resolve(contract)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    owned_document_info,
                    contract,
                    document_type,
                };
                let mut operations = drive.add_document_for_contract_operations(
                    document_and_contract_info,
                    override_document,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                drive_operations.append(&mut operations);
                Ok(drive_operations)
            }
            DocumentOperationType::AddContestedDocument {
                owned_document_info,
                contested_document_resource_vote_poll,
                contract_info,
                document_type_info,
                insert_without_check,
                also_insert_vote_poll_stored_info,
            } => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let contract_resolved_info = contract_info.resolve(
                    drive,
                    block_info,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                let contract = contract_resolved_info.as_ref();
                let document_type = document_type_info.resolve(contract)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    owned_document_info,
                    contract,
                    document_type,
                };
                let mut operations = drive.add_contested_document_for_contract_operations(
                    document_and_contract_info,
                    contested_document_resource_vote_poll,
                    insert_without_check,
                    block_info,
                    also_insert_vote_poll_stored_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                drive_operations.append(&mut operations);
                Ok(drive_operations)
            }
            DocumentOperationType::AddWithdrawalDocument {
                owned_document_info,
            } => {
                let contract = drive
                    .cache
                    .system_data_contracts
                    .load_withdrawals(platform_version)?;

                let document_type = contract
                    .document_type_for_name(withdrawal::NAME)
                    .map_err(ProtocolError::DataContractError)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    owned_document_info,
                    contract: &contract,
                    document_type,
                };
                drive.add_document_for_contract_operations(
                    document_and_contract_info,
                    false,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )
            }
            DocumentOperationType::UpdateDocument {
                owned_document_info,
                contract_info,
                document_type_info,
            } => {
                let mut drive_operations = vec![];
                let contract_resolved_info = contract_info.resolve(
                    drive,
                    block_info,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                let contract = contract_resolved_info.as_ref();
                let document_type = document_type_info.resolve(contract)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    owned_document_info,
                    contract,
                    document_type,
                };
                let mut operations = drive.update_document_for_contract_operations(
                    document_and_contract_info,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                drive_operations.append(&mut operations);
                Ok(drive_operations)
            }
            DocumentOperationType::DocumentHistory {
                source_data_contract_id,
                source_document_type_name,
                source_document_id,
                owner_id,
                nonce,
                event,
            } => {
                let batch_operations = drive.add_document_history_operations(
                    source_data_contract_id,
                    source_document_type_name.as_str(),
                    source_document_id,
                    owner_id,
                    nonce,
                    event,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;
                Ok(batch_operations)
            }
            DocumentOperationType::DeleteDocument {
                document_id,
                contract_info,
                document_type_info,
            } => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let contract_resolved_info = contract_info.resolve(
                    drive,
                    block_info,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                let contract = contract_resolved_info.as_ref();
                let document_type = document_type_info.resolve(contract)?;

                drive.delete_document_for_contract_operations(
                    document_id,
                    contract,
                    document_type,
                    None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )
            }
            DocumentOperationType::DeleteIndexOnlyDocument {
                document_id,
                owner_id,
                data,
                contract_info,
                document_type_info,
            } => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let contract_resolved_info = contract_info.resolve(
                    drive,
                    block_info,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                let contract = contract_resolved_info.as_ref();
                let document_type = document_type_info.resolve(contract)?;

                // Reconstruct the document the entries were written from.
                let document = Drive::index_only_document_from_values(document_id, owner_id, data)?;

                drive.delete_index_only_document_for_contract_operations(
                    document,
                    contract,
                    document_type,
                    None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )
            }
            DocumentOperationType::MultipleDocumentOperationsForSameContractDocumentType {
                document_operations,
            } => {
                let DocumentOperationsForContractDocumentType {
                    operations,
                    contract,
                    document_type,
                } = document_operations;

                let mut drive_operations = vec![];
                for document_operation in operations {
                    match document_operation {
                        DocumentOperation::AddOperation {
                            owned_document_info,
                            override_document,
                        } => {
                            let document_and_contract_info = DocumentAndContractInfo {
                                owned_document_info,
                                contract,
                                document_type,
                            };
                            let mut operations = drive.add_document_for_contract_operations(
                                document_and_contract_info,
                                override_document,
                                block_info,
                                &mut Some(&mut drive_operations),
                                estimated_costs_only_with_layer_info,
                                transaction,
                                platform_version,
                            )?;
                            drive_operations.append(&mut operations);
                        }
                        DocumentOperation::UpdateOperation(update_operation) => {
                            let UpdateOperationInfo {
                                document,
                                serialized_document,
                                owner_id,
                                storage_flags,
                            } = update_operation;

                            let document_info =
                                if let Some(serialized_document) = serialized_document {
                                    DocumentRefAndSerialization((
                                        document,
                                        serialized_document,
                                        storage_flags,
                                    ))
                                } else {
                                    DocumentRefInfo((document, storage_flags))
                                };
                            let document_and_contract_info = DocumentAndContractInfo {
                                owned_document_info: OwnedDocumentInfo {
                                    document_info,
                                    owner_id,
                                },
                                contract,
                                document_type,
                            };
                            let mut operations = drive.update_document_for_contract_operations(
                                document_and_contract_info,
                                block_info,
                                &mut Some(&mut drive_operations),
                                estimated_costs_only_with_layer_info,
                                transaction,
                                platform_version,
                            )?;
                            drive_operations.append(&mut operations);
                        }
                    }
                }
                Ok(drive_operations)
            }
        }
    }
}

/// A wrapper for an update operation
#[derive(Clone, Debug)]
pub struct UpdateOperationInfo<'a> {
    /// The document to update
    pub document: &'a Document,
    /// The document in pre-serialized form
    pub serialized_document: Option<&'a [u8]>,
    /// The owner id, if none is specified will try to recover from serialized document
    pub owner_id: Option<[u8; 32]>,
    /// Add storage flags (like epoch, owner id, etc)
    pub storage_flags: Option<Cow<'a, StorageFlags>>,
}
