//! Raw conversion and application must agree on pending lifecycle containers.

use super::*;
use crate::config::DriveConfig;
use crate::drive::document::paths::contract_document_type_path_vec;
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::ConsumedDocument;
use crate::util::batch::{DocumentOperationType, DriveOperation, GroveDbOpBatch};
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
use dpp::data_contract::DataContractFactory;
use dpp::document::document_factory::DocumentFactory;
use dpp::platform_value::{platform_value, Value};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;

#[derive(Clone, Copy, Debug)]
enum DeleteKind {
    Delete,
    Force,
    ReadWithoutDocument,
}

struct DeleteFixture {
    drive: Drive,
    contract: DataContract,
    ids: [Identifier; 2],
    _directory: tempfile::TempDir,
}

impl DeleteFixture {
    fn new(history: bool, indexed: bool, verify: bool) -> Self {
        let version = latest();
        let directory = tempfile::TempDir::new().unwrap();
        let (drive, _) = Drive::open(
            directory.path(),
            Some(DriveConfig {
                batching_consistency_verification: verify,
                ..Default::default()
            }),
        )
        .unwrap();
        drive.create_initial_state_structure(None, version).unwrap();
        let indices = if indexed {
            vec![platform_value!({"name": "amount", "properties": [{"amount": "asc"}]})]
        } else {
            vec![]
        };
        let mut schema = platform_value!({
            "type": "object", "documentsKeepHistory": history,
            "canBeDeleted": true,
            "properties": {"amount": {"type": "integer", "minimum": 5, "maximum": 5, "position": 0}},
            "required": ["amount"], "additionalProperties": false,
            "indices": indices
        });
        if !indexed {
            schema
                .as_map_mut()
                .unwrap()
                .retain(|(key, _)| key != &Value::Text("indices".into()));
        }
        let contract = DataContractFactory::new(version.protocol_version)
            .unwrap()
            .create_with_value_config(
                [7; 32].into(),
                0,
                platform_value!({"person": schema}),
                None,
                None,
            )
            .unwrap()
            .data_contract_owned();
        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
            .unwrap();
        let ids = [[70; 32].into(), [71; 32].into()];
        for id in ids {
            let mut document = DocumentFactory::new(version.protocol_version)
                .unwrap()
                .create_document(
                    &contract,
                    [7; 32].into(),
                    "person".into(),
                    platform_value!({"amount": 5}),
                )
                .unwrap();
            document.set_id(id);
            document.set_revision(Some(1));
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type: document_type_of(&contract),
                    },
                    false,
                    BlockInfo::default_with_time(1000),
                    true,
                    None,
                    version,
                )
                .unwrap();
        }
        Self {
            drive,
            contract,
            ids,
            _directory: directory,
        }
    }

    fn operations(&self, kind: DeleteKind) -> Vec<DriveOperation<'_>> {
        self.ids
            .iter()
            .map(|id| {
                let contract_info = DataContractInfo::BorrowedDataContract(&self.contract);
                let document_type_info = DocumentTypeInfo::DocumentTypeNameAsStr("person");
                let operation = match kind {
                    DeleteKind::Delete => DocumentOperationType::DeleteDocument {
                        document_id: *id,
                        deleter_id: Some([7; 32].into()),
                        contract_info,
                        document_type_info,
                    },
                    DeleteKind::Force => DocumentOperationType::ForceDeleteDocument {
                        document_id: *id,
                        contract_info,
                        document_type_info,
                    },
                    DeleteKind::ReadWithoutDocument => {
                        DocumentOperationType::ForceDeleteReadDocument {
                            consumed_document: ConsumedDocument {
                                document_id: *id,
                                document_type_name: "person".into(),
                                read: None,
                            },
                            contract_info,
                        }
                    }
                };
                DriveOperation::DocumentOperation(operation)
            })
            .collect()
    }

    fn container_inserts(&self, batch: &GroveDbOpBatch) -> usize {
        let path = KeyInfoPath::from_known_owned_path(contract_document_type_path_vec(
            self.contract.id_ref().as_bytes(),
            "person",
        ));
        batch
            .operations
            .iter()
            .filter(|operation| {
                operation.path == path && operation.key == Some(KeyInfo::KnownKey(vec![1]))
            })
            .count()
    }

    fn assert_deleted(&self) {
        for id in self.ids {
            assert!(matches!(
                lifecycle_of(&self.drive, &self.contract, id),
                DocumentLifecycleState::Deleted(_)
            ));
            assert_eq!(
                record_of(&self.drive, &self.contract, id)
                    .unwrap()
                    .revision_count(),
                1
            );
        }
    }
}

#[test]
fn should_convert_sibling_history_deletes_without_duplicate_lifecycle_containers() {
    let version = latest();
    for verify in [false, true] {
        for kind in [
            DeleteKind::Delete,
            DeleteKind::Force,
            DeleteKind::ReadWithoutDocument,
        ] {
            let fixture = DeleteFixture::new(true, false, verify);
            let batch = fixture
                .drive
                .convert_drive_operations_to_grove_operations(
                    fixture.operations(kind),
                    &BlockInfo::default_with_time(2000),
                    None,
                    version,
                )
                .unwrap();
            assert_eq!(
                fixture.container_inserts(&batch),
                1,
                "{kind:?}: one lifecycle container regardless of batch verification"
            );
            fixture
                .drive
                .grove_apply_batch(batch, false, None, &version.drive)
                .unwrap();
            fixture.assert_deleted();
        }
    }
}

#[test]
fn should_apply_sibling_history_force_deletes_without_duplicate_lifecycle_containers() {
    let version = latest();
    for kind in [DeleteKind::Force, DeleteKind::ReadWithoutDocument] {
        let fixture = DeleteFixture::new(true, false, true);
        fixture
            .drive
            .apply_drive_operations(
                fixture.operations(kind),
                true,
                &BlockInfo::default_with_time(2000),
                None,
                version,
            )
            .unwrap();
        fixture.assert_deleted();
    }
}

#[test]
fn should_match_raw_conversion_with_application_for_shared_history_index() {
    let version = latest();
    for kind in [
        DeleteKind::Delete,
        DeleteKind::Force,
        DeleteKind::ReadWithoutDocument,
    ] {
        let converted = DeleteFixture::new(true, true, true);
        let applied = DeleteFixture::new(true, true, true);
        let block = BlockInfo::default_with_time(2000);
        let batch = converted
            .drive
            .convert_drive_operations_to_grove_operations(
                converted.operations(kind),
                &block,
                None,
                version,
            )
            .unwrap();
        converted
            .drive
            .grove_apply_batch(batch, false, None, &version.drive)
            .unwrap();
        applied
            .drive
            .apply_drive_operations(applied.operations(kind), true, &block, None, version)
            .unwrap();
        assert_eq!(
            converted
                .drive
                .grove
                .root_hash(None, &version.drive.grove_version)
                .value
                .unwrap(),
            applied
                .drive
                .grove
                .root_hash(None, &version.drive.grove_version)
                .value
                .unwrap(),
            "{kind:?}: the public converter and application settle the same tree",
        );
        converted.assert_deleted();
        applied.assert_deleted();
    }
}

#[test]
fn should_keep_non_history_raw_delete_operations_independent_of_pending_deletes() {
    let fixture = DeleteFixture::new(false, true, true);
    let version = latest();
    let mut released_converter = version.clone();
    released_converter
        .drive
        .methods
        .batch_operations
        .convert_drive_operations_to_grove_operations = 1;
    let block = BlockInfo::default_with_time(2000);
    let convert = |version, kind| {
        fixture
            .drive
            .convert_drive_operations_to_grove_operations(
                fixture.operations(kind),
                &block,
                None,
                version,
            )
            .unwrap()
    };
    for kind in [
        DeleteKind::Delete,
        DeleteKind::Force,
        DeleteKind::ReadWithoutDocument,
    ] {
        assert_eq!(
            convert(&released_converter, kind).operations,
            convert(version, kind).operations,
            "{kind:?}: pending siblings must not change non-history deletion"
        );
    }

    let mut unknown = version.clone();
    unknown
        .drive
        .methods
        .batch_operations
        .convert_drive_operations_to_grove_operations = 3;
    assert!(matches!(
        fixture.drive.convert_drive_operations_to_grove_operations(vec![], &block, None, &unknown),
        Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received: 3 }))
            if method == "convert_drive_operations_to_grove_operations" && known_versions == vec![0, 1, 2]
    ));
}
