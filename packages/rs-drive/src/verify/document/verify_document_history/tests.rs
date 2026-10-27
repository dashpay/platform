use crate::drive::document::paths::{
    contract_document_type_path_vec, document_history_path, DOCUMENT_HISTORY_TREE_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryDriveQueryExecutionResult, DocumentHistoryFilter,
    DocumentHistoryLifecycle, DocumentHistoryState,
};
use crate::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use crate::util::common::encode::encode_u64;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use crate::verify::RootHash;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::DocumentV0Getters;
use dpp::document::DocumentV0Setters;
use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
use dpp::version::PlatformVersion;
use grovedb::{Element, PathQuery, Query, SizedQuery};

/// Fetches and proves a page at a protocol version that stores the history
/// tree, returning the ordinary GroveDB proof for tampering.
fn prove(
    drive: &Drive,
    query: &DocumentHistoryDriveQuery,
    document_type: DocumentTypeRef,
    version: &PlatformVersion,
) -> (DocumentHistoryDriveQueryExecutionResult, Vec<u8>) {
    let page = drive
        .fetch_document_history(query, document_type, None, version)
        .unwrap();
    let proof = drive
        .prove_document_history(query, document_type, None, version)
        .unwrap();
    (page, proof)
}

fn verify(
    query: &DocumentHistoryDriveQuery,
    proof: &[u8],
    document_type: DocumentTypeRef,
    version: &PlatformVersion,
) -> Result<(RootHash, DocumentHistoryDriveQueryExecutionResult), Error> {
    Drive::verify_document_history(query, proof, document_type, version)
}

fn lifecycle(page: &DocumentHistoryDriveQueryExecutionResult) -> &DocumentHistoryLifecycle {
    page.lifecycle
        .as_ref()
        .expect("pages of the history tree carry a lifecycle")
}

#[test]
fn should_serve_history_as_one_grovedb_proof_with_trusted_read_parity() {
    let version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
        false,
        version,
    )
    .unwrap();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = contract.document_type_for_name("profile").unwrap();
    let mut document = json_document_to_document(
        "tests/supporting_files/contract/dashpay/profile0.json",
        Some([8; 32].into()),
        document_type,
        version,
    )
    .unwrap();
    for revision in 1..=3 {
        document.set_revision(Some(revision));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::default_with_time(2000),
                true,
                None,
                version,
                None,
            )
            .unwrap();
    }
    let root = drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .unwrap()
        .unwrap();
    let mut failures = vec![];
    for document_id in [document.id().to_buffer(), [255; 32]] {
        for filter in [
            DocumentHistoryFilter::StartAtTime(2000),
            DocumentHistoryFilter::StartAfter {
                time_ms: 2000,
                revision: 1,
            },
            DocumentHistoryFilter::StartAtRevision(2),
            DocumentHistoryFilter::Revision(2),
            DocumentHistoryFilter::Revision(4),
            DocumentHistoryFilter::Revision(65536),
        ] {
            let query = DocumentHistoryDriveQuery {
                contract_id: contract.id().to_buffer(),
                document_type_name: "profile".into(),
                document_id,
                filter,
                limit: None,
            };
            let fetched = query
                .execute_no_proof(&drive, document_type, None, version)
                .unwrap();
            let proof = query
                .execute_with_proof(&drive, document_type, None, version)
                .unwrap();
            println!(
                "single proof sample {:?}, absent={}: {} bytes",
                query.filter,
                document_id == [255; 32],
                proof.len()
            );
            match grovedb::GroveDb::verify_query_raw(
                &proof,
                &query.construct_path_query(version).unwrap(),
                &version.drive.grove_version,
            ) {
                Ok((proved_root, rows)) => {
                    assert_eq!(proved_root, root);
                    assert_eq!(
                        rows.len(),
                        fetched.entries.len() + usize::from(document_id != [255; 32]) * 2
                    );
                    assert_eq!(
                        query
                            .verify_document_history_proof(&proof, document_type, version)
                            .unwrap(),
                        (root, fetched)
                    );
                }
                Err(error) => failures.push(format!(
                    "{:?}, absent={}: {error}",
                    query.filter,
                    document_id == [255; 32]
                )),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "history must be one ordinary GroveDB proof: {failures:#?}"
    );
}

#[test]
fn should_leave_exact_history_reads_unlimited_in_the_merged_query() {
    fn check(query: &Query, page_limits: &mut usize) {
        let children: Vec<_> = query
            .default_subquery_branch
            .subquery
            .iter()
            .chain(
                query
                    .conditional_subquery_branches
                    .iter()
                    .flat_map(|branches| branches.values())
                    .filter_map(|branch| branch.subquery.as_ref()),
            )
            .collect();
        assert!(query.left_to_right);
        if children.is_empty()
            && query
                .items
                .iter()
                .any(|item| !matches!(item, grovedb::QueryItem::Key(_)))
        {
            assert!(query.limit.is_some(), "only the history page is limited");
            *page_limits += 1;
        } else {
            assert_eq!(query.offset, None, "only the page may skip results");
            assert_eq!(
                query.limit, None,
                "exact selections and enclosing nodes must prove absence without truncation"
            );
        }
        for child in children {
            check(child, page_limits);
        }
    }
    let query = DocumentHistoryDriveQuery {
        contract_id: [1; 32],
        document_type_name: "profile".into(),
        document_id: [2; 32],
        filter: DocumentHistoryFilter::StartAtRevision(7),
        limit: Some(2),
    };
    let merged = query
        .construct_path_query(PlatformVersion::latest())
        .unwrap();
    assert_eq!(
        merged.path,
        contract_document_type_path_vec(&query.contract_id, &query.document_type_name)
    );
    assert_eq!(
        merged.query.limit, None,
        "a global limit could omit an exact sibling"
    );
    assert_eq!(merged.query.offset, None);
    let history = merged
        .query
        .query
        .conditional_subquery_branches
        .as_ref()
        .unwrap()
        .get(&grovedb::QueryItem::Key(vec![DOCUMENT_HISTORY_TREE_KEY]))
        .unwrap()
        .subquery
        .as_ref()
        .unwrap();
    assert!(history.add_parent_tree_on_subquery);
    let mut page_limits = 0;
    check(&merged.query.query, &mut page_limits);
    assert_eq!(page_limits, 1);
}

#[test]
fn should_serve_both_history_pointer_types_as_one_grovedb_proof() {
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;
    use dpp::platform_value::platform_value;

    let version = PlatformVersion::latest();
    for summable in [true, false] {
        let drive = setup_drive_with_initial_state_structure(Some(version));
        let mut schema = platform_value!({
            "type": "object", "documentsKeepHistory": true, "canBeDeleted": false,
            "properties": {"amount": {"type": "integer", "minimum": 0, "maximum": 100, "position": 0}},
            "required": ["amount"], "additionalProperties": false
        });
        if summable {
            schema
                .insert("documentsSummable".into(), "amount".into())
                .unwrap();
        }
        let contract = DataContractFactory::new(version.protocol_version)
            .unwrap()
            .create_with_value_config(
                [7; 32].into(),
                0,
                platform_value!({"note": schema}),
                None,
                None,
            )
            .unwrap()
            .data_contract_owned();
        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
            .unwrap();
        let document_type = contract.document_type_for_name("note").unwrap();
        let mut document = DocumentFactory::new(version.protocol_version)
            .unwrap()
            .create_document(
                &contract,
                [7; 32].into(),
                "note".into(),
                platform_value!({"amount": 5}),
            )
            .unwrap();
        for revision in 1..=3 {
            document.set_revision(Some(revision));
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    true,
                    BlockInfo::default_with_time(2000),
                    true,
                    None,
                    version,
                    None,
                )
                .unwrap();
        }
        let mut primary = contract_document_type_path_vec(contract.id().as_bytes(), "note");
        primary.push(vec![0]);
        let pointer = drive
            .grove
            .get_raw(
                primary.as_slice().into(),
                document.id().as_bytes(),
                None,
                &version.drive.grove_version,
            )
            .unwrap()
            .unwrap();
        assert!(matches!(
            (summable, pointer),
            (false, Element::Reference(..)) | (true, Element::ReferenceWithSumItem(..))
        ));
        let current_bytes = document
            .serialize(document_type, &contract, version)
            .unwrap();
        let root = drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .unwrap()
            .unwrap();
        for filter in [
            DocumentHistoryFilter::StartAtTime(0),
            DocumentHistoryFilter::StartAfter {
                time_ms: 2000,
                revision: 1,
            },
            DocumentHistoryFilter::StartAtRevision(2),
            DocumentHistoryFilter::Revision(2),
        ] {
            let query = DocumentHistoryDriveQuery {
                contract_id: contract.id().to_buffer(),
                document_type_name: "note".into(),
                document_id: document.id().to_buffer(),
                filter,
                limit: Some(1),
            };
            let page = query
                .execute_no_proof(&drive, document_type, None, version)
                .unwrap();
            let proof = query
                .execute_with_proof(&drive, document_type, None, version)
                .unwrap();
            let (proved_root, _) = grovedb::GroveDb::verify_query_raw(
                &proof,
                &query.construct_path_query(version).unwrap(),
                &version.drive.grove_version,
            )
            .expect("history uses one ordinary GroveDB proof");
            assert_eq!(proved_root, root);
            assert_eq!(
                query
                    .verify_document_history_proof(&proof, document_type, version)
                    .unwrap(),
                (root, page)
            );
            // These pages exclude the current revision, so these bytes belong
            // only to the resolved primary reference in the proof.
            let matches: Vec<_> = proof
                .windows(current_bytes.len())
                .enumerate()
                .filter_map(|(position, bytes)| (bytes == current_bytes).then_some(position))
                .collect();
            assert_eq!(matches.len(), 1);
            let mut changed = proof.clone();
            changed[matches[0] + current_bytes.len() - 1] ^= 1;
            let result = query.verify_document_history_proof(&changed, document_type, version);
            assert!(
                result.is_err() || result.unwrap().0 != root,
                "changed reference target bytes must never authenticate under the trusted root"
            );
        }
    }
}

#[test]
fn should_preserve_non_history_roots_fees_and_proofs_after_grove_upgrade() {
    use dpp::dashcore::hashes::{sha256, Hash};
    use grovedb_version::version::v4::GROVE_V4;

    for selected in [PlatformVersion::get(14).unwrap(), PlatformVersion::latest()] {
        // Historical bytes pin replay compatibility independently of agreement
        // between the current GroveDB engines.
        let (expected_root, expected_proof) = if selected.protocol_version == 14 {
            (
                "7dbeb13c417200f86e7f29d5f0967d62c222ae6f4abaf6d83a79adfc6ec732c1",
                "b5b70531d251fcc2dfb134f35dd34ad7599e8fcbb22aed4ea9e3e7e9f73e6cae",
            )
        } else {
            (
                "7ea3acea864c917efe5c201b7be554ebbb105d3c065ae85684c93f355223db31",
                "5a2bf14e60db3fb2b1deef28a8a9f94da1e0cbaab961ff958ad8bd0b15a01512",
            )
        };
        let mut reference = selected.clone();
        reference.drive.grove_version = GROVE_V4;
        let mut observations = vec![];
        for version in [&reference, selected] {
            let drive = setup_drive_with_initial_state_structure(Some(version));
            let contract = json_document_to_contract(
                "tests/supporting_files/contract/dashpay/dashpay-contract.json",
                false,
                version,
            )
            .unwrap();
            let contract_fee = drive
                .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
                .unwrap();
            let document_type = contract.document_type_for_name("profile").unwrap();
            let document = json_document_to_document(
                "tests/supporting_files/contract/dashpay/profile0.json",
                Some([8; 32].into()),
                document_type,
                version,
            )
            .unwrap();
            let document_fee = drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    true,
                    BlockInfo::default_with_time(2000),
                    true,
                    None,
                    version,
                    None,
                )
                .unwrap();
            let mut primary = contract_document_type_path_vec(contract.id().as_bytes(), "profile");
            primary.push(vec![0]);
            let document_query = PathQuery::new_single_key(primary.clone(), document.id().to_vec());
            let absent_query = PathQuery::new_single_key(primary, vec![255; 32]);
            let query = PathQuery::merge(
                vec![&document_query, &absent_query],
                &version.drive.grove_version,
            )
            .unwrap();
            let reference_query =
                PathQuery::merge(vec![&document_query, &absent_query], &GROVE_V4).unwrap();
            assert_eq!(query, reference_query);
            let encode = |query: &PathQuery| {
                bincode::encode_to_vec(query, bincode::config::standard().with_big_endian())
                    .unwrap()
            };
            assert_eq!(encode(&query), encode(&reference_query));
            let (rows, skipped) = drive
                .grove_get_raw_path_query(
                    &query,
                    None,
                    grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType,
                    &mut vec![],
                    &version.drive,
                )
                .unwrap();
            assert_eq!(skipped, 0);
            assert_eq!(rows.len(), 1);
            let proof = drive
                .grove_get_proved_path_query(&query, None, &mut vec![], &version.drive)
                .unwrap();
            let (root, _) =
                grovedb::GroveDb::verify_query_raw(&proof, &query, &version.drive.grove_version)
                    .unwrap();
            assert_eq!(
                grovedb::GroveDb::verify_query_raw(&proof, &reference_query, &GROVE_V4)
                    .unwrap()
                    .0,
                root,
                "released Grove V4 verifier must accept offset-free V5 proofs"
            );
            assert_eq!(
                root,
                drive
                    .grove
                    .root_hash(None, &version.drive.grove_version)
                    .unwrap()
                    .unwrap()
            );
            assert_eq!(hex::encode(root), expected_root);
            assert_eq!(sha256::Hash::hash(&proof).to_string(), expected_proof);
            assert_eq!(
                (contract_fee.storage_fee, contract_fee.processing_fee),
                (110_781_000, 1_773_200)
            );
            assert_eq!(
                (document_fee.storage_fee, document_fee.processing_fee),
                (35_235_000, 901_140)
            );
            println!("non-history PV{}: root={}, proof_sha256={}, contract_fee={contract_fee:?}, document_fee={document_fee:?}", version.protocol_version, hex::encode(root), sha256::Hash::hash(&proof));
            let mut reads = vec![];
            for grove_version in [&GROVE_V4, &version.drive.grove_version] {
                // A range of document types descends into each primary tree.
                // Branch-local limits already used by ordinary queries must
                // keep both read results and metered work across the upgrade.
                let mut types = Query::new_range_full();
                let mut primary = Query::new_single_key(vec![0]);
                let mut page = Query::new_range_full();
                page.limit = Some(1);
                primary.set_subquery(page);
                types.set_subquery(primary);
                let mut path = contract_document_type_path_vec(contract.id().as_bytes(), "profile");
                path.pop();
                let nested = PathQuery::new_unsized(path, types);
                let read = drive.grove.query_raw(
                    &nested,
                    false,
                    true,
                    true,
                    grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType,
                    None,
                    grove_version,
                );
                reads.push((read.value.unwrap(), read.cost));
            }
            assert_eq!(
                reads[0], reads[1],
                "nested local-limit reads must preserve metered work"
            );
            observations.push((root, proof, contract_fee, document_fee));
        }
        assert_eq!(
            observations[0], observations[1],
            "protocol {} roots, proofs and fees must stay equal to Grove V4",
            selected.protocol_version
        );
    }
}

#[test]
fn should_authenticate_history_pages_metadata_and_absence() {
    let version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
        false,
        version,
    )
    .unwrap();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = contract.document_type_for_name("profile").unwrap();
    let mut document = json_document_to_document(
        "tests/supporting_files/contract/dashpay/profile0.json",
        Some([8; 32].into()),
        document_type,
        version,
    )
    .unwrap();
    for revision in 1..=22 {
        document.set_revision(Some(revision));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::default_with_time(2000),
                true,
                None,
                version,
                None,
            )
            .unwrap();
    }
    let mut query = DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "profile".into(),
        document_id: document.id().to_buffer(),
        filter: DocumentHistoryFilter::StartAtTime(2000),
        limit: Some(10),
    };
    let mut seen = vec![];
    loop {
        let (page, proof) = prove(&drive, &query, document_type, version);
        let (_, verified) = verify(&query, &proof, document_type, version).unwrap();
        assert_eq!(verified, page);
        assert_eq!(lifecycle(&verified).remaining_revisions, 22);
        assert_eq!(lifecycle(&verified).state, DocumentHistoryState::Active);
        seen.extend(page.entries.iter().map(|entry| entry.revision));
        let mut missing_proof = proof.clone();
        missing_proof.truncate(proof.len() / 2);
        assert!(verify(&query, &missing_proof, document_type, version).is_err());
        let Some(last) = page.entries.last() else {
            break;
        };
        query.filter = DocumentHistoryFilter::StartAfter {
            time_ms: last.time_ms,
            revision: last.revision,
        };
    }
    assert_eq!(seen, (1..=22).collect::<Vec<_>>());
    for filter in [
        DocumentHistoryFilter::Revision(1),
        DocumentHistoryFilter::Revision(2),
        DocumentHistoryFilter::Revision(22),
        DocumentHistoryFilter::Revision(23),
        DocumentHistoryFilter::StartAtRevision(7),
        DocumentHistoryFilter::StartAtRevision(20),
    ] {
        let expected = match filter {
            DocumentHistoryFilter::Revision(revision) => {
                if revision <= 22 {
                    vec![revision]
                } else {
                    vec![]
                }
            }
            DocumentHistoryFilter::StartAtRevision(revision) => {
                (revision..=(revision + 9).min(22)).collect()
            }
            _ => unreachable!(),
        };
        query.filter = filter;
        query.limit = None;
        let (page, proof) = prove(&drive, &query, document_type, version);
        let (_, verified) = verify(&query, &proof, document_type, version).unwrap();
        assert_eq!(verified, page);
        assert_eq!(
            page.entries
                .iter()
                .map(|entry| entry.revision)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(lifecycle(&page).remaining_revisions, 22);
    }
    query.filter = DocumentHistoryFilter::StartAtTime(0);
    let (_, old_proof) = prove(&drive, &query, document_type, version);
    document.set_revision(Some(23));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::default_with_time(3000),
            true,
            None,
            version,
            None,
        )
        .unwrap();
    let (_, new_proof) = prove(&drive, &query, document_type, version);
    assert_ne!(
        verify(&query, &old_proof, document_type, version)
            .unwrap()
            .0,
        verify(&query, &new_proof, document_type, version)
            .unwrap()
            .0,
        "proofs authenticate their own state root"
    );
    let bytes = document
        .serialize(document_type, &contract, version)
        .unwrap();
    for key in [vec![0; 8], vec![0; 15], vec![0; 17]] {
        assert!(query
            .decode_entries(
                vec![(key, Element::new_item(bytes.clone()))],
                document_type,
                version
            )
            .is_err());
    }
    query.document_id = [255; 32];
    query.filter = DocumentHistoryFilter::Revision(2);
    let (page, proof) = prove(&drive, &query, document_type, version);
    let (_, verified) = verify(&query, &proof, document_type, version).unwrap();
    assert_eq!(page, verified);
    assert_eq!(lifecycle(&verified).state, DocumentHistoryState::Absent);
    assert_eq!(lifecycle(&verified).remaining_revisions, 0);
    assert!(verified.entries.is_empty());

    let history_path = document_history_path(
        &query.contract_id,
        &query.document_type_name,
        &query.document_id,
    );
    drive
        .grove
        .insert(
            history_path[..5].to_vec().as_slice(),
            &query.document_id,
            Element::empty_provable_count_tree(),
            None,
            None,
            &version.drive.grove_version,
        )
        .value
        .unwrap();
    query.filter = DocumentHistoryFilter::StartAtTime(0);
    let (page, proof) = prove(&drive, &query, document_type, version);
    let (_, verified) = verify(&query, &proof, document_type, version).unwrap();
    assert_eq!(page, verified);
    assert_eq!(lifecycle(&verified).state, DocumentHistoryState::Absent);
    assert_eq!(lifecycle(&verified).remaining_revisions, 0);
}

#[test]
fn should_reject_point_in_time_history_reads_only_after_activation() {
    let query = SingleDocumentDriveQuery {
        contract_id: [1; 32],
        document_type_name: "note".into(),
        document_type_keeps_history: true,
        document_id: [2; 32],
        block_time_ms: Some(1000),
        contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
    };
    for protocol in [12, 13, 14] {
        assert!(query
            .construct_path_query(PlatformVersion::get(protocol).unwrap())
            .is_ok());
    }
    assert!(query
        .construct_path_query(PlatformVersion::get(15).unwrap())
        .is_err());
}

#[test]
fn should_reject_invalid_selectors_and_unsupported_protocols() {
    let mut query = DocumentHistoryDriveQuery {
        contract_id: [1; 32],
        document_type_name: "note".into(),
        document_id: [2; 32],
        filter: DocumentHistoryFilter::StartAtTime(0),
        limit: None,
    };
    for filter in [
        DocumentHistoryFilter::Revision(0),
        DocumentHistoryFilter::StartAtRevision(65537),
        DocumentHistoryFilter::StartAtTime(1 << 63),
        DocumentHistoryFilter::StartAfter {
            time_ms: 0,
            revision: 0,
        },
    ] {
        query.filter = filter;
        assert!(query.validate().is_err());
    }
    query.filter = DocumentHistoryFilter::Revision(65535);
    assert!(query.validate().is_ok());
    query.limit = Some(2);
    assert!(query.validate().is_err());
    query.limit = Some(1);
    for protocol in [12, 13, 14] {
        assert!(query
            .construct_path_query(PlatformVersion::get(protocol).unwrap())
            .is_err());
    }
    assert!(query
        .construct_path_query(PlatformVersion::get(15).unwrap())
        .is_ok());
}

#[test]
fn should_address_every_revision_whose_offset_fits_the_grovedb_offset() {
    let mut query = DocumentHistoryDriveQuery {
        contract_id: [1; 32],
        document_type_name: "note".into(),
        document_id: [2; 32],
        filter: DocumentHistoryFilter::StartAtTime(0),
        limit: Some(1),
    };
    for (revision, offset) in [(65535, 65534), (65536, u16::MAX)] {
        for filter in [
            DocumentHistoryFilter::Revision(revision),
            DocumentHistoryFilter::StartAtRevision(revision),
        ] {
            query.filter = filter;
            let path_query = query
                .construct_path_query(PlatformVersion::get(15).unwrap())
                .expect("revision addressable through a u16 offset");
            fn leaf_offset(query: &Query) -> Option<u16> {
                query
                    .offset
                    .or_else(|| {
                        query
                            .default_subquery_branch
                            .subquery
                            .as_ref()
                            .and_then(|q| leaf_offset(q))
                    })
                    .or_else(|| {
                        query
                            .conditional_subquery_branches
                            .as_ref()
                            .and_then(|branches| {
                                branches.values().find_map(|branch| {
                                    branch.subquery.as_ref().and_then(|q| leaf_offset(q))
                                })
                            })
                    })
            }
            assert_eq!(path_query.query.offset, None);
            assert_eq!(leaf_offset(&path_query.query.query), Some(offset));
        }
    }
    for filter in [
        DocumentHistoryFilter::Revision(65537),
        DocumentHistoryFilter::StartAtRevision(65537),
    ] {
        query.filter = filter;
        assert!(query.validate().is_err());
    }
}

#[test]
fn should_use_sequence_one_for_an_immutable_document_without_a_revision() {
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;
    use dpp::platform_value::platform_value;
    let version = PlatformVersion::get(15).unwrap();
    let drive = setup_drive_with_initial_state_structure(Some(version));
    let contract = DataContractFactory::new(15).unwrap().create_with_value_config([7; 32].into(), 0, platform_value!({
        "note": {"type": "object", "documentsMutable": false, "documentsKeepHistory": true, "canBeDeleted": false,
        "properties": {"message": {"type": "string", "maxLength": 256, "position": 0}}, "required": ["message"], "additionalProperties": false}
    }), None, None).unwrap().data_contract_owned();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = contract.document_type_for_name("note").unwrap();
    let document = DocumentFactory::new(15)
        .unwrap()
        .create_document(
            &contract,
            [7; 32].into(),
            "note".into(),
            platform_value!({"message": "immutable"}),
        )
        .unwrap();
    assert_eq!(document.revision(), None);
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default_with_time(2000),
            true,
            None,
            version,
            None,
        )
        .unwrap();
    let query = DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "note".into(),
        document_id: document.id().to_buffer(),
        filter: DocumentHistoryFilter::Revision(1),
        limit: None,
    };
    let (history, proof) = prove(&drive, &query, document_type, version);
    assert_eq!(history.entries[0].revision, 1);
    assert_eq!(history.entries[0].document.revision(), None);
    assert_eq!(lifecycle(&history).remaining_revisions, 1);
    let (_, verified) = verify(&query, &proof, document_type, version).unwrap();
    assert_eq!(verified, history);
}

#[test]
fn should_bound_estimates_for_deep_plain_and_summable_histories() {
    use crate::util::storage_flags::StorageFlags;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;
    use dpp::platform_value::platform_value;
    use std::borrow::Cow;
    let version = PlatformVersion::get(15).unwrap();
    for summable in [false, true] {
        let drive = setup_drive_with_initial_state_structure(Some(version));
        let mut schema = platform_value!({
            "type": "object", "documentsKeepHistory": true, "canBeDeleted": false,
            "documentsCountable": true,
            "properties": {"amount": {"type": "integer", "minimum": 0, "maximum": 4294967295i64, "position": 0}, "message": {"type": "string", "maxLength": 1024, "position": 1}},
            "required": ["amount", "message"], "additionalProperties": false
        });
        if summable {
            schema
                .insert("documentsSummable".into(), "amount".into())
                .unwrap();
        }
        let contract = DataContractFactory::new(15)
            .unwrap()
            .create_with_value_config(
                [7; 32].into(),
                0,
                platform_value!({"note": schema}),
                None,
                None,
            )
            .unwrap()
            .data_contract_owned();
        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
            .unwrap();
        let document_type = contract.document_type_for_name("note").unwrap();
        let mut document = DocumentFactory::new(15)
            .unwrap()
            .create_document(
                &contract,
                [7; 32].into(),
                "note".into(),
                platform_value!({"amount": 1, "message": "a".repeat(1024)}),
            )
            .unwrap();
        for revision in 1..=257 {
            document.set_revision(Some(revision));
            let info = DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((
                        &document,
                        Some(Cow::Owned(StorageFlags::new_single_epoch(
                            (revision % 3) as u16,
                            Some([revision as u8; 32]),
                        ))),
                    )),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            };
            let block = BlockInfo::default_with_time(revision * 1000);
            let estimated = (revision == 257).then(|| {
                drive
                    .add_document_for_contract(
                        info.clone(),
                        true,
                        block,
                        false,
                        None,
                        version,
                        None,
                    )
                    .unwrap()
            });
            let actual = drive
                .add_document_for_contract(info, true, block, true, None, version, None)
                .unwrap();
            if let Some(estimated) = estimated {
                assert!(
                    estimated.storage_fee >= actual.storage_fee,
                    "storage estimate {estimated:?}, actual {actual:?}"
                );
                assert!(
                    estimated.processing_fee >= actual.processing_fee,
                    "processing estimate {estimated:?}, actual {actual:?}"
                );
            }
        }
    }
}

#[test]
fn should_include_reference_hops_in_history_pointer_size_estimates() {
    use crate::drive::constants::DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE;
    use grovedb::reference_path::ReferencePathType::UpstreamRootHeightReference;
    let version = PlatformVersion::get(15).unwrap();
    for flags in [
        None,
        Some(vec![1; 35]),
        Some(vec![1; 255]),
        Some(vec![1; 65536]),
    ] {
        let reference = UpstreamRootHeightReference(
            4,
            vec![vec![DOCUMENT_HISTORY_TREE_KEY], vec![1; 32], vec![1; 16]],
        );
        let flags_len = flags.as_ref().map_or(0, |flags| flags.len() as u32);
        for sum in [None, Some(0), Some(i64::MAX), Some(i64::MIN)] {
            let (element, estimate) = if let Some(sum) = sum {
                (
                    Element::ReferenceWithSumItem(reference.clone(), Some(1), sum, flags.clone()),
                    Element::required_reference_with_sum_item_space(
                        DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE,
                        flags_len,
                        &version.drive.grove_version,
                    )
                    .unwrap(),
                )
            } else {
                (
                    Element::Reference(reference.clone(), Some(1), flags.clone()),
                    Element::required_item_space(
                        DOCUMENT_HISTORY_CURRENT_REFERENCE_PATH_SIZE,
                        flags_len,
                        &version.drive.grove_version,
                    )
                    .unwrap(),
                )
            };
            let serialized_size = element
                .serialized_size(&version.drive.grove_version)
                .unwrap();
            assert!(
                estimate as usize >= serialized_size,
                "pointer estimate {estimate} is smaller than its serialized size {serialized_size}"
            );
        }
    }
}

#[test]
fn should_charge_count_tree_overhead_when_propagating_history_roots() {
    use crate::drive::document::paths::contract_documents_primary_key_path;
    use crate::util::storage_flags::StorageFlags;
    use grovedb::batch::KeyInfoPath;
    use grovedb::{EstimatedLayerCount, EstimatedLayerSizes, EstimatedSumTrees};
    use std::collections::HashMap;
    let version = PlatformVersion::get(15).unwrap();
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
        false,
        version,
    )
    .unwrap();
    let document_type = contract.document_type_for_name("profile").unwrap();
    let document = json_document_to_document(
        "tests/supporting_files/contract/dashpay/profile0.json",
        Some([8; 32].into()),
        document_type,
        version,
    )
    .unwrap();
    let info = DocumentAndContractInfo {
        owned_document_info: OwnedDocumentInfo {
            document_info: DocumentInfo::DocumentRefInfo((&document, None)),
            owner_id: None,
        },
        contract: &contract,
        document_type,
    };
    let mut layers = HashMap::new();
    Drive::add_estimation_costs_for_add_document_to_primary_storage(
        &info,
        contract_documents_primary_key_path(contract.id().as_slice(), "profile"),
        &mut layers,
        version,
    )
    .unwrap();
    let mut root = contract_document_type_path_vec(contract.id().as_slice(), "profile");
    root.push(vec![DOCUMENT_HISTORY_TREE_KEY]);
    let mut layer = layers
        .remove(&KeyInfoPath::from_known_path(
            root.iter().map(Vec::as_slice),
        ))
        .unwrap();
    layer.estimated_layer_count = EstimatedLayerCount::ApproximateElements(7);
    let propagate = |layer: &grovedb::EstimatedLayerInformation| {
        grovedb::GroveDb::average_case_merk_insert_tree(
            &grovedb::batch::key_info::KeyInfo::KnownKey(vec![9; 32]),
            &None,
            grovedb::TreeType::ProvableCountTree,
            grovedb::TreeType::NormalTree,
            0,
            Some(layer),
            &version.drive.grove_version,
        )
    };
    let actual = propagate(&layer);
    actual.value.unwrap();
    layer.estimated_layer_sizes = EstimatedLayerSizes::AllSubtrees(
        32,
        EstimatedSumTrees::NoSumTrees,
        Some(StorageFlags::approximate_size(true, None)),
    );
    let plain = propagate(&layer);
    plain.value.unwrap();
    assert_eq!(actual.cost.seek_count, plain.cost.seek_count);
    assert_eq!(
        actual.cost.storage_cost.replaced_bytes,
        plain.cost.storage_cost.replaced_bytes + 4 * 8,
        "each rewritten ancestor carries its child's authenticated count"
    );
}

#[test]
fn should_reject_history_keys_that_are_not_sixteen_bytes() {
    let version = PlatformVersion::get(15).unwrap();
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
        false,
        version,
    )
    .unwrap();
    let document_type = contract.document_type_for_name("profile").unwrap();
    let mut document = json_document_to_document(
        "tests/supporting_files/contract/dashpay/profile0.json",
        Some([8; 32].into()),
        document_type,
        version,
    )
    .unwrap();
    document.set_revision(Some(1));
    let query = DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "profile".into(),
        document_id: document.id().to_buffer(),
        filter: DocumentHistoryFilter::StartAtTime(0),
        limit: None,
    };
    let bytes = document
        .serialize(document_type, &contract, version)
        .unwrap();
    let mut key = encode_u64(1000);
    key.extend(encode_u64(1));
    assert_eq!(
        query
            .decode_entries(
                vec![(key.clone(), Element::new_item(bytes.clone()))],
                document_type,
                version
            )
            .unwrap()[0]
            .revision,
        1
    );
    let mut overlong = key.clone();
    overlong.push(0);
    for invalid in [key[..8].to_vec(), key[..15].to_vec(), overlong] {
        let error = query
            .decode_entries(
                vec![(invalid, Element::new_item(bytes.clone()))],
                document_type,
                version,
            )
            .unwrap_err();
        assert!(
            error.to_string().contains("exactly sixteen bytes"),
            "{error}"
        );
    }
}

#[test]
fn should_reject_history_proofs_that_omit_a_queried_branch() {
    let version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
        false,
        version,
    )
    .unwrap();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = contract.document_type_for_name("profile").unwrap();
    let mut document = json_document_to_document(
        "tests/supporting_files/contract/dashpay/profile0.json",
        Some([8; 32].into()),
        document_type,
        version,
    )
    .unwrap();
    document.set_revision(Some(1));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::default_with_time(2000),
            true,
            None,
            version,
            None,
        )
        .unwrap();
    let query = DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "profile".into(),
        document_id: document.id().to_buffer(),
        filter: DocumentHistoryFilter::StartAtTime(0),
        limit: Some(10),
    };
    let (page, honest) = prove(&drive, &query, document_type, version);
    assert_eq!(lifecycle(&page).state, DocumentHistoryState::Active);
    let (_, verified) = verify(&query, &honest, document_type, version).unwrap();
    assert_eq!(verified, page);

    // A node that answers only for the lifecycle record and the history tree,
    // leaving the current pointer out of the proof entirely, must not be able
    // to pass the document off as absent with an empty page.
    for omitted in [0u8, DOCUMENT_HISTORY_TREE_KEY] {
        let branches = [0u8, 1, DOCUMENT_HISTORY_TREE_KEY]
            .into_iter()
            .filter(|branch| *branch != omitted)
            .map(|branch| {
                let mut path =
                    contract_document_type_path_vec(&query.contract_id, &query.document_type_name);
                path.push(vec![branch]);
                let mut key_query = Query::new();
                key_query.insert_key(query.document_id.to_vec());
                PathQuery::new(path, SizedQuery::new(key_query, None, None))
            })
            .collect::<Vec<_>>();
        let mut narrowed =
            PathQuery::merge(branches.iter().collect(), &version.drive.grove_version).unwrap();
        narrowed.query.limit = Some(2);
        let forged = drive
            .grove_get_proved_path_query(&narrowed, None, &mut vec![], &version.drive)
            .unwrap();
        assert!(
            verify(&query, &forged, document_type, version).is_err(),
            "a metadata proof that omits branch {omitted} verified as an authenticated absence"
        );
    }
}
