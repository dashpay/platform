use crate::drive::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{DocumentRefAndSerialization, DocumentRefInfo};
use crate::drive::object_size_info::{
    DataContractInfo, DocumentAndContractInfo, DocumentTypeInfo, OwnedDocumentInfo,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::prelude::Identifier;

use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;

use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::HashMap;

/// A wrapper for a document operation
#[derive(Clone, Debug)]
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
            DocumentOperationType::AddWithdrawalDocument {
                owned_document_info,
            } => {
                let contract = drive.cache.system_data_contracts.load_withdrawals();

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
