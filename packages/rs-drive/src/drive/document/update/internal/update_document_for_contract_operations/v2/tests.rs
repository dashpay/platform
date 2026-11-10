use crate::config::DriveConfig;
use crate::drive::document::paths::{contract_documents_primary_key_path, document_history_path};
use crate::drive::Drive;
use crate::util::common::encode::encode_u64;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::DataContractFactory;
use dpp::document::document_factory::DocumentFactory;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::platform_value;
use dpp::version::PlatformVersion;
use grovedb::Element;
use std::borrow::Cow;

#[test]
fn should_hand_over_history_indexes_without_changing_retained_revision_owners() {
    assert_history_handover(false);
}

#[test]
fn should_use_the_current_revision_owner_after_migrating_a_transferred_history() {
    assert_history_handover(true);
}

fn assert_history_handover(migrated: bool) {
    let current = PlatformVersion::latest();
    let initial = if migrated {
        PlatformVersion::get(14).unwrap()
    } else {
        current
    };
    let directory = tempfile::TempDir::new().unwrap();
    let (drive, _) = Drive::open(
        directory.path(),
        Some(DriveConfig {
            batching_consistency_verification: true,
            ..Default::default()
        }),
    )
    .unwrap();
    drive.create_initial_state_structure(None, initial).unwrap();
    let contract = DataContractFactory::new(initial.protocol_version).unwrap()
        .create_with_value_config([7; 32].into(), 0, platform_value!({"note": {
            "type": "object", "documentsKeepHistory": true, "canBeDeleted": false,
            "properties": {"amount": {"type": "integer", "minimum": 0, "maximum": 100, "position": 0}},
            "required": ["amount"], "additionalProperties": false,
            "indices": [{"name": "amount", "properties": [{"amount": "asc"}]}]
        }}), Some(platform_value!({"canBeDeleted": true})), None).unwrap().data_contract_owned();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, initial)
        .unwrap();
    let document_type = contract.document_type_for_name("note").unwrap();
    let mut document = DocumentFactory::new(initial.protocol_version)
        .unwrap()
        .create_document(
            &contract,
            [7; 32].into(),
            "note".into(),
            platform_value!({"amount": 1}),
        )
        .unwrap();
    let first_owner = [7; 32];
    let next_owner = [8; 32];
    let flags = StorageFlags::new_single_epoch(0, Some(first_owner));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((
                        &document,
                        Some(Cow::Borrowed(&flags)),
                    )),
                    owner_id: Some(first_owner),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default_with_time(1000),
            true,
            None,
            initial,
        )
        .unwrap();
    if migrated {
        document.set_owner_id(next_owner.into());
        document.set_revision(Some(2));
        drive
            .update_document_for_contract(
                &document,
                &contract,
                document_type,
                Some(next_owner),
                BlockInfo::default_with_time(2000),
                true,
                Some(Cow::Owned(StorageFlags::new_single_epoch(
                    0,
                    Some(next_owner),
                ))),
                None,
                initial,
            )
            .unwrap();
        let transaction = drive.grove.start_transaction();
        drive
            .migrate_document_history_storage(&transaction, current)
            .unwrap();
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .unwrap();
    }
    let contract_id = contract.id();
    let primary = contract_documents_primary_key_path(contract_id.as_slice(), "note");
    let raw = |path: &[&[u8]], key: &[u8]| {
        drive
            .grove
            .get_raw(path.into(), key, None, &current.drive.grove_version)
            .unwrap()
            .unwrap()
    };
    let owner = |element: &Element| {
        StorageFlags::map_some_element_flags_ref(element.get_flags())
            .unwrap()
            .and_then(|flags| flags.owner_id().copied())
    };
    assert_eq!(
        owner(&raw(&primary, document.id().as_slice())),
        Some(first_owner)
    );
    document.set_owner_id(next_owner.into());
    document.set_revision(Some(if migrated { 3 } else { 2 }));
    drive
        .update_document_for_contract(
            &document,
            &contract,
            document_type,
            Some(next_owner),
            BlockInfo::default_with_time(3000),
            true,
            Some(Cow::Owned(StorageFlags::new_single_epoch(
                0,
                Some(next_owner),
            ))),
            None,
            current,
        )
        .unwrap();
    assert_eq!(
        owner(&raw(&primary, document.id().as_slice())),
        Some(next_owner)
    );
    let mut index_path = primary[..4].to_vec();
    let amount = document_type
        .serialize_value_for_key("amount", &1.into(), current)
        .unwrap();
    index_path.extend([b"amount".as_slice(), amount.as_slice(), &[0]]);
    let index_reference = raw(&index_path, document.id().as_slice());
    assert!(index_reference.is_reference());
    assert_eq!(
        owner(&index_reference),
        Some(if migrated { first_owner } else { next_owner })
    );
    let resolved = drive
        .grove
        .get(
            index_path.as_slice(),
            document.id().as_slice(),
            None,
            &current.drive.grove_version,
        )
        .unwrap()
        .unwrap();
    assert_eq!(owner(&resolved), Some(next_owner));
    let history = document_history_path(contract.id().as_slice(), "note", document.id().as_slice());
    let history_refs = history.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let key = |time: u64, revision: u64| [encode_u64(time), encode_u64(revision)].concat();
    assert_eq!(owner(&raw(&history_refs, &key(1000, 1))), Some(first_owner));
    if migrated {
        assert_eq!(owner(&raw(&history_refs, &key(2000, 2))), Some(next_owner));
    }
    assert_eq!(
        owner(&raw(
            &history_refs,
            &key(3000, if migrated { 3 } else { 2 })
        )),
        Some(next_owner)
    );
}
