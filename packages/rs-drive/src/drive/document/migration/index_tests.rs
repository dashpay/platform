use super::*;
use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::query::DriveDocumentQuery;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContractFactory;
use dpp::document::document_factory::DocumentFactory;
use dpp::document::DocumentV0Setters;
use dpp::platform_value::platform_value;
use std::borrow::Cow;

#[test]
fn should_read_and_prove_fresh_and_migrated_history_through_every_index_kind() {
    for countable in [false, true] {
        read_and_prove_index_matrix(countable);
    }
}

fn read_and_prove_index_matrix(countable: bool) {
    let new = PlatformVersion::get(14).unwrap();
    for migrated in [false, true] {
        let old = PlatformVersion::get(if migrated { 13 } else { 14 }).unwrap();
        let directory = tempfile::TempDir::new().unwrap();
        let (drive, _) = Drive::open(directory.path(), None).unwrap();
        drive.create_initial_state_structure(None, old).unwrap();
        let contract = DataContractFactory::new(old.protocol_version).unwrap().create_with_value_config([7;32].into(), 0, platform_value!({
            "tip": {
                "type":"object", "documentsKeepHistory":true, "documentsMutable":true, "canBeDeleted":false,
                "documentsCountable":countable, "documentsSummable":"amount",
                "properties": {
                    "slug":{"type":"string", "maxLength":10, "position":0},
                    "group":{"type":"string", "maxLength":10, "position":1},
                    "recipient":{"type":"string", "maxLength":10, "position":2},
                    "amount":{"type":"integer", "minimum":0, "maximum":4294967295i64, "position":3}
                },
                "required":["slug","group","recipient","amount"], "additionalProperties":false,
                "indices":[
                    {"name":"slug","properties":[{"slug":"asc"}],"unique":true},
                    {"name":"group","properties":[{"group":"asc"}]},
                    {"name":"recipient","properties":[{"recipient":"asc"}],"summable":"amount"}
                ]
            }
        }), None, None).unwrap().data_contract_owned();
        let contract_flags = StorageFlags::new_single_epoch(2, Some([55; 32]));
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                Some(Cow::Borrowed(&contract_flags)),
                None,
                old,
            )
            .unwrap();
        let document_type = contract.document_type_for_name("tip").unwrap();
        let writer_flags = StorageFlags::new_single_epoch(3, Some([99; 32]));
        let mut documents = vec![];
        for (id, slug, amount) in [(8u8, "a", 10u32), (9, "b", 20)] {
            let mut document = DocumentFactory::new(old.protocol_version)
                .unwrap()
                .create_document(
                    &contract,
                    [7; 32].into(),
                    "tip".into(),
                    platform_value!({"slug":slug,"group":"all","recipient":"bob","amount":amount}),
                )
                .unwrap();
            document.set_id([id; 32].into());
            document.set_revision(Some(1));
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentRefInfo((
                                &document,
                                Some(Cow::Borrowed(&writer_flags)),
                            )),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default_with_time(1000),
                    true,
                    None,
                    old,
                    None,
                )
                .unwrap();
            documents.push(document);
        }
        documents[0].set_revision(Some(2));
        documents[0].set("amount", 40u32.into());
        drive
            .update_document_for_contract(
                &documents[0],
                &contract,
                document_type,
                Some([7; 32]),
                BlockInfo::default_with_time(2000),
                true,
                Some(Cow::Borrowed(&writer_flags)),
                None,
                old,
                None,
            )
            .unwrap();
        let type_path = contract_document_type_path_vec(contract.id().as_slice(), "tip");
        let transaction = drive.grove.start_transaction();
        let mut stats = DocumentHistoryMigrationStats::default();
        let mut before = BTreeMap::new();
        for name in ["slug", "group", "recipient"] {
            let mut path = type_path.clone();
            path.push(name.as_bytes().to_vec());
            drive
                .history_migration_index_entries(path, &transaction, old, &mut stats, &mut before)
                .unwrap();
        }
        assert_eq!(stats.index_entries, 6);
        if migrated {
            let stats = drive
                .migrate_document_history_storage(&transaction, new)
                .unwrap();
            assert_eq!(stats.documents, 2);
            assert_eq!(stats.index_entries, 6);
            assert_eq!(stats.rewritten_index_entries, 6);
        }
        drive.grove.commit_transaction(transaction).value.unwrap();
        drop(drive);
        let (drive, _) = Drive::open(directory.path(), None).unwrap();
        for (document_id, references) in &before {
            for (path, key, original) in references {
                let current = drive
                    .grove
                    .get_raw(path.as_slice().into(), key, None, &new.drive.grove_version)
                    .value
                    .unwrap();
                assert_eq!(current.get_flags(), original.get_flags());
                match (&current, original) {
                    (
                        Element::Reference(UpstreamRootHeightReference(4, target), Some(2), _),
                        Element::Reference(..),
                    ) => assert_eq!(target, &vec![vec![0], document_id.clone()]),
                    (
                        Element::ReferenceWithSumItem(
                            UpstreamRootHeightReference(4, target),
                            Some(2),
                            sum,
                            _,
                        ),
                        Element::ReferenceWithSumItem(_, _, previous_sum, _),
                    ) => {
                        assert_eq!(target, &vec![vec![0], document_id.clone()]);
                        assert_eq!(sum, previous_sum);
                    }
                    _ => panic!("unexpected index reference: {current:?}"),
                }
            }
        }
        for (sql, expected) in [
            (
                "select * from tip where slug = 'a'",
                vec![documents[0].clone()],
            ),
            ("select * from tip where group = 'all'", documents.clone()),
            (
                "select * from tip where recipient = 'bob'",
                documents.clone(),
            ),
        ] {
            let query = DriveDocumentQuery::from_sql_expr(sql, &contract, None, new).unwrap();
            let mut fetched = drive
                .query_documents(query.clone(), None, false, None, Some(14))
                .unwrap()
                .documents()
                .to_vec();
            let (proof, _) = query
                .clone()
                .execute_with_proof(&drive, None, None, new)
                .unwrap();
            let (_, mut verified) = query.verify_proof(&proof, new).unwrap();
            fetched.sort_by_key(|doc| doc.id());
            verified.sort_by_key(|doc| doc.id());
            assert_eq!(fetched, expected, "{sql}, migrated={migrated}");
            assert_eq!(verified, expected, "{sql}, migrated={migrated}");
        }
        let primary = drive
            .grove
            .get_raw(
                type_path.as_slice().into(),
                &[0],
                None,
                &new.drive.grove_version,
            )
            .value
            .unwrap();
        if countable {
            assert!(
                matches!(primary, Element::CountSumTree(_, 2, 60, _)),
                "{primary:?}"
            );
        } else {
            assert!(matches!(primary, Element::SumTree(_, 60, _)), "{primary:?}");
        }
        let mut path = type_path.clone();
        path.push(b"recipient".to_vec());
        let mut ignored = DocumentHistoryMigrationStats::default();
        let values = drive
            .history_migration_entries(&path, &drive.grove.start_transaction(), new, &mut ignored)
            .unwrap();
        assert_eq!(values.len(), 1);
        assert!(
            matches!(values[0].1, Element::SumTree(_, 60, _)),
            "{:?}",
            values[0]
        );
    }
}
