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

use crate::contract::document::Document;
use crate::contract::Contract;
use crate::drive::block_info::BlockInfo;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentAndContractInfo;
use crate::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee::{calculate_fee, FeeResult};
use dpp::data_contract::extra::DriveContractExt;
use grovedb::TransactionArg;

/// A converter that will get Drive Operations from High Level Operations
pub trait DriveOperationConverter {
    /// This will get a list of atomic drive operations from a high level operations
    fn to_drive_operations(
        self,
        drive: &Drive,
        apply: bool,
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
        apply: bool,
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
                    apply,
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
                apply,
                storage_flags,
                transaction,
            ),
        }
    }
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
        document: &'a Document,
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
        apply: bool,
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

                let document = Document::from_cbor(serialized_document, None, owner_id)?;

                let document_info =
                    DocumentRefAndSerialization((&document, serialized_document, storage_flags));

                let document_type = contract.document_type_for_name(document_type_name)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    document_info,
                    contract: &contract,
                    document_type,
                    owner_id,
                };
                drive.add_document_for_contract_operations(
                    document_and_contract_info,
                    override_document,
                    block_info,
                    apply,
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
                    document_info,
                    contract,
                    document_type,
                    owner_id,
                };
                drive.add_document_for_contract_operations(
                    document_and_contract_info,
                    override_document,
                    block_info,
                    apply,
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
                apply,
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
                apply,
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
                    apply,
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

                let document = Document::from_cbor(serialized_document, None, owner_id)?;

                let document_info =
                    DocumentRefAndSerialization((&document, serialized_document, storage_flags));

                let document_type = contract.document_type_for_name(document_type_name)?;

                let document_and_contract_info = DocumentAndContractInfo {
                    document_info,
                    contract: &contract,
                    document_type,
                    owner_id,
                };
                drive.update_document_for_contract_operations(
                    document_and_contract_info,
                    block_info,
                    apply,
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
                    document_info,
                    contract,
                    document_type,
                    owner_id,
                };
                drive.update_document_for_contract_operations(
                    document_and_contract_info,
                    block_info,
                    apply,
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
                    document_info,
                    contract,
                    document_type,
                    owner_id,
                };
                drive.update_document_for_contract_operations(
                    document_and_contract_info,
                    block_info,
                    apply,
                    transaction,
                )
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

/// All types of Drive Operations
pub enum DriveOperationType<'a> {
    /// A contract operation
    ContractOperation(ContractOperationType<'a>),
    /// A document operation
    DocumentOperation(DocumentOperationType<'a>),
    // /// An identity operation
    // IdentityOperation(IdentityOperationType<'a>),
}

impl DriveOperationConverter for DriveOperationType<'_> {
    fn to_drive_operations(
        self,
        drive: &Drive,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        match self {
            DriveOperationType::ContractOperation(contract_operation_type) => {
                contract_operation_type.to_drive_operations(drive, apply, block_info, transaction)
            }
            DriveOperationType::DocumentOperation(document_operation_type) => {
                document_operation_type.to_drive_operations(drive, apply, block_info, transaction)
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
        for drive_op in operations {
            drive_operations.append(&mut drive_op.to_drive_operations(
                self,
                apply,
                block_info,
                transaction,
            )?);
        }
        let mut cost_operations = vec![];
        self.apply_batch_drive_operations(
            apply,
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
    use rand::Rng;
    use serde_json::json;
    use tempfile::TempDir;

    use crate::common::json_document_to_cbor;
    use crate::drive::batch::ContractOperationType::ApplyContractWithSerialization;
    use crate::drive::batch::DocumentOperationType::AddSerializedDocumentForContract;
    use crate::drive::batch::DriveOperationType::{ContractOperation, DocumentOperation};
    use crate::drive::contract::contract_root_path;
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
        );
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
        );

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
        let where_cbor = common::value_to_cbor(query_value, None);

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
