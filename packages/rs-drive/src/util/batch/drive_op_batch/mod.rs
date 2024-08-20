mod contract;
mod document;
mod drive_methods;
mod finalize_task;
mod identity;
mod prefunded_specialized_balance;
mod system;
mod withdrawals;

use crate::util::batch::GroveDbOpBatch;

use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;

pub use contract::DataContractOperationType;
pub use document::DocumentOperation;
pub use document::DocumentOperationType;
pub use document::DocumentOperationsForContractDocumentType;
pub use document::UpdateOperationInfo;
pub use identity::IdentityOperationType;
pub use prefunded_specialized_balance::PrefundedSpecializedBalanceOperationType;
pub use system::SystemOperationType;
pub use withdrawals::WithdrawalOperationType;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use crate::fees::op::LowLevelDriveOperation::GroveOperation;

use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};

use crate::error::drive::DriveError;
use crate::util::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};

use std::collections::{BTreeMap, HashMap};

/// A converter that will get Drive Operations from High Level Operations
pub trait DriveLowLevelOperationConverter {
    /// This will get a list of atomic drive operations from a high level operations
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error>;
}

/// The drive operation context keeps track of changes that might affect other operations
/// Notably Identity balance changes are kept track of
pub struct DriveOperationContext {
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    identity_balance_changes: BTreeMap<[u8; 32], i64>,
}

/// All types of Drive Operations
#[derive(Clone, Debug)]
pub enum DriveOperation<'a> {
    /// A contract operation
    DataContractOperation(DataContractOperationType<'a>),
    /// A document operation
    DocumentOperation(DocumentOperationType<'a>),
    /// Withdrawal operation
    WithdrawalOperation(WithdrawalOperationType),
    /// An identity operation
    IdentityOperation(IdentityOperationType),
    /// An operation on prefunded balances
    PrefundedSpecializedBalanceOperation(PrefundedSpecializedBalanceOperationType),
    /// A system operation
    SystemOperation(SystemOperationType),
    /// A single low level groveDB operation
    GroveDBOperation(QualifiedGroveDbOp),
    /// Multiple low level groveDB operations
    GroveDBOpBatch(GroveDbOpBatch),
}

impl DriveLowLevelOperationConverter for DriveOperation<'_> {
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
            DriveOperation::DataContractOperation(contract_operation_type) => {
                contract_operation_type.into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )
            }
            DriveOperation::DocumentOperation(document_operation_type) => document_operation_type
                .into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                ),
            DriveOperation::WithdrawalOperation(withdrawal_operation_type) => {
                withdrawal_operation_type.into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )
            }
            DriveOperation::IdentityOperation(identity_operation_type) => identity_operation_type
                .into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                ),
            DriveOperation::PrefundedSpecializedBalanceOperation(
                prefunded_balance_operation_type,
            ) => prefunded_balance_operation_type.into_low_level_drive_operations(
                drive,
                estimated_costs_only_with_layer_info,
                block_info,
                transaction,
                platform_version,
            ),
            DriveOperation::SystemOperation(system_operation_type) => system_operation_type
                .into_low_level_drive_operations(
                    drive,
                    estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                ),
            DriveOperation::GroveDBOperation(op) => Ok(vec![GroveOperation(op)]),
            DriveOperation::GroveDBOpBatch(operations) => Ok(operations
                .operations
                .into_iter()
                .map(GroveOperation)
                .collect()),
        }
    }
}

impl DriveOperationFinalizationTasks for DriveOperation<'_> {
    fn finalization_tasks(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .operations
            .finalization_tasks
        {
            0 => self.finalization_tasks_v0(platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveOperation.finalization_tasks".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl DriveOperation<'_> {
    fn finalization_tasks_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error> {
        match self {
            DriveOperation::DataContractOperation(o) => o.finalization_tasks(platform_version),
            _ => Ok(None),
        }
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use grovedb::Element;
    use std::borrow::Cow;
    use std::option::Option::None;

    use super::*;

    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::serialization::PlatformSerializableWithPlatformVersion;
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
    use dpp::util::cbor_serializer;
    use rand::Rng;
    use serde_json::json;

    use crate::util::test_helpers::setup_contract;

    use crate::util::batch::drive_op_batch::document::DocumentOperation::{
        AddOperation, UpdateOperation,
    };
    use crate::util::batch::drive_op_batch::document::DocumentOperationType::MultipleDocumentOperationsForSameContractDocumentType;
    use crate::util::batch::drive_op_batch::document::{
        DocumentOperationsForContractDocumentType, UpdateOperationInfo,
    };
    use crate::util::batch::DataContractOperationType::ApplyContract;
    use crate::util::batch::DocumentOperationType::AddDocument;
    use crate::util::batch::DriveOperation::{DataContractOperation, DocumentOperation};

    use crate::drive::contract::paths::contract_root_path;
    use crate::drive::Drive;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    #[test]
    fn test_add_dashpay_documents() {
        let drive: Drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        let contract = json_document_to_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let _document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        drive_operations.push(DataContractOperation(ApplyContract {
            contract: Cow::Borrowed(&contract),
            storage_flags: None,
        }));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        let dashpay_cr_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        drive_operations.push(DocumentOperation(AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &dashpay_cr_document,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: None,
            },
            contract_info: DataContractInfo::BorrowedDataContract(&contract),
            document_type_info: DocumentTypeInfo::DocumentTypeRef(document_type),
            override_document: false,
        }));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to insert contract and document");

        let element = drive
            .grove
            .get(
                &contract_root_path(&contract.id().to_buffer()),
                &[0],
                Some(&db_transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to get contract back");

        assert_eq!(
            element,
            Element::Item(
                contract
                    .serialize_to_bytes_with_platform_version(platform_version)
                    .expect("expected to serialize contract"),
                None
            )
        );

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["$ownerId", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn test_add_multiple_dashpay_documents_individually_should_succeed() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        let contract = json_document_to_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        drive_operations.push(DataContractOperation(ApplyContract {
            contract: Cow::Borrowed(&contract),
            storage_flags: None,
        }));
        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let dashpay_cr_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get contract");

        drive_operations.push(DocumentOperation(AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((&dashpay_cr_document, None)),
                owner_id: None,
            },
            contract_info: DataContractInfo::BorrowedDataContract(&contract),
            document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("contactRequest"),
            override_document: false,
        }));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let dashpay_cr_1_document = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get contract");

        drive_operations.push(DocumentOperation(AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((&dashpay_cr_1_document, None)),
                owner_id: None,
            },
            contract_info: DataContractInfo::BorrowedDataContract(&contract),
            document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("contactRequest"),
            override_document: false,
        }));

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
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
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_add_multiple_dashpay_documents() {
        let drive: Drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        let contract = json_document_to_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            false,
            platform_version,
        )
        .expect("expected to get contract");

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type");

        drive_operations.push(DataContractOperation(ApplyContract {
            contract: Cow::Borrowed(&contract),
            storage_flags: None,
        }));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document0 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document 0");

        let document1 = json_document_to_document(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(random_owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document 1");

        let mut operations = vec![];

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document0,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: Some(random_owner_id),
            },
            override_document: false,
        });

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &document1,
                    StorageFlags::optional_default_as_cow(),
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
                platform_version,
                None,
            )
            .expect("expected to be able to insert documents");

        let element = drive
            .grove
            .get(
                &contract_root_path(&contract.id().to_buffer()),
                &[0],
                Some(&db_transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to get contract back");

        assert_eq!(
            element,
            Element::Item(
                contract
                    .serialize_to_bytes_with_platform_version(platform_version)
                    .expect("expected to serialize contract"),
                None
            )
        );

        let query_value = json!({
            "where": [
            ],
            "limit": 100,
            "orderBy": [
                ["$ownerId", "asc"],
            ]
        });
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_add_multiple_family_documents() {
        let drive: Drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person3.json",
            Some(random_owner_id1.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let mut operations = vec![];

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &person_document0,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: Some(random_owner_id0),
            },
            override_document: false,
        });

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &person_document1,
                    StorageFlags::optional_default_as_cow(),
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
                platform_version,
                None,
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
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_update_multiple_family_documents() {
        let drive: Drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let mut drive_operations = vec![];
        let db_transaction = drive.grove.start_transaction();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-only-age-index.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person3.json",
            Some(random_owner_id1.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let mut operations = vec![];

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &person_document0,
                    StorageFlags::optional_default_as_cow(),
                )),
                owner_id: Some(random_owner_id0),
            },
            override_document: false,
        });

        operations.push(AddOperation {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((
                    &person_document1,
                    StorageFlags::optional_default_as_cow(),
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
                platform_version,
                None,
            )
            .expect("expected to be able to insert documents");

        // This was the setup now let's do the update

        drive_operations = vec![];

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0-older.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person3-older.json",
            Some(random_owner_id1.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let mut operations = vec![];

        operations.push(UpdateOperation(UpdateOperationInfo {
            document: &person_document0,
            serialized_document: None,
            owner_id: Some(random_owner_id0),
            storage_flags: None,
        }));

        operations.push(UpdateOperation(UpdateOperationInfo {
            document: &person_document1,
            serialized_document: None,
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
                platform_version,
                None,
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
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
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
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
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
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn test_update_multiple_family_documents_with_index_being_removed_and_added() {
        let drive: Drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let db_transaction = drive.grove.start_transaction();

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-only-age-index.json",
            None,
            Some(&db_transaction),
        );

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get document type");

        let random_owner_id0 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let random_owner_id1 = rand::thread_rng().gen::<[u8; 32]>();

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person3-older.json",
            Some(random_owner_id1.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let operations = vec![
            AddOperation {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &person_document0,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(random_owner_id0),
                },
                override_document: false,
            },
            AddOperation {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &person_document1,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(random_owner_id1),
                },
                override_document: false,
            },
        ];
        let drive_operations = vec![DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        )];

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
            )
            .expect("expected to be able to insert documents");

        // This was the setup now let's do the update

        let person_document0 = json_document_to_document(
            "tests/supporting_files/contract/family/person0-older.json",
            Some(random_owner_id0.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let person_document1 = json_document_to_document(
            "tests/supporting_files/contract/family/person3.json",
            Some(random_owner_id1.into()),
            document_type,
            platform_version,
        )
        .expect("expected to get document");

        let operations = vec![
            UpdateOperation(UpdateOperationInfo {
                document: &person_document0,
                serialized_document: None,
                owner_id: Some(random_owner_id0),
                storage_flags: None,
            }),
            UpdateOperation(UpdateOperationInfo {
                document: &person_document1,
                serialized_document: None,
                owner_id: Some(random_owner_id1),
                storage_flags: None,
            }),
        ];

        let drive_operations = vec![DocumentOperation(
            MultipleDocumentOperationsForSameContractDocumentType {
                document_operations: DocumentOperationsForContractDocumentType {
                    operations,
                    contract: &contract,
                    document_type,
                },
            },
        )];

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&db_transaction),
                platform_version,
                None,
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
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
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
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
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
        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        let (docs, _, _) = drive
            .query_documents_cbor_from_contract(
                &contract,
                document_type,
                where_cbor.as_slice(),
                None,
                Some(&db_transaction),
                Some(platform_version.protocol_version),
            )
            .expect("expected to query");
        assert_eq!(docs.len(), 1);
    }
}
