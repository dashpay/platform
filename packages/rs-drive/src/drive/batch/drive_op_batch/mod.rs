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

pub(crate) mod identity;
pub(crate) mod document;
pub(crate) mod contract;

use crate::contract::document::Document;
use crate::contract::Contract;
use crate::drive::block_info::BlockInfo;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{
    DocumentRefAndSerialization, DocumentRefWithoutSerialization,
};
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;
use crate::fee::result::FeeResult;
use dpp::data_contract::extra::{DocumentType, DriveContractExt};
use dpp::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use grovedb::batch::{GroveDbOp, KeyInfoPath};
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
use itertools::Itertools;
use contract::ContractOperationType;
use document::DocumentOperationType;
use dpp::prelude::Revision;
use identity::IdentityOperationType;
use crate::drive::batch::GroveDbOpBatch;

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

/// All types of Drive Operations
pub enum DriveOperationType<'a> {
    /// A contract operation
    ContractOperation(ContractOperationType<'a>),
    /// A document operation
    DocumentOperation(DocumentOperationType<'a>),
    /// An identity operation
    IdentityOperation(IdentityOperationType),
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
            DriveOperationType::IdentityOperation(identity_operation_type) => {
                identity_operation_type.to_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                )
            }
        }
    }
}

impl Drive {
    pub fn convert_drive_operations_to_grove_operations(&self, drive_batch_operations: Vec<DriveOperationType>,         block_info: &BlockInfo,
                                                        transaction: TransactionArg)
        -> Result<GroveDbOpBatch, Error>{
        let ops = drive_batch_operations.into_iter().map(|drive_op| {
            let mut inner_drive_operations = drive_op.to_drive_operations(
                self,
                &mut None,
                block_info,
                transaction,
            )?;
            Ok(DriveOperation::grovedb_operations_consume(inner_drive_operations))
        }).flatten_ok().collect::<Result<Vec<GroveDbOp>,Error>>()?;
        Ok(GroveDbOpBatch::from_operations(ops))
    }
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
    use rand::Rng;
    use serde_json::json;
    use tempfile::TempDir;

    use crate::common::{json_document_to_cbor, setup_contract};
    use crate::drive::batch::drive_op_batch::document::DocumentOperation::{AddOperation, UpdateOperation};
    use crate::drive::batch::drive_op_batch::contract::ContractOperationType::ApplyContractWithSerialization;
    use crate::drive::batch::drive_op_batch::document::{DocumentOperationsForContractDocumentType, UpdateOperationInfo};
    use crate::drive::batch::drive_op_batch::document::DocumentOperationType::{
        AddSerializedDocumentForContract, MultipleDocumentOperationsForSameContractDocumentType,
    };
    use crate::drive::batch::DriveOperationType::{ContractOperation, DocumentOperation};
    use crate::drive::config::DriveConfig;
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
        );
        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("contract should be deserialized");
        let serialized_contract =
            DriveContractExt::to_cbor(&contract).expect("contract should be serialized");

        let _document_type = contract
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

        let dashpay_cr_serialized_document2 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(1),
        );

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

        let dashpay_cr_serialized_document0 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let dashpay_cr_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let mut operations = vec![];

        let document0 = Document::from_cbor(
            dashpay_cr_serialized_document0.as_slice(),
            None,
            Some(random_owner_id),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization {
                    0: (
                        &document0,
                        dashpay_cr_serialized_document0.as_slice(),
                        StorageFlags::optional_default_as_ref(),
                    ),
                },
                owner_id: Some(random_owner_id),
            },
            override_document: false,
        });

        let document1 = Document::from_cbor(
            dashpay_cr_serialized_document1.as_slice(),
            None,
            Some(random_owner_id),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization {
                    0: (
                        &document1,
                        dashpay_cr_serialized_document1.as_slice(),
                        StorageFlags::optional_default_as_ref(),
                    ),
                },
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
        );

        let person_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person3.json",
            Some(1),
        );

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let mut operations = vec![];

        let document0 = Document::from_cbor(
            person_serialized_document0.as_slice(),
            None,
            Some(random_owner_id0),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization {
                    0: (
                        &document0,
                        person_serialized_document0.as_slice(),
                        StorageFlags::optional_default_as_ref(),
                    ),
                },
                owner_id: Some(random_owner_id0),
            },
            override_document: false,
        });

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let document1 = Document::from_cbor(
            person_serialized_document1.as_slice(),
            None,
            Some(random_owner_id1),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization {
                    0: (
                        &document1,
                        person_serialized_document0.as_slice(),
                        StorageFlags::optional_default_as_ref(),
                    ),
                },
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
        );

        let person_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person3.json",
            Some(1),
        );

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let mut operations = vec![];

        let document0 = Document::from_cbor(
            person_serialized_document0.as_slice(),
            None,
            Some(random_owner_id0),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization {
                    0: (
                        &document0,
                        person_serialized_document0.as_slice(),
                        StorageFlags::optional_default_as_ref(),
                    ),
                },
                owner_id: Some(random_owner_id0),
            },
            override_document: false,
        });

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let document1 = Document::from_cbor(
            person_serialized_document1.as_slice(),
            None,
            Some(random_owner_id1),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization {
                    0: (
                        &document1,
                        person_serialized_document1.as_slice(),
                        StorageFlags::optional_default_as_ref(),
                    ),
                },
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
        );

        let person_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person3-older.json",
            Some(1),
        );

        let mut operations = vec![];

        let document0 = Document::from_cbor(
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

        let document1 = Document::from_cbor(
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
        );

        let person_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person3-older.json",
            Some(1),
        );

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let mut operations = vec![];

        let document0 = Document::from_cbor(
            person_serialized_document0.as_slice(),
            None,
            Some(random_owner_id0),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization {
                    0: (
                        &document0,
                        person_serialized_document0.as_slice(),
                        StorageFlags::optional_default_as_ref(),
                    ),
                },
                owner_id: Some(random_owner_id0),
            },
            override_document: false,
        });

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let document1 = Document::from_cbor(
            person_serialized_document1.as_slice(),
            None,
            Some(random_owner_id1),
        )
        .expect("expected to deserialize contact request");

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefAndSerialization {
                    0: (
                        &document1,
                        person_serialized_document1.as_slice(),
                        StorageFlags::optional_default_as_ref(),
                    ),
                },
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
        );

        let person_serialized_document1 = json_document_to_cbor(
            "tests/supporting_files/contract/family/person3.json",
            Some(1),
        );

        let mut operations = vec![];

        let document0 = Document::from_cbor(
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

        let document1 = Document::from_cbor(
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

        let query_value = json!({
            "where": [
                ["age", "==", 36]
            ],
            "limit": 100,
            "orderBy": [
                ["age", "asc"],
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
