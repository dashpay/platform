use super::*;
use crate::config::DriveConfig;
use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryFilter,
};
use crate::query::{resolve_integer_range_bucket_clause, DriveDocumentQuery};
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContractFactory;
use dpp::document::document_factory::DocumentFactory;
use dpp::document::DocumentV0Setters;
use dpp::platform_value::{platform_value, Value};
use std::borrow::Cow;

#[test]
fn should_read_and_prove_fresh_and_migrated_history_through_every_index_kind() {
    for countable in [false, true] {
        read_and_prove_index_matrix(countable);
    }
}

fn read_and_prove_index_matrix(countable: bool) {
    let new = PlatformVersion::get(15).unwrap();
    for migrated in [false, true] {
        let old = PlatformVersion::get(if migrated { 14 } else { 15 }).unwrap();
        let directory = tempfile::TempDir::new().unwrap();
        let (drive, _) = Drive::open(directory.path(), None).unwrap();
        drive.create_initial_state_structure(None, old).unwrap();
        let schemas = platform_value!({
            "tip": {
                "type":"object", "documentsKeepHistory":true, "documentsMutable":true, "canBeDeleted":false,
                "documentsCountable":countable, "documentsSummable":"amount",
                "immutable":["postId"],
                "properties": {
                    "slug":{"type":"string", "maxLength":10, "position":0},
                    "group":{"type":"string", "maxLength":10, "position":1},
                    "recipient":{"type":"string", "maxLength":10, "position":2},
                    "amount":{"type":"integer", "minimum":0, "maximum":4294967295i64, "position":3},
                    "tags":{"type":"array", "maxItems":4, "items":{"type":"string", "minLength":1, "maxLength":10}, "position":4},
                    "optional":{"type":"integer", "minimum":0, "maximum":9, "position":5},
                    "postId":{"type":"array", "byteArray":true, "minItems":32, "maxItems":32,
                        "contentMediaType":"application/x.dash.dpp.identifier",
                        "refersTo":{"type":"permanentDocument", "documentType":"source"}, "position":6}
                },
                "required":["slug","group","recipient","amount","tags","postId","$createdAt"], "additionalProperties":false,
                "indices":[
                    {"name":"slug","properties":[{"slug":"asc"}],"unique":true},
                    {"name":"groupAggregate","properties":[{"group":"asc"}],"countable":"countable","summable":"amount"},
                    {"name":"groupAmount","properties":[{"group":"asc"},{"amount":"asc"}]},
                    {"name":"recipientAmount","properties":[{"recipient":"asc"},{"amount":"asc"}],"countable":"countable","rangeCountable":true,"rankedCountable":{"at":"recipient"}},
                    {"name":"recipientSlugAmount","properties":[{"recipient":"asc"},{"slug":"asc"},{"amount":"asc"}]},
                    {"name":"trending","properties":[{"$createdAt":"asc"},{"group":"asc"},{"slug":"asc"}],"countable":"countable","timeRange":{"on":"$createdAt","range":3600,"step":900}},
                    {"name":"byTag","properties":[{"tags":"asc"}],"countable":"countable","rangeCountable":true,"rankedCountable":true},
                    {"name":"byBand","properties":[{"amount":"asc"},{"slug":"asc"}],"countable":"countable","integerRange":{"on":"amount","range":30,"step":10}},
                    {"name":"sparse","properties":[{"group":"asc"},{"optional":"asc"}],"skipIfAbsent":["optional"]},
                    {"name":"byTopic","properties":[{"postId.topic":"asc"}]}
                ]
            },
            "winner": {
                "type":"object", "documentsMutable":false, "canBeDeleted":false, "documentsKeepHistory":true,
                "properties":{"code":{"type":"string", "maxLength":10, "position":0}},
                "required":["code"], "additionalProperties":false,
                "indices":[{"name":"byCode", "properties":[{"code":"asc"}], "unique":true, "contested":{"resolution":1}}]
            },
            "source": {
                "type":"object", "documentsMutable":false, "canBeDeleted":false,
                "properties":{"topic":{"type":"string", "maxLength":10, "position":0}},
                "required":["topic"], "additionalProperties":false
            }
        });
        let contract = DataContractFactory::new(old.protocol_version)
            .unwrap()
            .create_with_value_config([7; 32].into(), 0, schemas, None, None)
            .unwrap()
            .data_contract_owned();
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
        let insert = |document: &Document, name: &str| {
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentRefInfo((
                                document,
                                Some(Cow::Borrowed(&writer_flags)),
                            )),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type: contract.document_type_for_name(name).unwrap(),
                    },
                    false,
                    BlockInfo::default_with_time(1000),
                    true,
                    None,
                    old,
                    None,
                )
                .unwrap();
        };
        let mut source = DocumentFactory::new(old.protocol_version)
            .unwrap()
            .create_document(
                &contract,
                [7; 32].into(),
                "source".into(),
                platform_value!({"topic":"history"}),
            )
            .unwrap();
        source.set_id([4; 32].into());
        insert(&source, "source");
        let mut winner = DocumentFactory::new(old.protocol_version)
            .unwrap()
            .create_document(
                &contract,
                [7; 32].into(),
                "winner".into(),
                platform_value!({"code":"winner"}),
            )
            .unwrap();
        winner.set_id([5; 32].into());
        insert(&winner, "winner");
        let mut documents = vec![];
        for (id, slug, amount) in [(8u8, "a", 10u32), (9, "b", 20)] {
            let mut document = DocumentFactory::new(old.protocol_version)
                .unwrap()
                .create_document(
                    &contract,
                    [7; 32].into(),
                    "tip".into(),
                    platform_value!({"slug":slug,"group":"all","recipient":"bob","amount":amount,
                        "tags":["shared",slug],"postId":Value::Identifier([4;32])}),
                )
                .unwrap();
            document.set_id([id; 32].into());
            document.set_revision(Some(1));
            document.set_created_at(Some(7_200_000));
            if id == 8 {
                document.set("optional", 1u8.into());
            }
            insert(&document, "tip");
            documents.push(document);
        }
        let original = documents[0].clone();
        documents[0].set_revision(Some(2));
        documents[0].set("amount", 40u32.into());
        documents[0].set("tags", platform_value!(["shared", "new"]));
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
        for name in document_type.index_structure().sub_levels().keys() {
            let mut path = type_path.clone();
            path.push(name.as_bytes().to_vec());
            drive
                .history_migration_index_entries(path, &transaction, &mut stats, &mut before, old)
                .unwrap();
        }
        // Per document: five plain references, four time windows, two array
        // elements, three integer buckets and one derived topic; only the
        // first document contributes to the sparse optional index.
        let expected_index_entries = 2 * (5 + 4 + 2 + 3 + 1) + 1;
        assert_eq!(
            before.values().map(Vec::len).sum::<usize>() as u64,
            expected_index_entries
        );
        assert_eq!(stats.index_entries, expected_index_entries);
        if migrated {
            let stats = drive
                .migrate_document_history_storage(&transaction, new)
                .unwrap();
            assert_eq!(stats.documents, 3);
            assert_eq!(stats.index_entries, expected_index_entries + 1);
            assert_eq!(stats.rewritten_index_entries, expected_index_entries + 1);
        }
        drive.grove.commit_transaction(transaction).value.unwrap();
        drop(drive);
        let (drive, _) = Drive::open(directory.path(), None).unwrap();
        for (name, document) in documents
            .iter()
            .map(|document| ("tip", document))
            .chain(std::iter::once(("winner", &winner)))
        {
            let document_type = contract.document_type_for_name(name).unwrap();
            let query = DocumentHistoryDriveQuery {
                contract_id: contract.id().to_buffer(),
                document_type_name: name.into(),
                document_id: document.id().to_buffer(),
                filter: DocumentHistoryFilter::StartAtTime(0),
                limit: None,
            };
            let page = drive
                .fetch_document_history(&query, document_type, None, new)
                .unwrap();
            let expected = if document.id() == original.id() {
                vec![original.clone(), document.clone()]
            } else {
                vec![document.clone()]
            };
            assert_eq!(
                page.entries
                    .iter()
                    .map(|entry| entry.document.clone())
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(
                page.lifecycle.as_ref().unwrap().remaining_revisions,
                expected.len() as u64
            );
            let proof = drive
                .prove_document_history(&query, document_type, None, new)
                .unwrap();
            assert_eq!(
                Drive::verify_document_history(&query, &proof, document_type, new)
                    .unwrap()
                    .1,
                page
            );
        }
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
                "select * from tip where group = 'all' and amount > 30 order by amount asc",
                vec![documents[0].clone()],
            ),
            (
                "select * from tip where recipient = 'bob'",
                documents.clone(),
            ),
            (
                "select * from tip where recipient = 'bob' and amount < 30 order by amount asc",
                vec![documents[1].clone()],
            ),
            (
                "select * from tip where recipient = 'bob' and slug = 'a'",
                vec![documents[0].clone()],
            ),
            (
                "select * from tip where group = 'all' and optional = 1",
                vec![documents[0].clone()],
            ),
            (
                "select * from tip where group = 'all' and optional = 2",
                vec![],
            ),
        ] {
            let query = DriveDocumentQuery::from_sql_expr(sql, &contract, None, new).unwrap();
            let mut fetched = drive
                .query_documents(query.clone(), None, false, None, Some(15))
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
        let mut new_queries = vec![(
            DriveDocumentQuery::from_sql_expr(
                "select * from winner where code = 'winner'",
                &contract,
                None,
                new,
            )
            .unwrap(),
            vec![winner.clone()],
        )];
        new_queries.push((
            DriveDocumentQuery::from_value(
                platform_value!({"where":[["postId.topic","==","history"]]}),
                &contract,
                document_type,
                &DriveConfig::default(),
                new,
            )
            .unwrap(),
            documents.clone(),
        ));
        for (tag, expected) in [
            ("shared", documents.clone()),
            ("new", vec![documents[0].clone()]),
            ("a", vec![]),
        ] {
            new_queries.push((
                DriveDocumentQuery::from_value(
                    platform_value!({"where":[["tags","contains",tag]]}),
                    &contract,
                    document_type,
                    &DriveConfig::default(),
                    new,
                )
                .unwrap(),
                expected,
            ));
        }
        for (start, expected) in [
            (30u64, vec![documents[0].clone()]),
            (0, vec![documents[1].clone()]),
        ] {
            let (clause, resolved) = resolve_integer_range_bucket_clause(
                "amount",
                &Value::U64(start),
                None,
                document_type,
            )
            .unwrap();
            let mut query = DriveDocumentQuery::from_value(
                Value::Map(vec![]),
                &contract,
                document_type,
                &DriveConfig::default(),
                new,
            )
            .unwrap();
            query
                .internal_clauses
                .equal_clauses
                .insert(clause.field.clone(), clause);
            query.resolved_time_ranges = vec![resolved];
            new_queries.push((query, expected));
        }
        for (query, expected) in new_queries {
            let mut fetched = drive
                .query_documents(query.clone(), None, false, None, Some(15))
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
            assert_eq!(fetched, expected, "migrated={migrated}");
            assert_eq!(verified, expected, "migrated={migrated}");
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
        // Aggregates survive migration: the count-sum value tree of the
        // shared-prefix index keeps both documents and their amounts, and the
        // count tree of the ranked index keeps its document count.
        let value_tree = |index: &[u8]| {
            let mut path = type_path.clone();
            path.push(index.to_vec());
            let values = drive
                .history_migration_entries(&path, &drive.grove.start_transaction(), new)
                .unwrap();
            assert_eq!(values.len(), 1, "{values:?}");
            values.into_iter().next().unwrap().1
        };
        let group = value_tree(b"group");
        assert!(
            matches!(group, Element::CountSumTree(_, 2, 60, _)),
            "{group:?}"
        );
        let recipient = value_tree(b"recipient");
        assert!(
            matches!(recipient, Element::CountTree(_, 2, _)),
            "{recipient:?}"
        );
        // The time-range index keeps its four windows, each still a tree.
        let mut path = type_path.clone();
        path.push(b"$createdAt#3600#900".to_vec());
        let windows = drive
            .history_migration_entries(&path, &drive.grove.start_transaction(), new)
            .unwrap();
        assert_eq!(windows.len(), 4, "{windows:?}");
        assert!(windows.iter().all(|(_, element)| element.is_any_tree()));
    }
}
