// MIT LICENSE
//
// Copyright (c) 2022 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

use crate::contract::Contract;
use crate::drive::block_info::BlockInfo;
use crate::drive::flags::StorageFlags;
use crate::drive::grove_operations::BatchDeleteApplyType;
use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path, get_withdrawal_transactions_expired_ids_path,
    get_withdrawal_transactions_expired_ids_path_as_u8, get_withdrawal_transactions_queue_path,
    WithdrawalTransaction, WITHDRAWAL_TRANSACTIONS_COUNTER_ID,
};
use crate::drive::object_size_info::DocumentInfo::{
    DocumentRefAndSerialization, DocumentRefWithoutSerialization,
};
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;
use crate::fee::result::FeeResult;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DriveContractExt;
use dpp::document::document_stub::DocumentStub;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// A converter that will get Drive Operations from High Level Operations
pub trait DriveOperationConverter {
    /// This will get a list of atomic drive operations from a high level operations
    fn to_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error>;
}

/// Operations on Contracts
pub enum ContractOperationType<'a> {
    /// Deserializes a contract from CBOR and applies it.
    ApplyContractCbor {
        /// The cbor serialized contract
        contract_cbor: Vec<u8>,
        /// The contract id, if it is not present will try to recover it from the contract
        contract_id: Option<[u8; 32]>,
        /// Storage flags for the contract
        storage_flags: Option<&'a StorageFlags>,
    },
    /// Applies a contract and returns the fee for applying.
    /// If the contract already exists, an update is applied, otherwise an insert.
    ApplyContractWithSerialization {
        /// The contract
        contract: &'a Contract,
        /// The serialized contract
        serialized_contract: Vec<u8>,
        /// Storage flags for the contract
        storage_flags: Option<&'a StorageFlags>,
    },
}

impl DriveOperationConverter for ContractOperationType<'_> {
    fn to_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        match self {
            ContractOperationType::ApplyContractCbor {
                contract_cbor,
                contract_id,
                storage_flags,
            } => {
                // first we need to deserialize the contract
                let contract =
                    <Contract as DriveContractExt>::from_cbor(&contract_cbor, contract_id)?;

                drive.apply_contract_operations(
                    &contract,
                    contract_cbor,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    storage_flags,
                    transaction,
                )
            }
            ContractOperationType::ApplyContractWithSerialization {
                contract,
                serialized_contract: contract_serialization,
                storage_flags,
            } => drive.apply_contract_operations(
                contract,
                contract_serialization,
                block_info,
                estimated_costs_only_with_layer_info,
                storage_flags,
                transaction,
            ),
        }
    }
}

/// A wrapper for an update operation
pub struct UpdateOperationInfo<'a> {
    /// The document to update
    pub document: &'a DocumentStub,
    /// The document in pre-serialized form
    pub serialized_document: Option<&'a [u8]>,
    /// The owner id, if none is specified will try to recover from serialized document
    pub owner_id: Option<[u8; 32]>,
    /// Add storage flags (like epoch, owner id, etc)
    pub storage_flags: Option<&'a StorageFlags>,
}

/// A wrapper for a document operation
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
pub struct DocumentOperationsForContractDocumentType<'a> {
    /// Document info
    pub operations: Vec<DocumentOperation<'a>>,
    /// Contract
    pub contract: &'a Contract,
    /// Document type
    pub document_type: &'a DocumentType,
}

/// Operations on Documents
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
        storage_flags: Option<&'a StorageFlags>,
    },
    /// Deserializes a document and adds it to a contract.
    AddSerializedDocumentForContract {
        /// The serialized document
        serialized_document: &'a [u8],
        /// The contract
        contract: &'a Contract,
        /// The name of the document type
        document_type_name: &'a str,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
        /// Should we override the document if one already exists?
        override_document: bool,
        /// Add storage flags (like epoch, owner id, etc)
        storage_flags: Option<&'a StorageFlags>,
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
    DeleteDocumentForContract {
        /// The document id
        document_id: [u8; 32],
        /// The contract
        contract: &'a Contract,
        /// The name of the document type
        document_type_name: &'a str,
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
        storage_flags: Option<&'a StorageFlags>,
    },
    /// Updates a serialized document and returns the associated fee.
    UpdateSerializedDocumentForContract {
        /// The serialized document
        serialized_document: &'a [u8],
        /// The contract
        contract: &'a Contract,
        /// The name of the document type
        document_type_name: &'a str,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
        /// Add storage flags (like epoch, owner id, etc)
        storage_flags: Option<&'a StorageFlags>,
    },
    /// Updates a document and returns the associated fee.
    UpdateDocumentForContract {
        /// The document to update
        document: &'a DocumentStub,
        /// The document in pre-serialized form
        serialized_document: &'a [u8],
        /// The contract
        contract: &'a Contract,
        /// The name of the document type
        document_type_name: &'a str,
        /// The owner id, if none is specified will try to recover from serialized document
        owner_id: Option<[u8; 32]>,
        /// Add storage flags (like epoch, owner id, etc)
        storage_flags: Option<&'a StorageFlags>,
    },
}

impl DriveOperationConverter for DocumentOperationType<'_> {
    fn to_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        match self {
            DocumentOperationType::AddSerializedDocumentForSerializedContract {
                serialized_document,
                serialized_contract,
                document_type_name,
                owner_id,
                override_document,
                storage_flags,
            } => {
                let contract =
                    <Contract as DriveContractExt>::from_cbor(serialized_contract, None)?;

                let document = DocumentStub::from_cbor(serialized_document, None, owner_id)?;

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
                let document = DocumentStub::from_cbor(serialized_document, None, owner_id)?;

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
                document_type_name,
                owner_id,
            } => drive.delete_document_for_contract_operations(
                document_id,
                contract,
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
                let contract = <Contract as DriveContractExt>::from_cbor(contract_cbor, None)?;
                drive.delete_document_for_contract_operations(
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
                let contract = <Contract as DriveContractExt>::from_cbor(contract_cbor, None)?;

                let document = DocumentStub::from_cbor(serialized_document, None, owner_id)?;

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
                let document = DocumentStub::from_cbor(serialized_document, None, owner_id)?;

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

                            let document_cbor = document.to_cbor();

                            // If serialized document not submitted updates are not performed
                            // TODO: figure that out
                            let serialized_document = serialized_document.or(Some(&document_cbor));

                            let document_info =
                                if let Some(serialized_document) = serialized_document {
                                    DocumentRefAndSerialization((
                                        document,
                                        serialized_document,
                                        storage_flags,
                                    ))
                                } else {
                                    DocumentRefWithoutSerialization((document, storage_flags))
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
        }
    }
}
//
// /// Operations on Identities
// pub enum IdentityOperationType<'a> {
//     /// Inserts a new identity to the `Identities` subtree.
//     InsertIdentity {
//         /// The identity we wish to insert
//         identity: Identity,
//         /// Add storage flags (like epoch, owner id, etc)
//         storage_flags: Option<&'a StorageFlags>,
//     },
// }
//
// impl DriveOperationConverter for IdentityOperationType<'_> {
//     fn to_grove_db_operations(
//         self,
//         drive: &Drive,
//         apply: bool,
//         block_info: &BlockInfo,
//         transaction: TransactionArg,
//     ) -> Result<Vec<DriveOperation>, Error> {
//         match self {
//             IdentityOperationType::InsertIdentity {
//                 identity,
//                 storage_flags,
//             } => {
//                 drive.insert_identity(identity, block_info, apply, storage_flags, transaction)
//             }
//         }
//     }
// }

/// Operations for Withdrawals
pub enum WithdrawalOperationType<'a> {
    /// Inserts expired index into it's tree
    InsertExpiredIndex {
        /// index value
        index: u64,
    },
    /// Removes expired index from the tree
    DeleteExpiredIndex {
        /// index value
        key: &'a [u8],
    },
    /// Update index counter
    UpdateIndexCounter {
        /// index counter value
        index: u64,
    },
    /// Insert Core Transaction into queue
    InsertTransactions {
        /// transaction id bytes
        transactions: &'a [WithdrawalTransaction],
    },
}

impl DriveOperationConverter for WithdrawalOperationType<'_> {
    fn to_drive_operations(
        self,
        drive: &Drive,
        _estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        match self {
            WithdrawalOperationType::InsertExpiredIndex { index } => {
                let mut drive_operations = vec![];

                let index_bytes = index.to_be_bytes();

                let path = get_withdrawal_transactions_expired_ids_path();

                drive.batch_insert(
                    crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement::<'_, 1>((
                        path,
                        &index_bytes,
                        Element::Item(vec![], None),
                    )),
                    &mut drive_operations,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::DeleteExpiredIndex { key } => {
                let mut drive_operations = vec![];

                let path: [&[u8]; 2] = get_withdrawal_transactions_expired_ids_path_as_u8();

                drive.batch_delete(
                    path,
                    key,
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some((false, false)),
                    },
                    transaction,
                    &mut drive_operations,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::UpdateIndexCounter { index } => {
                let mut drive_operations = vec![];

                let path = get_withdrawal_root_path();

                drive.batch_insert(
                    crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement::<'_, 1>((
                        path,
                        &WITHDRAWAL_TRANSACTIONS_COUNTER_ID,
                        Element::Item(index.to_be_bytes().to_vec(), None),
                    )),
                    &mut drive_operations,
                )?;

                Ok(drive_operations)
            }
            WithdrawalOperationType::InsertTransactions { transactions } => {
                let mut drive_operations = vec![];

                let path = get_withdrawal_transactions_queue_path();

                for (id, bytes) in transactions {
                    drive.batch_insert(
                        crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement::<'_, 1>(
                            (path.clone(), id, Element::Item(bytes.clone(), None)),
                        ),
                        &mut drive_operations,
                    )?;
                }

                Ok(drive_operations)
            }
        }
    }
}

/// All types of Drive Operations
pub enum DriveOperationType<'a> {
    /// A contract operation
    ContractOperation(ContractOperationType<'a>),
    /// A document operation
    DocumentOperation(DocumentOperationType<'a>),
    // /// An identity operation
    // IdentityOperation(IdentityOperationType<'a>),
    /// Withdrawal operation
    WithdrawalOperation(WithdrawalOperationType<'a>),
}

impl DriveOperationConverter for DriveOperationType<'_> {
    fn to_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        match self {
            DriveOperationType::ContractOperation(contract_operation_type) => {
                contract_operation_type.to_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                )
            }
            DriveOperationType::DocumentOperation(document_operation_type) => {
                document_operation_type.to_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                )
            }
            DriveOperationType::WithdrawalOperation(withdrawal_operation_type) => {
                withdrawal_operation_type.to_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                )
            } // DriveOperationType::IdentityOperation(identity_operation_type) => {
              //     identity_operation_type.to_grove_db_operations(
              //         drive,
              //         apply,
              //         block_info,
              //         transaction,
              //     )
              // }
        }
    }
}

impl Drive {
    /// We can apply multiple operations at once
    pub fn apply_drive_operations(
        &self,
        operations: Vec<DriveOperationType>,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        for drive_op in operations {
            drive_operations.append(&mut drive_op.to_drive_operations(
                self,
                &mut estimated_costs_only_with_layer_info,
                block_info,
                transaction,
            )?);
        }
        let mut cost_operations = vec![];
        self.apply_batch_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            drive_operations,
            &mut cost_operations,
        )?;
        calculate_fee(None, Some(cost_operations), &block_info.epoch)
    }
}

#[cfg(test)]
mod tests {
    use grovedb::Element;
    use std::option::Option::None;

    use super::*;
    use crate::common;
    use dpp::data_contract::extra::common::json_document_to_cbor;
    use dpp::util::serializer;
    use rand::Rng;
    use serde_json::json;
    use tempfile::TempDir;

    use crate::common::setup_contract;
    use crate::drive::batch::drive_op_batch::DocumentOperation::{AddOperation, UpdateOperation};
    use crate::drive::batch::ContractOperationType::ApplyContractWithSerialization;
    use crate::drive::batch::DocumentOperationType::{
        AddSerializedDocumentForContract, MultipleDocumentOperationsForSameContractDocumentType,
    };
    use crate::drive::batch::DriveOperationType::{ContractOperation, DocumentOperation};
    use crate::drive::config::DriveConfig;
    use crate::drive::contract::paths::contract_root_path;
    use crate::drive::flags::StorageFlags;
    use crate::drive::Drive;

    #[test]
    fn test_add_dashpay_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            Some(crate::drive::defaults::PROTOCOL_VERSION),
        )
        .expect("expected to get cbor contract");
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");
        let serialized_contract =
            DriveContractExt::to_cbor(&contract).expect("contract should be serialized");

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        drive_operations.push(ContractOperation(ApplyContractWithSerialization {
            contract: &contract,
            serialized_contract: serialized_contract.clone(),
            storage_flags: None,
        }));

        let dashpay_cr_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        )
        .expect("expected to get cbor contract");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        drive_operations.push(DocumentOperation(AddSerializedDocumentForContract {
            serialized_document: dashpay_cr_serialized_document.as_slice(),
            contract: &contract,
            document_type_name: "contactRequest",
            owner_id: Some(random_owner_id),
            override_document: false,
            storage_flags: StorageFlags::optional_default_as_ref(),
        }));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
            )
            .expect("expected to insert contract and document");

        let element = drive
            .grove
            .get(
                contract_root_path(&contract.id.buffer),
                &[0],
                Some(&db_transaction),
            )
            .unwrap()
            .expect("expected to get contract back");

        assert_eq!(element, Element::Item(serialized_contract, None));

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["$ownerId", "asc"],
            ]
        });
        let where_cbor =
            serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn test_add_multiple_dashpay_documents_individually_should_fail() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            Some(crate::drive::defaults::PROTOCOL_VERSION),
        )
        .expect("expected to get cbor contract");
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");
        let serialized_contract =
            DriveContractExt::to_cbor(&contract).expect("contract should be serialized");

        let _document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        drive_operations.push(ContractOperation(ApplyContractWithSerialization {
            contract: &contract,
            serialized_contract,
            storage_flags: None,
        }));

        let dashpay_cr_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        )
        .expect("expected to get cbor contract");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        drive_operations.push(DocumentOperation(AddSerializedDocumentForContract {
            serialized_document: dashpay_cr_serialized_document.as_slice(),
            contract: &contract,
            document_type_name: "contactRequest",
            owner_id: Some(random_owner_id),
            override_document: false,
            storage_flags: StorageFlags::optional_default_as_ref(),
        }));

        let dashpay_cr_serialized_document2 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(1),
        )
        .expect("expected to get cbor contract");

        drive_operations.push(DocumentOperation(AddSerializedDocumentForContract {
            serialized_document: dashpay_cr_serialized_document2.as_slice(),
            contract: &contract,
            document_type_name: "contactRequest",
            owner_id: Some(random_owner_id),
            override_document: false,
            storage_flags: StorageFlags::optional_default_as_ref(),
        }));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
            )
            .expect_err("expected to not be able to insert documents");
    }

    #[test]
    fn test_add_multiple_dashpay_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(
            tmp_dir,
            Some(DriveConfig {
                batching_consistency_verification: true,
                ..Default::default()
            }),
        )
        .expect("expected to open Drive successfully");

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            Some(crate::drive::defaults::PROTOCOL_VERSION),
        )
        .expect("expected to get cbor contract");
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");
        let serialized_contract =
            DriveContractExt::to_cbor(&contract).expect("contract should be serialized");

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        drive_operations.push(ContractOperation(ApplyContractWithSerialization {
            contract: &contract,
            serialized_contract: serialized_contract.clone(),
            storage_flags: None,
        }));

        let dashpay_cr_serialized_document0 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        )
        .expect("expected to get cbor contract");

        let dashpay_cr_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(1),
        )
        .expect("expected to get cbor contract");

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let mut operations = vec![];

        let document0 = DocumentStub::from_cbor(
            dashpay_cr_serialized_document0.as_slice(),
            None,
            Some(random_owner_id),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization((
                    &document0,
                    dashpay_cr_serialized_document0.as_slice(),
                    StorageFlags::optional_default_as_ref(),
                )),
                owner_id: Some(random_owner_id),
            },
            override_document: false,
        });

        let document1 = DocumentStub::from_cbor(
            dashpay_cr_serialized_document1.as_slice(),
            None,
            Some(random_owner_id),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization((
                    &document1,
                    dashpay_cr_serialized_document1.as_slice(),
                    StorageFlags::optional_default_as_ref(),
                )),
                owner_id: Some(random_owner_id),
            },
            override_document: false,
        });

        drive_operations.push(DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        ));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
            )
            .expect("expected to be able to insert documents");

        let element = drive
            .grove
            .get(
                contract_root_path(&contract.id.buffer),
                &[0],
                Some(&db_transaction),
            )
            .unwrap()
            .expect("expected to get contract back");

        assert_eq!(element, Element::Item(serialized_contract, None));

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["$ownerId", "asc"],
            ]
        });
        let where_cbor =
            serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_add_multiple_family_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(
            tmp_dir,
            Some(DriveConfig {
                batching_consistency_verification: true,
                ..Default::default()
            }),
        )
        .expect("expected to open Drive successfully");

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let person_serialized_document0 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        )
        .expect("expected to get cbor document");

        let person_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person3.json",
            Some(1),
        )
        .expect("expected to get cbor document");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let mut operations = vec![];

        let document0 = DocumentStub::from_cbor(
            person_serialized_document0.as_slice(),
            None,
            Some(random_owner_id0),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization((
                    &document0,
                    person_serialized_document0.as_slice(),
                    StorageFlags::optional_default_as_ref(),
                )),
                owner_id: Some(random_owner_id0),
            },
            override_document: false,
        });

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let document1 = DocumentStub::from_cbor(
            person_serialized_document1.as_slice(),
            None,
            Some(random_owner_id1),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization((
                    &document1,
                    person_serialized_document0.as_slice(),
                    StorageFlags::optional_default_as_ref(),
                )),
                owner_id: Some(random_owner_id1),
            },
            override_document: false,
        });

        drive_operations.push(DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        ));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
            )
            .expect("expected to be able to insert documents");

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["$ownerId", "asc"],
            ]
        });
        let where_cbor =
            serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_update_multiple_family_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(
            tmp_dir,
            Some(DriveConfig {
                batching_consistency_verification: true,
                ..Default::default()
            }),
        )
        .expect("expected to open Drive successfully");

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-only-age-index.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let person_serialized_document0 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        )
        .expect("expected to get cbor document");

        let person_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person3.json",
            Some(1),
        )
        .expect("expected to get cbor document");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let mut operations = vec![];

        let document0 = DocumentStub::from_cbor(
            person_serialized_document0.as_slice(),
            None,
            Some(random_owner_id0),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization((
                    &document0,
                    person_serialized_document0.as_slice(),
                    StorageFlags::optional_default_as_ref(),
                )),
                owner_id: Some(random_owner_id0),
            },
            override_document: false,
        });

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let document1 = DocumentStub::from_cbor(
            person_serialized_document1.as_slice(),
            None,
            Some(random_owner_id1),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization((
                    &document1,
                    person_serialized_document1.as_slice(),
                    StorageFlags::optional_default_as_ref(),
                )),
                owner_id: Some(random_owner_id1),
            },
            override_document: false,
        });

        drive_operations.push(DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        ));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
            )
            .expect("expected to be able to insert documents");

        // This was the setup now let's do the update

        drive_operations = vec![];

        let person_serialized_document0 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0-older.json",
            Some(1),
        )
        .expect("expected to get cbor document");

        let person_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person3-older.json",
            Some(1),
        )
        .expect("expected to get cbor document");

        let mut operations = vec![];

        let document0 = DocumentStub::from_cbor(
            person_serialized_document0.as_slice(),
            None,
            Some(random_owner_id0),
        )
        .expect("expected to deserialize contact request");

        operations.push(UpdateOperation(UpdateOperationInfo {
            document: &document0,
            serialized_document: Some(person_serialized_document0.as_slice()),
            owner_id: Some(random_owner_id0),
            storage_flags: None,
        }));

        let document1 = DocumentStub::from_cbor(
            person_serialized_document1.as_slice(),
            None,
            Some(random_owner_id1),
        )
        .expect("expected to deserialize contact request");

        operations.push(UpdateOperation(UpdateOperationInfo {
            document: &document1,
            serialized_document: Some(person_serialized_document1.as_slice()),
            owner_id: Some(random_owner_id1),
            storage_flags: None,
        }));

        drive_operations.push(DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        ));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
            )
            .expect("expected to be able to update documents");

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor =
            serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);

        let query_value = json!({
            "where": [
                ["age", "==", 35]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor =
            serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 0);

        let query_value = json!({
            "where": [
                ["age", "==", 36]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor =
            serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_update_multiple_family_documents_with_index_being_removed_and_added() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(
            tmp_dir,
            Some(DriveConfig {
                batching_consistency_verification: true,
                ..Default::default()
            }),
        )
        .expect("expected to open Drive successfully");

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-only-age-index.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let person_serialized_document0 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        )
        .expect("expected to get cbor document");

        let person_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person3-older.json",
            Some(1),
        )
        .expect("expected to get cbor document");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let mut operations = vec![];

        let document0 = DocumentStub::from_cbor(
            person_serialized_document0.as_slice(),
            None,
            Some(random_owner_id0),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization((
                    &document0,
                    person_serialized_document0.as_slice(),
                    StorageFlags::optional_default_as_ref(),
                )),
                owner_id: Some(random_owner_id0),
            },
            override_document: false,
        });

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let document1 = DocumentStub::from_cbor(
            person_serialized_document1.as_slice(),
            None,
            Some(random_owner_id1),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization((
                    &document1,
                    person_serialized_document1.as_slice(),
                    StorageFlags::optional_default_as_ref(),
                )),
                owner_id: Some(random_owner_id1),
            },
            override_document: false,
        });

        drive_operations.push(DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        ));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
            )
            .expect("expected to be able to insert documents");

        // This was the setup now let's do the update

        drive_operations = vec![];

        let person_serialized_document0 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0-older.json",
            Some(1),
        )
        .expect("expected to get cbor document");

        let person_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person3.json",
            Some(1),
        )
        .expect("expected to get cbor document");

        let mut operations = vec![];

        let document0 = DocumentStub::from_cbor(
            person_serialized_document0.as_slice(),
            None,
            Some(random_owner_id0),
        )
        .expect("expected to deserialize contact request");

        operations.push(UpdateOperation(UpdateOperationInfo {
            document: &document0,
            serialized_document: Some(person_serialized_document0.as_slice()),
            owner_id: Some(random_owner_id0),
            storage_flags: None,
        }));

        let document1 = DocumentStub::from_cbor(
            person_serialized_document1.as_slice(),
            None,
            Some(random_owner_id1),
        )
        .expect("expected to deserialize contact request");

        operations.push(UpdateOperation(UpdateOperationInfo {
            document: &document1,
            serialized_document: Some(person_serialized_document1.as_slice()),
            owner_id: Some(random_owner_id1),
            storage_flags: None,
        }));

        drive_operations.push(DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        ));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
            )
            .expect("expected to be able to update documents");

        let query_value = json!({
            "where": [
                ["age", ">=", 5]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor =
            serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);

        let query_value = json!({
            "where": [
                ["age", "==", 35]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor =
            serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 1);

        let query_value = json!({
            "where": [
                ["age", "==", 36]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
            ]
        });
        let where_cbor =
            serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 1);
    }
}
