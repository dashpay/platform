use super::DocumentHistoryProof;
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
/// tree, returning the two GroveDB proofs for tampering.
fn prove(
    drive: &Drive,
    query: &DocumentHistoryDriveQuery,
    document_type: DocumentTypeRef,
    version: &PlatformVersion,
) -> (
    DocumentHistoryDriveQueryExecutionResult,
    DocumentHistoryProof,
) {
    let page = drive
        .fetch_document_history(query, document_type, None, version)
        .unwrap();
    let proof = DocumentHistoryProof::from_bytes(
        &drive
            .prove_document_history(query, document_type, None, version)
            .unwrap(),
    )
    .unwrap();
    (page, proof)
}

fn verify(
    query: &DocumentHistoryDriveQuery,
    proof: &DocumentHistoryProof,
    document_type: DocumentTypeRef,
    version: &PlatformVersion,
) -> Result<(RootHash, DocumentHistoryDriveQueryExecutionResult), Error> {
    Drive::verify_document_history(query, &proof.to_bytes()?, document_type, version)
}

fn lifecycle(page: &DocumentHistoryDriveQueryExecutionResult) -> &DocumentHistoryLifecycle {
    page.lifecycle
        .as_ref()
        .expect("pages of the history tree carry a lifecycle")
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
        missing_proof.entries_proof = None;
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
    let (_, mut mixed_proof) = prove(&drive, &query, document_type, version);
    mixed_proof.metadata_proof = old_proof.metadata_proof;
    assert!(
        verify(&query, &mixed_proof, document_type, version).is_err(),
        "proofs from different states must not combine"
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
    assert!(proof.entries_proof.is_none());
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
    assert!(drive
        .fetch_document_history(&query, document_type, None, version)
        .is_err());
    assert!(drive
        .prove_document_history(&query, document_type, None, version)
        .is_err());

    // A hostile peer can still assemble the two individually valid GroveDB
    // proofs, so the verifier must reject the corrupt empty-present tree too.
    let proof = DocumentHistoryProof {
        metadata_proof: drive
            .grove_get_proved_path_query(
                &query.metadata_path_query(version).unwrap(),
                None,
                &mut vec![],
                &version.drive,
            )
            .unwrap(),
        entries_proof: Some(
            drive
                .grove_get_proved_path_query(
                    &query.construct_path_query(version).unwrap(),
                    None,
                    &mut vec![],
                    &version.drive,
                )
                .unwrap(),
        ),
    };
    assert!(verify(&query, &proof, document_type, version).is_err());
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
        DocumentHistoryFilter::StartAtRevision(65536),
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
fn should_reject_metadata_proofs_that_omit_a_queried_branch() {
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
        let metadata_proof = drive
            .grove_get_proved_path_query(&narrowed, None, &mut vec![], &version.drive)
            .unwrap();
        let forged = DocumentHistoryProof {
            entries_proof: None,
            metadata_proof,
        };
        assert!(
            verify(&query, &forged, document_type, version).is_err(),
            "a metadata proof that omits branch {omitted} verified as an authenticated absence"
        );
    }
}

#[test]
fn should_round_trip_the_proof_envelope_with_and_without_an_entries_proof() {
    for entries_proof in [Some(vec![1u8, 2, 3]), None, Some(vec![])] {
        let proof = DocumentHistoryProof {
            metadata_proof: vec![9u8; 5],
            entries_proof,
        };
        assert_eq!(
            DocumentHistoryProof::from_bytes(&proof.to_bytes().unwrap()).unwrap(),
            proof
        );
    }
}

/// A proof is produced only by a node, so bytes that are not exactly one
/// envelope are not the proof this code believes it is reading.
#[test]
fn should_reject_truncated_and_padded_proof_envelopes() {
    let bytes = DocumentHistoryProof {
        metadata_proof: vec![9u8; 5],
        entries_proof: Some(vec![1u8, 2, 3]),
    }
    .to_bytes()
    .unwrap();
    assert!(DocumentHistoryProof::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    let mut padded = bytes.clone();
    padded.push(0);
    assert!(DocumentHistoryProof::from_bytes(&padded).is_err());
    assert!(DocumentHistoryProof::from_bytes(&[]).is_err());
}
