use crate::drive::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{DocumentRefAndSerialization, DocumentRefInfo};
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::prelude::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::borrow::{Borrow, Cow};
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
    /// Contract
    pub contract: &'a DataContract,
    /// Document type
    pub document_type: &'a DocumentType,
}

/// Operations on Documents
#[derive(Clone, Debug)]
pub enum DocumentOperationType<'a> {
    /// Deserializes a document and a contract and adds the document to the contract.
    AddSerializedDocumentForSerializedContract {
        /// The serialized document
        serialized_document: &'a [u8],
        /// The serialized contract
        serialized_contract: &'a [u8],
        /// The name of the document type
        document_type_name: &'a str,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
        /// Should we override the document if one already exists?
        override_document: bool,
        /// Add storage flags (like epoch, owner id, etc)
        storage_flags: Option<Cow<'a, StorageFlags>>,
    },
    /// Deserializes a document and adds it to a contract.
    AddSerializedDocumentForContract {
        /// The serialized document
        serialized_document: &'a [u8],
        /// The contract
        contract: &'a DataContract,
        /// The name of the document type
        document_type_name: &'a str,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
        /// Should we override the document if one already exists?
        override_document: bool,
        /// Add storage flags (like epoch, owner id, etc)
        storage_flags: Option<Cow<'a, StorageFlags>>,
    },
    /// Adds a document to a contract matching the desired info.
    AddDocument {
        /// The document and contract info, also may contain the owner_id
        owned_document_info: OwnedDocumentInfo<'a>,
        /// Contract
        contract_id: Identifier,
        /// Document type
        document_type_name: Cow<'a, String>,
        /// Should we override the document if one already exists?
        override_document: bool,
    },
    /// Adds a withdrawal document.
    AddWithdrawalDocument {
        /// The document and contract info, also may contain the owner_id
        owned_document_info: OwnedDocumentInfo<'a>,
    },
    /// Adds a document to a contract.
    AddDocumentForContract {
        /// The document and contract info, also may contain the owner_id
        document_and_contract_info: DocumentAndContractInfo<'a>,
        /// Should we override the document if one already exists?
        override_document: bool,
    },
    /// Adds a document to a contract.
    MultipleDocumentOperationsForSameContractDocumentType {
        /// The document operations
        document_operations: DocumentOperationsForContractDocumentType<'a>,
    },
    /// Deletes a document and returns the associated fee.
    DeleteDocumentOfNamedTypeForContractId {
        /// The document id
        document_id: [u8; 32],
        /// The contract id
        contract_id: [u8; 32],
        /// The name of the document type
        document_type_name: Cow<'a, String>,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
    },
    /// Deletes a document and returns the associated fee.
    DeleteDocumentOfNamedTypeForContract {
        /// The document id
        document_id: [u8; 32],
        /// The contract
        contract: &'a DataContract,
        /// The name of the document type
        document_type_name: &'a str,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
    },
    /// Deletes a document and returns the associated fee.
    DeleteDocumentForContract {
        /// The document id
        document_id: [u8; 32],
        /// The contract
        contract: &'a DataContract,
        /// The name of the document type
        document_type: &'a DocumentType,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
    },
    /// Deletes a document and returns the associated fee.
    /// The contract CBOR is given instead of the contract itself.
    DeleteDocumentForContractCbor {
        /// The document id
        document_id: [u8; 32],
        /// The serialized contract
        contract_cbor: &'a [u8],
        /// The name of the document type
        document_type_name: &'a str,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
    },
    /// Updates a serialized document given a contract CBOR and returns the associated fee.
    UpdateDocumentForContractCbor {
        /// The serialized document
        serialized_document: &'a [u8],
        /// The serialized contract
        contract_cbor: &'a [u8],
        /// The name of the document type
        document_type_name: &'a str,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
        /// Add storage flags (like epoch, owner id, etc)
        storage_flags: Option<Cow<'a, StorageFlags>>,
    },
    /// Updates a serialized document and returns the associated fee.
    UpdateSerializedDocumentForContract {
        /// The serialized document
        serialized_document: &'a [u8],
        /// The contract
        contract: &'a DataContract,
        /// The name of the document type
        document_type_name: &'a str,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
        /// Add storage flags (like epoch, owner id, etc)
        storage_flags: Option<Cow<'a, StorageFlags>>,
    },
    /// Updates a document and returns the associated fee.
    UpdateDocument {
        /// The document and contract info, also may contain the owner_id
        owned_document_info: OwnedDocumentInfo<'a>,
        /// Contract
        contract_id: Identifier,
        /// Document type
        document_type_name: Cow<'a, String>,
    },
    /// Updates a document and returns the associated fee.
    UpdateDocumentForContract {
        /// The document to update
        document: &'a Document,
        /// The document in pre-serialized form
        serialized_document: &'a [u8],
        /// The contract
        contract: &'a DataContract,
        /// The name of the document type
        document_type_name: &'a str,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
        /// Add storage flags (like epoch, owner id, etc)
        storage_flags: Option<Cow<'a, StorageFlags>>,
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
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            DocumentOperationType::AddSerializedDocumentForSerializedContract {
                serialized_document,
                serialized_contract,
                document_type_name,
                owner_id,
                override_document,
                storage_flags,
            } => {
                let contract = DataContract::from_cbor(serialized_contract)?;

                let document = Document::from_cbor(serialized_document, None, owner_id)?;

                let document_info =
                    DocumentRefAndSerialization((&document, serialized_document, storage_flags));

                let document_type = contract.document_type_for_name(document_type_name)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info,
                        owner_id,
                    },
                    contract: &contract,
                    document_type,
                };
                drive.add_document_for_contract_operations(
                    document_and_contract_info,
                    override_document,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                )
            }
            DocumentOperationType::AddSerializedDocumentForContract {
                serialized_document,
                contract,
                document_type_name,
                owner_id,
                override_document,
                storage_flags,
            } => {
                let document = Document::from_cbor(serialized_document, None, owner_id)?;

                let document_info =
                    DocumentRefAndSerialization((&document, serialized_document, storage_flags));

                let document_type = contract.document_type_for_name(document_type_name)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info,
                        owner_id,
                    },
                    contract,
                    document_type,
                };
                drive.add_document_for_contract_operations(
                    document_and_contract_info,
                    override_document,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                )
            }
            DocumentOperationType::AddDocument {
                owned_document_info,
                contract_id,
                document_type_name,
                override_document,
            } => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let contract_fetch_info = drive
                    .get_contract_with_fetch_info_and_add_to_operations(
                        contract_id.into_buffer(),
                        Some(&block_info.epoch),
                        true,
                        transaction,
                        &mut drive_operations,
                    )?
                    .ok_or(Error::Document(DocumentError::ContractNotFound))?;

                let contract = &contract_fetch_info.contract;

                let document_type = contract.document_type_for_name(document_type_name.as_str())?;

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
                )?;
                drive_operations.append(&mut operations);
                Ok(drive_operations)
            }
            DocumentOperationType::AddWithdrawalDocument {
                owned_document_info,
            } => {
                let contract = &drive.system_contracts.withdrawal_contract;

                let document_type = contract.document_type_for_name(
                    dpp::contracts::withdrawals_contract::document_types::WITHDRAWAL,
                )?;

                let document_and_contract_info = DocumentAndContractInfo {
                    owned_document_info,
                    contract,
                    document_type,
                };
                drive.add_document_for_contract_operations(
                    document_and_contract_info,
                    false,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                )
            }
            DocumentOperationType::AddDocumentForContract {
                document_and_contract_info,
                override_document,
            } => drive.add_document_for_contract_operations(
                document_and_contract_info,
                override_document,
                block_info,
                &mut None,
                estimated_costs_only_with_layer_info,
                transaction,
            ),
            DocumentOperationType::DeleteDocumentForContract {
                document_id,
                contract,
                document_type,
                owner_id,
            } => drive.delete_document_for_contract_operations(
                document_id,
                contract.borrow(),
                document_type,
                owner_id,
                None,
                estimated_costs_only_with_layer_info,
                transaction,
            ),
            DocumentOperationType::DeleteDocumentOfNamedTypeForContractId {
                document_id,
                contract_id,
                document_type_name,
                owner_id,
            } => drive.delete_document_for_contract_id_with_named_type_operations(
                document_id,
                contract_id,
                document_type_name.as_str(),
                owner_id,
                &block_info.epoch,
                None,
                estimated_costs_only_with_layer_info,
                transaction,
            ),
            DocumentOperationType::DeleteDocumentOfNamedTypeForContract {
                document_id,
                contract,
                document_type_name,
                owner_id,
            } => drive.delete_document_for_contract_with_named_type_operations(
                document_id,
                contract.borrow(),
                document_type_name,
                owner_id,
                None,
                estimated_costs_only_with_layer_info,
                transaction,
            ),
            DocumentOperationType::DeleteDocumentForContractCbor {
                document_id,
                contract_cbor,
                document_type_name,
                owner_id,
            } => {
                let contract = DataContract::from_cbor(contract_cbor)?;
                drive.delete_document_for_contract_with_named_type_operations(
                    document_id,
                    &contract,
                    document_type_name,
                    owner_id,
                    None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                )
            }
            DocumentOperationType::UpdateDocumentForContractCbor {
                serialized_document,
                contract_cbor,
                document_type_name,
                owner_id,
                storage_flags,
            } => {
                let contract = DataContract::from_cbor(contract_cbor)?;

                let document = Document::from_cbor(serialized_document, None, owner_id)?;

                let document_info =
                    DocumentRefSerialization((&document, serialized_document, storage_flags));

                let document_type = contract.document_type_for_name(document_type_name)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info,
                        owner_id,
                    },
                    contract: &contract,
                    document_type,
                };
                drive.update_document_for_contract_operations(
                    document_and_contract_info,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                )
            }
            DocumentOperationType::UpdateSerializedDocumentForContract {
                serialized_document,
                contract,
                document_type_name,
                owner_id,
                storage_flags,
            } => {
                let document = Document::from_cbor(serialized_document, None, owner_id)?;

                let document_info =
                    DocumentRefAndSerialization((&document, serialized_document, storage_flags));

                let document_type = contract.document_type_for_name(document_type_name)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info,
                        owner_id,
                    },
                    contract,
                    document_type,
                };
                drive.update_document_for_contract_operations(
                    document_and_contract_info,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
                )
            }
            DocumentOperationType::UpdateDocumentForContract {
                document,
                serialized_document,
                contract,
                document_type_name,
                owner_id,
                storage_flags,
            } => {
                let document_info =
                    DocumentRefAndSerialization((document, serialized_document, storage_flags));

                let document_type = contract.document_type_for_name(document_type_name)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info,
                        owner_id,
                    },
                    contract,
                    document_type,
                };
                drive.update_document_for_contract_operations(
                    document_and_contract_info,
                    block_info,
                    &mut None,
                    estimated_costs_only_with_layer_info,
                    transaction,
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
                            )?;
                            drive_operations.append(&mut operations);
                        }
                    }
                }
                Ok(drive_operations)
            }
            DocumentOperationType::UpdateDocument {
                owned_document_info,
                contract_id,
                document_type_name,
            } => {
                let mut drive_operations = vec![];
                let contract_fetch_info = drive
                    .get_contract_with_fetch_info_and_add_to_operations(
                        contract_id.into_buffer(),
                        Some(&block_info.epoch),
                        true,
                        transaction,
                        &mut drive_operations,
                    )?
                    .ok_or(Error::Document(DocumentError::ContractNotFound))?;

                let contract = &contract_fetch_info.contract;

                let document_type = contract.document_type_for_name(document_type_name.as_str())?;

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
                )?;
                drive_operations.append(&mut operations);
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
