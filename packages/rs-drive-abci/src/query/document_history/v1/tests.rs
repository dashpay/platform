use super::*;
use crate::query::tests::setup_platform;
use dapi_grpc::platform::v0::get_document_history_request::get_document_history_request_v1::Cursor;
use dapi_grpc::platform::v0::get_document_history_request::GetDocumentHistoryRequestV0;
use dapi_grpc::platform::v0::get_document_history_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{
    GetDocumentHistoryRequest, GetDocumentHistoryResponse, Proof, ResponseMetadata,
};
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::{ExtendedBlockInfoV0, ExtendedBlockInfoV0Setters};
use dpp::bls::SecretKey;
use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
use drive::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use drive_proof_verifier::types::DocumentHistory;
use drive_proof_verifier::{ContextProvider, ContextProviderError, FromProof};
use std::sync::Arc;
use tenderdash_abci::{
    proto::types::{CanonicalVote, SignedMsgType, StateId},
    signatures::{Hashable, Signable},
};

struct Provider {
    contract: Arc<DataContract>,
    key: [u8; 48],
}
impl ContextProvider for Provider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok((id == &self.contract.id()).then(|| self.contract.clone()))
    }
    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
    fn get_quorum_public_key(
        &self,
        _: u32,
        _: [u8; 32],
        _: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Ok(self.key)
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

fn signed_proof(
    bytes: Vec<u8>,
    root: [u8; 32],
    metadata: &ResponseMetadata,
    key: &SecretKey,
    quorum_type: u32,
) -> Proof {
    let state = StateId {
        app_version: metadata.protocol_version as u64,
        core_chain_locked_height: metadata.core_chain_locked_height,
        time: metadata.time_ms,
        app_hash: root.to_vec(),
        height: metadata.height,
    };
    let commit = CanonicalVote {
        r#type: SignedMsgType::Precommit.into(),
        block_id: vec![7; 32],
        chain_id: metadata.chain_id.clone(),
        height: metadata.height as i64,
        round: 0,
        state_id: state
            .calculate_msg_hash(&metadata.chain_id, metadata.height as i64, 0)
            .unwrap(),
    };
    let digest = commit
        .calculate_sign_hash(
            &metadata.chain_id,
            quorum_type.try_into().unwrap(),
            &[9; 32],
            metadata.height as i64,
            0,
        )
        .unwrap();
    Proof {
        grovedb_proof: bytes,
        quorum_hash: vec![9; 32],
        signature: key.sign(&digest).unwrap().to_bytes().to_vec(),
        round: 0,
        block_id_hash: vec![7; 32],
        quorum_type,
    }
}

/// The proof a proved history response answers with; a proved response never
/// carries the history itself.
fn proof_mut(response: &mut GetDocumentHistoryResponseV1) -> &mut Proof {
    match response.result.as_mut() {
        Some(ResponseResult::Proof(proof)) => proof,
        _ => panic!("a proved history query answers with a proof"),
    }
}

#[test]
fn should_round_trip_history_api_through_quorum_and_grove_proof_verification() {
    history_api_proof_round_trip(false);
}

#[test]
fn should_reject_legacy_proof_envelopes_in_history_v1_through_sdk() {
    history_api_proof_round_trip(true);
}

#[test]
fn should_verify_signed_legacy_history_from_protocol_14_with_protocol_15_client() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, Some(14));
    let mut state = state.as_ref().clone();
    let contract = json_document_to_contract(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json"
        ),
        false,
        version,
    )
    .unwrap();
    platform
        .drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = contract.document_type_for_name("profile").unwrap();
    let mut document = json_document_to_document(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rs-drive/tests/supporting_files/contract/dashpay/profile0.json"
        ),
        Some([8; 32].into()),
        document_type,
        version,
    )
    .unwrap();
    for revision in 1..=3 {
        document.set_revision(Some(revision));
        platform
            .drive
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
                BlockInfo::default_with_time(revision * 1000),
                true,
                None,
                version,
            )
            .unwrap();
    }
    let key = SecretKey::from_be_bytes(&[1; 32]).unwrap();
    let provider = Provider {
        contract: Arc::new(contract.clone()),
        key: key.public_key().to_bytes(),
    };
    state.last_committed_block_info = Some(
        ExtendedBlockInfoV0 {
            basic_info: BlockInfo {
                height: 42,
                core_height: 12,
                time_ms: 4000,
                epoch: Default::default(),
            },
            app_hash: [0; 32],
            quorum_hash: [9; 32],
            block_id_hash: [7; 32],
            proposer_pro_tx_hash: [0; 32],
            signature: [0; 96],
            round: 0,
        }
        .into(),
    );
    let metadata = platform.response_metadata_v0(&state, CheckpointUsed::Current);
    assert_eq!(metadata.protocol_version, 14);
    let root = platform
        .drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .value
        .unwrap();
    let signed = signed_proof(
        vec![],
        root,
        &metadata,
        &key,
        platform.config.validator_set.quorum_type as u32,
    );
    let committed = state.last_committed_block_info.as_mut().unwrap();
    committed.set_app_hash(root);
    committed.set_signature(signed.signature.try_into().unwrap());
    let request: GetDocumentHistoryRequest = GetDocumentHistoryRequestV0 {
        data_contract_id: contract.id().to_vec(),
        document_type_name: "profile".into(),
        document_id: document.id().to_vec(),
        limit: Some(2),
        offset: None,
        // V0's start time is exclusive: the SDK asks for revisions from 2000 on as after 1999.
        start_at_ms: 1999,
        prove: true,
    }
    .into();
    let response = platform
        .query_document_history(request.clone(), &state, version)
        .unwrap()
        .into_data()
        .unwrap();
    assert!(matches!(
        response.version.as_ref(),
        Some(ResponseVersion::V0(_))
    ));
    for client_version in [version, PlatformVersion::latest()] {
        let history = DocumentHistory::maybe_from_proof(
            request.clone(),
            response.clone(),
            Network::Testnet,
            client_version,
            &provider,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            history
                .entries
                .iter()
                .map(|entry| (entry.time_ms, entry.revision))
                .collect::<Vec<_>>(),
            vec![(2000, 2), (3000, 3)],
        );
        assert_eq!(history.lifecycle, None);
    }
    if std::env::var_os("UPDATE_DOCUMENT_HISTORY_VECTORS").is_some() {
        let vector = serde_json::json!({
            "contract": hex::encode(contract.serialize_to_bytes_with_platform_version(version).unwrap()),
            "quorum_public_key": hex::encode(provider.key),
            "request": request,
            "response": response,
        });
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../rs-sdk/tests/vectors/document_history/pv14_time.json");
        std::fs::write(
            path,
            format!("{}\n", serde_json::to_string(&vector).unwrap()),
        )
        .unwrap();
    }
}

fn history_api_proof_round_trip(gapped: bool) {
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let mut state = state.as_ref().clone();
    let contract = json_document_to_contract(concat!(env!("CARGO_MANIFEST_DIR"), "/../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json"), false, version).unwrap();
    platform
        .drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = contract.document_type_for_name("profile").unwrap();
    let mut document = json_document_to_document(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rs-drive/tests/supporting_files/contract/dashpay/profile0.json"
        ),
        Some([8; 32].into()),
        document_type,
        version,
    )
    .unwrap();
    for revision in 1..=3 {
        if gapped && revision == 2 {
            continue;
        }
        document.set_revision(Some(revision));
        platform
            .drive
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
            )
            .unwrap();
    }
    let key = SecretKey::from_be_bytes(&[1; 32]).unwrap();
    let provider = Provider {
        contract: Arc::new(contract.clone()),
        key: key.public_key().to_bytes(),
    };
    state.last_committed_block_info = Some(
        dpp::block::extended_block_info::v0::ExtendedBlockInfoV0 {
            basic_info: BlockInfo {
                height: 42,
                core_height: 12,
                time_ms: 3000,
                epoch: Default::default(),
            },
            app_hash: [0; 32],
            quorum_hash: [9; 32],
            block_id_hash: [7; 32],
            proposer_pro_tx_hash: [0; 32],
            signature: [0; 96],
            round: 0,
        }
        .into(),
    );
    let metadata = platform.response_metadata_v0(&state, CheckpointUsed::Current);
    let root = platform
        .drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .value
        .unwrap();
    let signed = signed_proof(
        vec![],
        root,
        &metadata,
        &key,
        platform.config.validator_set.quorum_type as u32,
    );
    let committed = state.last_committed_block_info.as_mut().unwrap();
    committed.set_app_hash(root);
    committed.set_signature(signed.signature.try_into().unwrap());
    if gapped {
        for (wire_selector, filter) in [
            (Filter::Revision(3), DocumentHistoryFilter::Revision(3)),
            (
                Filter::StartAtRevision(3),
                DocumentHistoryFilter::StartAtRevision(3),
            ),
        ] {
            let request = GetDocumentHistoryRequestV1 {
                data_contract_id: contract.id().to_vec(),
                document_type_name: "profile".into(),
                document_id: document.id().to_vec(),
                limit: None,
                prove: true,
                filter: Some(wire_selector),
            };
            // Drive refuses revision reads over a gapped history; the handler
            // must answer that as an invalid argument, not as a node error
            // the client would retry.
            for prove in [true, false] {
                let mut refused = request.clone();
                refused.prove = prove;
                let result = platform
                    .query_document_history_v1(refused, &state, version)
                    .expect("a refused revision read is a query error, not a node error");
                assert!(!result.is_valid());
                assert!(
                    matches!(result.errors.first(), Some(QueryError::Query(_))),
                    "prove={prove}: {:?}",
                    result.errors
                );
            }
            let mut time_request = request.clone();
            time_request.filter = Some(Filter::StartAtMs(0));
            let mut response = platform
                .query_document_history_v1(time_request, &state, version)
                .unwrap()
                .into_data()
                .unwrap();
            let query = DocumentHistoryDriveQuery {
                contract_id: contract.id().to_buffer(),
                document_type_name: "profile".into(),
                document_id: document.id().to_buffer(),
                filter,
                limit: None,
            };
            let proof = proof_mut(&mut response);
            let bytes = platform
                .drive
                .grove_get_proved_path_query(
                    &query.construct_path_query(version).unwrap(),
                    None,
                    &mut vec![],
                    &version.drive,
                )
                .unwrap();
            proof.grovedb_proof = drive::util::test_helpers::history_proof::downgrade_history_count(
                &bytes,
                Some(3),
                version,
            );
            let error = DocumentHistory::maybe_from_proof(
                request.clone(),
                response.clone(),
                Network::Testnet,
                version,
                &provider,
            )
            .expect_err("document history requires GroveDB v1 even with a valid quorum signature");
            assert!(matches!(
                error,
                drive_proof_verifier::Error::UnsupportedGroveDBProofVersion { .. }
            ));
            assert!(drive_proof_verifier::types::DocumentHistoryProofInfo::maybe_from_proof_with_metadata(
                request, response, Network::Testnet, version, &provider,
            ).is_err());
        }
        return;
    }
    let selections = [
        (
            document.id().to_buffer(),
            Filter::StartAtMs(2000),
            DocumentHistoryFilter::StartAtTime(2000),
        ),
        (
            document.id().to_buffer(),
            Filter::StartAfter(Cursor {
                time_ms: 2000,
                revision: 1,
            }),
            DocumentHistoryFilter::StartAfter {
                time_ms: 2000,
                revision: 1,
            },
        ),
        (
            document.id().to_buffer(),
            Filter::Revision(2),
            DocumentHistoryFilter::Revision(2),
        ),
        (
            document.id().to_buffer(),
            Filter::StartAtRevision(65535),
            DocumentHistoryFilter::StartAtRevision(65535),
        ),
        (
            document.id().to_buffer(),
            Filter::StartAtRevision(65536),
            DocumentHistoryFilter::StartAtRevision(65536),
        ),
        (
            [255; 32],
            Filter::Revision(2),
            DocumentHistoryFilter::Revision(2),
        ),
    ];
    for (id, wire_selector, filter) in selections {
        let request = GetDocumentHistoryRequestV1 {
            data_contract_id: contract.id().to_vec(),
            document_type_name: "profile".into(),
            document_id: id.to_vec(),
            limit: None,
            prove: true,
            filter: Some(wire_selector),
        };
        let request_v0 = request.clone();
        let response = platform
            .query_document_history_v1(request.clone(), &state, version)
            .unwrap()
            .into_data()
            .unwrap();
        let query = DocumentHistoryDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "profile".into(),
            document_id: id,
            limit: None,
            filter,
        };
        let expected = platform
            .drive
            .fetch_document_history(&query, document_type, None, version)
            .unwrap();
        let request: GetDocumentHistoryRequest = request.into();
        let verify = |response: GetDocumentHistoryResponseV1| {
            DocumentHistory::maybe_from_proof(
                request.clone(),
                GetDocumentHistoryResponse::from(response),
                Network::Testnet,
                version,
                &provider,
            )
        };
        let (exported, _, _) =
            drive_proof_verifier::types::DocumentHistoryProofInfo::maybe_from_proof_with_metadata(
                request.clone(),
                GetDocumentHistoryResponse::from(response.clone()),
                Network::Testnet,
                version,
                &provider,
            )
            .unwrap();
        let exported = exported.unwrap();
        assert_eq!(
            exported.response,
            GetDocumentHistoryResponse::from(response.clone())
        );
        assert_eq!(
            exported
                .verify(request.clone(), Network::Testnet, version, &provider)
                .unwrap(),
            Some(exported.history)
        );
        if std::env::var_os("UPDATE_DOCUMENT_HISTORY_VECTORS").is_some() {
            let name = match query.filter {
                DocumentHistoryFilter::Revision(2) if id == document.id().to_buffer() => {
                    Some("pv15_revision.json")
                }
                DocumentHistoryFilter::StartAfter {
                    time_ms: 2000,
                    revision: 1,
                } => Some("pv15_cursor.json"),
                _ => None,
            };
            if let Some(name) = name {
                let vector = serde_json::json!({
                    "contract": hex::encode(contract.serialize_to_bytes_with_platform_version(version).unwrap()),
                    "quorum_public_key": hex::encode(provider.key),
                    "request": request,
                    "response": GetDocumentHistoryResponse::from(response.clone()),
                });
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../rs-sdk/tests/vectors/document_history")
                    .join(name);
                std::fs::write(
                    path,
                    format!("{}\n", serde_json::to_string(&vector).unwrap()),
                )
                .unwrap();
            }
        }
        let result = verify(response.clone()).unwrap().unwrap();
        assert_eq!(result.entries, expected.entries);
        assert_eq!(result.lifecycle, expected.lifecycle.clone());
        // Without a proof the same query answers with the history itself.
        let mut plain_request = request_v0.clone();
        plain_request.prove = false;
        let plain = platform
            .query_document_history_v1(plain_request, &state, version)
            .unwrap()
            .into_data()
            .unwrap();
        let Some(ResponseResult::History(history)) = plain.result else {
            panic!("an unproved history query answers with the history");
        };
        assert_eq!(
            history
                .entries
                .iter()
                .map(|entry| (entry.time_ms, entry.revision))
                .collect::<Vec<_>>(),
            expected
                .entries
                .iter()
                .map(|entry| (entry.time_ms, entry.revision))
                .collect::<Vec<_>>()
        );
        let lifecycle = history
            .lifecycle
            .expect("the history carries its lifecycle");
        let expected_lifecycle = expected
            .lifecycle
            .as_ref()
            .expect("pages of the history tree carry a lifecycle");
        assert_eq!(
            lifecycle.remaining_revisions,
            expected_lifecycle.remaining_revisions
        );
        assert_eq!(
            lifecycle.state,
            match expected_lifecycle.state {
                DocumentHistoryState::Active => State::Active,
                DocumentHistoryState::Absent => State::Absent,
                DocumentHistoryState::Deleted => State::Deleted,
                DocumentHistoryState::Erasing => State::Erasing,
            } as i32
        );
        let mut legacy = response.clone();
        let proof = proof_mut(&mut legacy);
        proof.grovedb_proof = drive::util::test_helpers::history_proof::downgrade_history_count(
            &proof.grovedb_proof,
            None,
            version,
        );
        let error = verify(legacy)
            .expect_err("count-offset history pages require a GroveDB v1 proof envelope");
        assert!(matches!(
            error,
            drive_proof_verifier::Error::UnsupportedGroveDBProofVersion { .. }
        ));
        let mut torn = response.clone();
        proof_mut(&mut torn).grovedb_proof.push(0);
        assert!(
            verify(torn).is_err(),
            "a proof envelope with trailing bytes is not the proof the node produced"
        );
        let mut bad_signature = response.clone();
        proof_mut(&mut bad_signature).signature[0] ^= 1;
        assert!(verify(bad_signature).is_err());
        let mut missing = response.clone();
        proof_mut(&mut missing).grovedb_proof.clear();
        assert!(verify(missing).is_err());
    }
}

#[test]
fn should_reject_missing_selectors_before_reading_state() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let request = GetDocumentHistoryRequestV1 {
        data_contract_id: vec![1; 32],
        document_type_name: "note".into(),
        document_id: vec![2; 32],
        filter: None,
        limit: None,
        prove: false,
    };
    assert!(!platform
        .query_document_history_v1(request, &state, version)
        .unwrap()
        .is_valid());
}

/// A contract stored as its schemas is parsed each time it is read from state, every pattern
/// it holds compiled, so a history query reads it through the contract cache.
#[test]
fn should_read_the_contract_through_the_contract_cache() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let contract = json_document_to_contract(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json"
        ),
        false,
        version,
    )
    .unwrap();
    platform
        .drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let id = contract.id().to_buffer();
    // What stored the contract cached it
    let cache = &platform.drive.cache.data_contracts;
    cache.clear();

    let query = || {
        let request = GetDocumentHistoryRequestV1 {
            data_contract_id: id.to_vec(),
            document_type_name: "profile".into(),
            document_id: vec![2; 32],
            filter: Some(Filter::StartAtRevision(1)),
            limit: None,
            prove: false,
        };
        // The document does not exist; what matters is how the contract was read
        platform
            .query_document_history_v1(request, &state, version)
            .expect("expected the query to run");
        cache.get(id, false).expect("the query caches the contract")
    };

    let read_from_state = query();
    let read_again = query();
    assert!(
        Arc::ptr_eq(&read_from_state, &read_again),
        "the second query is answered with the cached contract"
    );
}

#[test]
fn should_derive_deleted_and_erasing_lifecycle_from_the_proof() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let mut state = state.as_ref().clone();
    let contract = json_document_to_contract(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json"
        ),
        false,
        version,
    )
    .unwrap();
    platform
        .drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = contract.document_type_for_name("profile").unwrap();
    let owner = Identifier::from([8; 32]);
    let mut document = json_document_to_document(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rs-drive/tests/supporting_files/contract/dashpay/profile0.json"
        ),
        Some(owner),
        document_type,
        version,
    )
    .unwrap();
    let chunk = version
        .system_limits
        .max_document_revisions_erased_per_transition
        .expect("protocol 15 bounds the erase chunk") as u64;
    for revision in 1..=chunk + 2 {
        document.set_revision(Some(revision));
        platform
            .drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                revision > 1,
                BlockInfo::default_with_time(1_000 + revision),
                true,
                None,
                version,
            )
            .unwrap();
    }

    let key = SecretKey::from_be_bytes(&[1; 32]).unwrap();
    let provider = Provider {
        contract: Arc::new(contract.clone()),
        key: key.public_key().to_bytes().try_into().unwrap(),
    };
    state.last_committed_block_info = Some(
        dpp::block::extended_block_info::v0::ExtendedBlockInfoV0 {
            basic_info: BlockInfo {
                height: 42,
                core_height: 12,
                time_ms: 3_000,
                epoch: Default::default(),
            },
            app_hash: [0; 32],
            quorum_hash: [9; 32],
            block_id_hash: [7; 32],
            proposer_pro_tx_hash: [0; 32],
            signature: [0; 96],
            round: 0,
        }
        .into(),
    );
    let wire_request = GetDocumentHistoryRequestV1 {
        data_contract_id: contract.id().to_vec(),
        document_type_name: "profile".into(),
        document_id: document.id().to_vec(),
        limit: None,
        prove: true,
        filter: Some(Filter::StartAtMs(0)),
    };
    let request: GetDocumentHistoryRequest = wire_request.clone().into();

    for (stage, expected_state) in [
        ("deleted", DocumentHistoryState::Deleted),
        ("erasing", DocumentHistoryState::Erasing),
    ] {
        let operations = if stage == "deleted" {
            platform
                .drive
                .delete_document_for_contract_operations(
                    document.id(),
                    &contract,
                    document_type,
                    &BlockInfo::default_with_time(5_000),
                    Some(owner),
                    None,
                    &mut None,
                    None,
                    version,
                )
                .unwrap()
        } else {
            platform
                .drive
                .erase_document_for_contract_operations(
                    document.id(),
                    &contract,
                    document_type,
                    &BlockInfo::default_with_time(6_000),
                    &mut None,
                    None,
                    version,
                )
                .unwrap()
        };
        platform
            .drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &version.drive,
            )
            .unwrap();

        let root = platform
            .drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .value
            .unwrap();
        let metadata = platform.response_metadata_v0(&state, CheckpointUsed::Current);
        let signed = signed_proof(
            vec![],
            root,
            &metadata,
            &key,
            platform.config.validator_set.quorum_type as u32,
        );
        let committed = state.last_committed_block_info.as_mut().unwrap();
        committed.set_app_hash(root);
        committed.set_signature(signed.signature.try_into().unwrap());

        let response = platform
            .query_document_history_v1(wire_request.clone(), &state, version)
            .unwrap()
            .into_data()
            .unwrap();
        // A proved response carries the proof and nothing else: there are no
        // lifecycle fields on the wire for a client to trust, so every value
        // the verifier returns below is derived from the proof itself.
        assert!(
            matches!(response.result, Some(ResponseResult::Proof(_))),
            "{stage}: a proved response must not carry lifecycle claims"
        );
        let verify = |response: GetDocumentHistoryResponseV1| {
            DocumentHistory::maybe_from_proof(
                request.clone(),
                GetDocumentHistoryResponse::from(response),
                Network::Testnet,
                version,
                &provider,
            )
        };
        let history = verify(response.clone())
            .unwrap_or_else(|error| panic!("{stage}: honest response must verify: {error}"))
            .expect("the proof authenticates a history");
        let lifecycle = history
            .lifecycle
            .expect("the proof authenticates the lifecycle");
        assert_eq!(lifecycle.state, expected_state, "{stage}");
        assert_eq!(lifecycle.times.deleted_at_ms, 5_000, "{stage}");

        assert_eq!(
            lifecycle.remaining_revisions,
            if stage == "deleted" { chunk + 2 } else { 2 }
        );
        assert_eq!(
            lifecycle.times.erasing_started_at_ms,
            if stage == "erasing" { 6_000 } else { 0 }
        );
        assert_eq!(
            lifecycle.times.erasing_from_time_ms,
            if stage == "erasing" {
                1_000 + chunk + 2
            } else {
                0
            }
        );
        assert_eq!(
            lifecycle.times.erasing_from_revision,
            if stage == "erasing" { chunk + 2 } else { 0 }
        );
        let mut plain_request = wire_request.clone();
        plain_request.prove = false;
        let plain = platform
            .query_document_history_v1(plain_request, &state, version)
            .unwrap()
            .into_data()
            .unwrap();
        let Some(ResponseResult::History(plain)) = plain.result else {
            panic!("an unproved response carries history");
        };
        let plain = plain.lifecycle.unwrap();
        assert_eq!(plain.remaining_revisions, lifecycle.remaining_revisions);
        assert_eq!(plain.deleted_at_ms, lifecycle.times.deleted_at_ms);
        assert_eq!(
            plain.erasing_started_at_ms,
            lifecycle.times.erasing_started_at_ms
        );
        assert_eq!(
            plain.erasing_from_time_ms,
            lifecycle.times.erasing_from_time_ms
        );
        assert_eq!(
            plain.erasing_from_revision,
            lifecycle.times.erasing_from_revision
        );

        let mut tampered = response;
        let proof = proof_mut(&mut tampered);
        let middle = proof.grovedb_proof.len() / 2;
        proof.grovedb_proof[middle] ^= 1;
        assert!(
            verify(tampered).is_err(),
            "{stage}: changing the proof that authenticates lifecycle must be rejected"
        );
    }
}

#[test]
fn should_surface_oversized_pv15_history_proof_as_unusable_verified_result_when_signed() {
    use dapi_grpc::platform::v0::get_document_history_request::Version as RequestVersion;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::document::{Document, DocumentV0};
    use dpp::platform_value::{platform_value, Value};
    use std::collections::BTreeMap;

    let pv14 = PlatformVersion::get(14).expect("PV14 exists");
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let mut state = state.as_ref().clone();

    let field_count = 60usize;
    let mut properties = Vec::with_capacity(field_count);
    let mut doc_properties = BTreeMap::new();
    for i in 0..field_count {
        let name = format!("f{i:02}");
        properties.push((
            Value::Text(name.clone()),
            platform_value!({
                "type": "array",
                "maxItems": 1024,
                "items": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 0,
                    "maxItems": 0,
                },
                "position": i as u32,
            }),
        ));
        doc_properties.insert(name, Value::Array(vec![Value::Bytes(vec![]); 1024]));
    }
    let contract = DataContract::from_value(
        platform_value!({
            "$formatVersion": "1",
            "id": Identifier::new([0x71; 32]),
            "version": 1u32,
            "ownerId": Identifier::new([0x72; 32]),
            "config": {
                "$formatVersion": "1",
                "canBeDeleted": false,
                "readonly": false,
                "keepsHistory": false,
                "documentsKeepHistoryContractDefault": true,
                "documentsMutableContractDefault": true,
                "documentsCanBeDeletedContractDefault": false,
                "requiresIdentityEncryptionBoundedKey": Value::Null,
                "requiresIdentityDecryptionBoundedKey": Value::Null,
                "sizedIntegerTypes": true,
            },
            "documentSchemas": {
                "note": {
                    "type": "object",
                    "documentsKeepHistory": true,
                    "documentsMutable": true,
                    "canBeDeleted": false,
                    "properties": Value::Map(properties),
                    "additionalProperties": false,
                }
            }
        }),
        true,
        pv14,
    )
    .expect("PV14 history contract validates");

    platform
        .drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .expect("apply contract");

    let document_type = contract.document_type_for_name("note").expect("note type");
    let owner_id = Identifier::new([0x73; 32]);
    let doc_id = Identifier::new([0x74; 32]);
    let mut document = Document::V0(DocumentV0 {
        id: doc_id,
        owner_id,
        properties: doc_properties,
        revision: Some(1),
        ..Default::default()
    });
    platform
        .drive
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
            BlockInfo::default_with_time(1000),
            true,
            None,
            version,
        )
        .expect("insert revision 1");
    for revision in 2u64..=5 {
        document.set_revision(Some(revision));
        platform
            .drive
            .update_document_for_contract(
                &document,
                &contract,
                document_type,
                Some(owner_id.to_buffer()),
                BlockInfo::default_with_time(1000 * revision),
                true,
                None,
                None,
                version,
            )
            .expect("update revision");
    }

    let key = SecretKey::from_be_bytes(&[1; 32]).unwrap();
    let provider = Provider {
        contract: Arc::new(contract.clone()),
        key: key.public_key().to_bytes(),
    };
    state.last_committed_block_info = Some(
        ExtendedBlockInfoV0 {
            basic_info: BlockInfo {
                height: 42,
                core_height: 12,
                time_ms: 6000,
                epoch: Default::default(),
            },
            app_hash: [0; 32],
            quorum_hash: [9; 32],
            block_id_hash: [7; 32],
            proposer_pro_tx_hash: [0; 32],
            signature: [0; 96],
            round: 0,
        }
        .into(),
    );
    let metadata = platform.response_metadata_v0(&state, CheckpointUsed::Current);
    let root = platform
        .drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .value
        .unwrap();

    let request = GetDocumentHistoryRequestV1 {
        data_contract_id: contract.id().to_vec(),
        document_type_name: "note".to_string(),
        document_id: doc_id.to_vec(),
        limit: Some(5),
        prove: true,
        filter: Some(Filter::StartAtMs(0)),
    };

    // Unproved response serves stored bytes directly without decoding properties.
    let mut unproved_request = request.clone();
    unproved_request.prove = false;
    let unproved = platform
        .query_document_history_v1(unproved_request, &state, version)
        .expect("unproved query succeeds")
        .into_data()
        .expect("unproved data");
    let Some(ResponseResult::History(unproved_history)) = unproved.result else {
        panic!("expected history result");
    };
    assert_eq!(unproved_history.entries.len(), 5);

    // Proved response generates without decoding properties; under a valid quorum signature
    // the client decode-budget refusal maps to `UnusableVerifiedResult` so healthy nodes are
    // not banned, while a bad signature remains `InvalidSignature`.
    let mut proved = platform
        .query_document_history_v1(request.clone(), &state, version)
        .expect("proved query succeeds")
        .into_data()
        .expect("proved data");
    let grovedb_proof = proof_mut(&mut proved).grovedb_proof.clone();
    let quorum_type = platform.config.validator_set.quorum_type as u32;
    let signed = signed_proof(grovedb_proof, root, &metadata, &key, quorum_type);
    *proof_mut(&mut proved) = signed.clone();
    proved.metadata = Some(metadata.clone());

    let outer_request = GetDocumentHistoryRequest {
        version: Some(RequestVersion::V1(request)),
    };
    let signed_response = GetDocumentHistoryResponse {
        version: Some(ResponseVersion::V1(proved.clone())),
    };
    let error = DocumentHistory::maybe_from_proof_with_metadata(
        outer_request.clone(),
        signed_response,
        Network::Testnet,
        version,
        &provider,
    )
    .expect_err("five-revision page exceeds client decoded-value budget");
    assert!(
        matches!(
            error,
            drive_proof_verifier::Error::UnusableVerifiedResult { ref reason }
                if reason.contains("decoded value budget exceeded")
        ),
        "valid signed proof over oversized page must surface as UnusableVerifiedResult, got: {error:?}"
    );

    let mut wrong_root = root;
    wrong_root[0] ^= 1;
    let mut bad_sig_proved = proved;
    let grovedb_proof = proof_mut(&mut bad_sig_proved).grovedb_proof.clone();
    *proof_mut(&mut bad_sig_proved) =
        signed_proof(grovedb_proof, wrong_root, &metadata, &key, quorum_type);
    let bad_sig_response = GetDocumentHistoryResponse {
        version: Some(ResponseVersion::V1(bad_sig_proved)),
    };
    let bad_sig_error = DocumentHistory::maybe_from_proof_with_metadata(
        outer_request,
        bad_sig_response,
        Network::Testnet,
        version,
        &provider,
    )
    .expect_err("signature over wrong root must fail as InvalidSignature");
    assert!(
        matches!(
            bad_sig_error,
            drive_proof_verifier::Error::InvalidSignature { .. }
        ),
        "forged signature must stay a node fault, got: {bad_sig_error:?}"
    );
}

#[test]
fn should_surface_unreadable_migrated_revision_as_unusable_verified_result_and_preserve_raw_bytes()
{
    use dapi_grpc::platform::v0::get_document_history_request::Version as RequestVersion;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::{Document, DocumentV0};
    use dpp::platform_value::{platform_value, Value};
    use drive::drive::document::paths::DOCUMENT_HISTORY_TREE_KEY;
    use drive::drive::RootTree;
    use drive::grovedb::Element;
    use drive::util::common::encode::encode_u64;
    use std::collections::BTreeMap;

    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let mut state = state.as_ref().clone();

    let contract = DataContract::from_value(
        platform_value!({
            "$formatVersion": "1",
            "id": Identifier::new([0x81; 32]),
            "version": 1u32,
            "ownerId": Identifier::new([0x82; 32]),
            "config": {
                "$formatVersion": "1",
                "canBeDeleted": false,
                "readonly": false,
                "keepsHistory": false,
                "documentsKeepHistoryContractDefault": true,
                "documentsMutableContractDefault": true,
                "documentsCanBeDeletedContractDefault": false,
                "requiresIdentityEncryptionBoundedKey": Value::Null,
                "requiresIdentityDecryptionBoundedKey": Value::Null,
                "sizedIntegerTypes": true,
            },
            "documentSchemas": {
                "note": {
                    "type": "object",
                    "documentsKeepHistory": true,
                    "documentsMutable": true,
                    "canBeDeleted": false,
                    "properties": {
                        "body": { "type": "string", "maxLength": 64, "position": 0 }
                    },
                    "required": ["body"],
                    "additionalProperties": false,
                }
            }
        }),
        true,
        version,
    )
    .expect("contract validates");

    platform
        .drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .expect("apply contract");

    let document_type = contract.document_type_for_name("note").expect("note type");
    let owner_id = Identifier::new([0x83; 32]);
    let doc_id = Identifier::new([0x84; 32]);
    let mut document = Document::V0(DocumentV0 {
        id: doc_id,
        owner_id,
        properties: BTreeMap::from([("body".to_string(), Value::Text("hello".to_string()))]),
        revision: Some(1),
        ..Default::default()
    });
    platform
        .drive
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
            BlockInfo::default_with_time(1000),
            true,
            None,
            version,
        )
        .expect("insert revision 1");

    // Advance the current pointer to revision 2 so the primary tree reference points to
    // revision 2 while revision 1 remains a retained historical entry.
    document.set_revision(Some(2));
    document.set("body", Value::Text("updated".to_string()));
    platform
        .drive
        .update_document_for_contract(
            &document,
            &contract,
            document_type,
            Some(owner_id.to_buffer()),
            BlockInfo::default_with_time(2000),
            true,
            None,
            None,
            version,
        )
        .expect("update to revision 2");

    // Replace the stored historical revision 1 bytes with a valid format-3 header for revision 1
    // followed by bytes the current schema cannot decode (modeling a migrated pre-update
    // revision whose header still matches).
    let mut unreadable_revision_bytes = Vec::with_capacity(69);
    unreadable_revision_bytes.push(3u8); // format 3
    unreadable_revision_bytes.push(0u8); // contract version stamp 0
    unreadable_revision_bytes.extend_from_slice(doc_id.as_bytes());
    unreadable_revision_bytes.extend_from_slice(owner_id.as_bytes());
    unreadable_revision_bytes.push(1u8); // revision 1
    unreadable_revision_bytes.extend_from_slice(&[0xff, 0xff]); // truncated/unreadable body
    assert!(
        Document::from_bytes(&unreadable_revision_bytes, document_type, version).is_err(),
        "body bytes must be unreadable under the current schema"
    );

    let contract_id = contract.id();
    let history_path: [&[u8]; 6] = [
        &[RootTree::DataContractDocuments as u8],
        contract_id.as_slice(),
        &[1],
        b"note",
        &[DOCUMENT_HISTORY_TREE_KEY],
        doc_id.as_slice(),
    ];
    let mut rev_key = encode_u64(1000);
    rev_key.extend(encode_u64(1));
    platform
        .drive
        .grove
        .insert(
            &history_path,
            &rev_key,
            Element::new_item(unreadable_revision_bytes.clone()),
            None,
            None,
            &version.drive.grove_version,
        )
        .unwrap()
        .expect("overwrite stored revision bytes");

    let drive_query = DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "note".to_string(),
        document_id: doc_id.to_buffer(),
        filter: DocumentHistoryFilter::StartAtTime(0),
        limit: Some(5),
    };
    // Direct Drive serialized fetch validates only headers and returns the exact stored bytes.
    let serialized = platform
        .drive
        .fetch_document_history_serialized(&drive_query, document_type, None, version)
        .expect("serialized fetch must not decode revision bodies");
    assert_eq!(serialized.entries.len(), 2);
    assert_eq!(
        serialized.entries[0].document_bytes,
        unreadable_revision_bytes
    );

    let key = SecretKey::from_be_bytes(&[2; 32]).unwrap();
    let provider = Provider {
        contract: Arc::new(contract.clone()),
        key: key.public_key().to_bytes(),
    };
    state.last_committed_block_info = Some(
        ExtendedBlockInfoV0 {
            basic_info: BlockInfo {
                height: 43,
                core_height: 12,
                time_ms: 2000,
                epoch: Default::default(),
            },
            app_hash: [0; 32],
            quorum_hash: [9; 32],
            block_id_hash: [7; 32],
            proposer_pro_tx_hash: [0; 32],
            signature: [0; 96],
            round: 0,
        }
        .into(),
    );
    let metadata = platform.response_metadata_v0(&state, CheckpointUsed::Current);
    let root = platform
        .drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .value
        .unwrap();

    let request = GetDocumentHistoryRequestV1 {
        data_contract_id: contract.id().to_vec(),
        document_type_name: "note".to_string(),
        document_id: doc_id.to_vec(),
        limit: Some(5),
        prove: false,
        filter: Some(Filter::StartAtMs(0)),
    };
    // Unproved response returns the exact stored bytes verbatim without decoding properties.
    let unproved = platform
        .query_document_history_v1(request.clone(), &state, version)
        .expect("unproved query succeeds without decoding bodies")
        .into_data()
        .expect("unproved data");
    let Some(ResponseResult::History(unproved_history)) = unproved.result else {
        panic!("expected history result");
    };
    assert_eq!(unproved_history.entries.len(), 2);
    assert_eq!(
        unproved_history.entries[0].document,
        unreadable_revision_bytes
    );

    // Proved response authenticates the root and headers first, then surfaces the local body
    // decode failure as `UnusableVerifiedResult` so healthy responders are never banned.
    let mut proved_request = request;
    proved_request.prove = true;
    let mut proved = platform
        .query_document_history_v1(proved_request.clone(), &state, version)
        .expect("proved query succeeds without decoding bodies")
        .into_data()
        .expect("proved data");
    let grovedb_proof = proof_mut(&mut proved).grovedb_proof.clone();
    let quorum_type = platform.config.validator_set.quorum_type as u32;
    *proof_mut(&mut proved) =
        signed_proof(grovedb_proof.clone(), root, &metadata, &key, quorum_type);
    proved.metadata = Some(metadata.clone());

    let outer_request = GetDocumentHistoryRequest {
        version: Some(RequestVersion::V1(proved_request)),
    };
    let signed_response = GetDocumentHistoryResponse {
        version: Some(ResponseVersion::V1(proved.clone())),
    };
    let error = DocumentHistory::maybe_from_proof_with_metadata(
        outer_request.clone(),
        signed_response,
        Network::Testnet,
        version,
        &provider,
    )
    .expect_err("unreadable revision body must be refused");
    assert!(
        matches!(error, drive_proof_verifier::Error::UnusableVerifiedResult { .. }),
        "signed proof over unreadable migrated revision must be UnusableVerifiedResult, got: {error:?}"
    );

    // A signature over a different root must still fail as `InvalidSignature` (node fault).
    let mut wrong_root = root;
    wrong_root[0] ^= 1;
    let mut bad_sig_proved = proved;
    *proof_mut(&mut bad_sig_proved) =
        signed_proof(grovedb_proof, wrong_root, &metadata, &key, quorum_type);
    let bad_sig_error = DocumentHistory::maybe_from_proof_with_metadata(
        outer_request,
        GetDocumentHistoryResponse {
            version: Some(ResponseVersion::V1(bad_sig_proved)),
        },
        Network::Testnet,
        version,
        &provider,
    )
    .expect_err("wrong-root signature must fail as InvalidSignature");
    assert!(
        matches!(
            bad_sig_error,
            drive_proof_verifier::Error::InvalidSignature { .. }
        ),
        "forged signature must stay a node fault, got: {bad_sig_error:?}"
    );
}

#[test]
fn should_surface_oversized_cursor_document_as_unusable_verified_result_when_signed() {
    use dapi_grpc::platform::v0::{get_documents_response, GetDocumentsResponse};
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::document::{Document, DocumentV0};
    use dpp::platform_value::{platform_value, Value};
    use drive::error::proof::ProofError;
    use drive::query::{
        DriveDocumentQuery, InternalClauses, OrderClause, WhereClause, WhereOperator,
    };
    use drive_proof_verifier::types::Documents;
    use indexmap::IndexMap;
    use std::collections::BTreeMap;

    let pv14 = PlatformVersion::get(14).expect("PV14 exists");
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let mut state = state.as_ref().clone();

    // 4 nested objects × 65 zero-width typed arrays = 266,504 decoded values > 262,144.
    let groups = 4usize;
    let fields_per_group = 65usize;
    let mut properties = vec![
        (
            Value::Text("category".to_string()),
            platform_value!({ "type": "string", "maxLength": 16, "position": 0 }),
        ),
        (
            Value::Text("rank".to_string()),
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 1000, "position": 1 }),
        ),
    ];
    let mut oversized_props = BTreeMap::from([
        ("category".to_string(), Value::Text("target".to_string())),
        ("rank".to_string(), Value::U16(1)),
    ]);
    for g in 0..groups {
        let mut inner_schema = Vec::with_capacity(fields_per_group);
        let mut inner_doc = Vec::with_capacity(fields_per_group);
        for i in 0..fields_per_group {
            let name = format!("f{i:02}");
            inner_schema.push((
                Value::Text(name.clone()),
                platform_value!({
                    "type": "array",
                    "maxItems": 1024,
                    "items": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 0,
                        "maxItems": 0,
                    },
                    "position": i as u32,
                }),
            ));
            inner_doc.push((
                Value::Text(name),
                Value::Array(vec![Value::Bytes(vec![]); 1024]),
            ));
        }
        let gname = format!("g{g}");
        properties.push((
            Value::Text(gname.clone()),
            platform_value!({
                "type": "object",
                "properties": Value::Map(inner_schema),
                "additionalProperties": false,
                "position": (g + 2) as u32,
            }),
        ));
        oversized_props.insert(gname, Value::Map(inner_doc));
    }

    let contract = DataContract::from_value(
        platform_value!({
            "$formatVersion": "1",
            "id": Identifier::new([0x91; 32]),
            "version": 1u32,
            "ownerId": Identifier::new([0x92; 32]),
            "config": {
                "$formatVersion": "1",
                "canBeDeleted": false,
                "readonly": false,
                "keepsHistory": false,
                "documentsKeepHistoryContractDefault": false,
                "documentsMutableContractDefault": true,
                "documentsCanBeDeletedContractDefault": true,
                "requiresIdentityEncryptionBoundedKey": Value::Null,
                "requiresIdentityDecryptionBoundedKey": Value::Null,
                "sizedIntegerTypes": true,
            },
            "documentSchemas": {
                "item": {
                    "type": "object",
                    "indices": [
                        {
                            "name": "byCategoryRank",
                            "properties": [
                                { "category": "asc" },
                                { "rank": "asc" }
                            ]
                        }
                    ],
                    "properties": Value::Map(properties),
                    "required": ["category", "rank"],
                    "additionalProperties": false,
                }
            }
        }),
        true,
        pv14,
    )
    .expect("PV14 contract validates");
    platform
        .drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .expect("apply contract");

    let document_type = contract.document_type_for_name("item").expect("item type");
    let owner_id = Identifier::new([0x93; 32]);
    let cursor_doc = Document::V0(DocumentV0 {
        id: Identifier::new([0x94; 32]),
        owner_id,
        properties: oversized_props,
        ..Default::default()
    });
    let small_doc = Document::V0(DocumentV0 {
        id: Identifier::new([0x95; 32]),
        owner_id,
        properties: BTreeMap::from([
            ("category".to_string(), Value::Text("target".to_string())),
            ("rank".to_string(), Value::U16(2)),
        ]),
        ..Default::default()
    });
    for doc in [&cursor_doc, &small_doc] {
        platform
            .drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((doc, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                version,
            )
            .expect("insert document");
    }

    let query = DriveDocumentQuery {
        contract: &contract,
        document_type,
        internal_clauses: InternalClauses {
            equal_clauses: BTreeMap::from([(
                "category".to_string(),
                WhereClause {
                    field: "category".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("target".to_string()),
                },
            )]),
            ..Default::default()
        },
        offset: None,
        limit: Some(10),
        order_by: IndexMap::from([(
            "rank".to_string(),
            OrderClause {
                field: "rank".to_string(),
                ascending: true,
            },
        )]),
        start_at: Some(cursor_doc.id().to_buffer()),
        start_at_included: false,
        block_time_ms: None,
        resolved_time_ranges: Vec::new(),
        sub_queries: Vec::new(),
    };
    let (grovedb_proof, _) = query
        .clone()
        .execute_with_proof(&platform.drive, None, None, version)
        .expect("page proof executes");

    let root = platform
        .drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .value
        .unwrap();

    // Direct Drive verification preserves the cursor root on budget exhaustion.
    let direct_err = query
        .verify_proof(&grovedb_proof, version)
        .expect_err("oversized cursor must exceed decode budget");
    assert!(
        matches!(
            direct_err,
            drive::error::Error::Proof(ProofError::DecodedValueBudgetExceeded { root_hash, .. })
                if root_hash == root
        ),
        "expected ProofError::DecodedValueBudgetExceeded with matching root, got: {direct_err:?}"
    );

    let key = SecretKey::from_be_bytes(&[3; 32]).unwrap();
    let provider = Provider {
        contract: Arc::new(contract.clone()),
        key: key.public_key().to_bytes(),
    };
    state.last_committed_block_info = Some(
        ExtendedBlockInfoV0 {
            basic_info: BlockInfo {
                height: 44,
                core_height: 12,
                time_ms: 3000,
                epoch: Default::default(),
            },
            app_hash: [0; 32],
            quorum_hash: [9; 32],
            block_id_hash: [7; 32],
            proposer_pro_tx_hash: [0; 32],
            signature: [0; 96],
            round: 0,
        }
        .into(),
    );
    let metadata = platform.response_metadata_v0(&state, CheckpointUsed::Current);
    let quorum_type = platform.config.validator_set.quorum_type as u32;
    let signed = signed_proof(grovedb_proof.clone(), root, &metadata, &key, quorum_type);
    let signed_response = GetDocumentsResponse {
        version: Some(get_documents_response::Version::V0(
            get_documents_response::GetDocumentsResponseV0 {
                result: Some(
                    get_documents_response::get_documents_response_v0::Result::Proof(signed),
                ),
                metadata: Some(metadata.clone()),
            },
        )),
    };
    let verified_err =
        <Documents as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
            query.clone(),
            signed_response,
            Network::Testnet,
            version,
            &provider,
        )
        .expect_err("signed oversized cursor must surface as UnusableVerifiedResult");
    assert!(
        matches!(
            verified_err,
            drive_proof_verifier::Error::UnusableVerifiedResult { ref reason }
                if reason.contains("decoded value budget exceeded")
        ),
        "expected UnusableVerifiedResult, got: {verified_err:?}"
    );

    let mut wrong_root = root;
    wrong_root[0] ^= 1;
    let bad_sig = signed_proof(grovedb_proof, wrong_root, &metadata, &key, quorum_type);
    let bad_sig_response = GetDocumentsResponse {
        version: Some(get_documents_response::Version::V0(
            get_documents_response::GetDocumentsResponseV0 {
                result: Some(
                    get_documents_response::get_documents_response_v0::Result::Proof(bad_sig),
                ),
                metadata: Some(metadata),
            },
        )),
    };
    let bad_sig_err = <Documents as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
        query,
        bad_sig_response,
        Network::Testnet,
        version,
        &provider,
    )
    .expect_err("wrong-root signature must fail as InvalidSignature");
    assert!(
        matches!(
            bad_sig_err,
            drive_proof_verifier::Error::InvalidSignature { .. }
        ),
        "forged signature must stay a node fault, got: {bad_sig_err:?}"
    );
}

#[test]
fn should_surface_oversized_merged_transfer_proof_as_unusable_verified_result_when_signed() {
    use dapi_grpc::platform::v0::{
        wait_for_state_transition_result_response, BroadcastStateTransitionRequest,
        WaitForStateTransitionResultResponse,
    };
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::document::{Document, DocumentV0};
    use dpp::identity::{Identity, IdentityV0};
    use dpp::platform_value::{platform_value, Value};
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::batched_transition::document_base_transition::{
        v0::DocumentBaseTransitionV0, DocumentBaseTransition,
    };
    use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::{
        DocumentTransferTransition, DocumentTransferTransitionV0,
    };
    use dpp::state_transition::batch_transition::batched_transition::{
        BatchedTransition, DocumentTransition,
    };
    use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
    use dpp::state_transition::proof_result::StateTransitionProofOutcome;
    use dpp::state_transition::StateTransition;
    use drive::error::proof::ProofError;
    use std::collections::BTreeMap;

    let pv14 = PlatformVersion::get(14).expect("PV14 exists");
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let mut state = state.as_ref().clone();

    // 4 nested objects × 65 zero-width typed arrays = 266,504 decoded values > 262,144.
    let groups = 4usize;
    let fields_per_group = 65usize;
    let mut properties = Vec::with_capacity(groups);
    let mut oversized_props = BTreeMap::new();
    for g in 0..groups {
        let mut inner_schema = Vec::with_capacity(fields_per_group);
        let mut inner_doc = Vec::with_capacity(fields_per_group);
        for i in 0..fields_per_group {
            let name = format!("f{i:02}");
            inner_schema.push((
                Value::Text(name.clone()),
                platform_value!({
                    "type": "array",
                    "maxItems": 1024,
                    "items": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 0,
                        "maxItems": 0,
                    },
                    "position": i as u32,
                }),
            ));
            inner_doc.push((
                Value::Text(name),
                Value::Array(vec![Value::Bytes(vec![]); 1024]),
            ));
        }
        let gname = format!("g{g}");
        properties.push((
            Value::Text(gname.clone()),
            platform_value!({
                "type": "object",
                "properties": Value::Map(inner_schema),
                "additionalProperties": false,
                "position": g as u32,
            }),
        ));
        oversized_props.insert(gname, Value::Map(inner_doc));
    }

    let contract = DataContract::from_value(
        platform_value!({
            "$formatVersion": "1",
            "id": Identifier::new([0xa1; 32]),
            "version": 1u32,
            "ownerId": Identifier::new([0xa2; 32]),
            "config": {
                "$formatVersion": "1",
                "canBeDeleted": false,
                "readonly": false,
                "keepsHistory": false,
                "documentsKeepHistoryContractDefault": false,
                "documentsMutableContractDefault": false,
                "documentsCanBeDeletedContractDefault": false,
                "requiresIdentityEncryptionBoundedKey": Value::Null,
                "requiresIdentityDecryptionBoundedKey": Value::Null,
                "sizedIntegerTypes": true,
            },
            "documentSchemas": {
                "token": {
                    "type": "object",
                    "documentsMutable": false,
                    "canBeDeleted": false,
                    "transferable": 1,
                    "properties": Value::Map(properties),
                    "additionalProperties": false,
                }
            }
        }),
        true,
        pv14,
    )
    .expect("PV14 transferable contract validates");
    platform
        .drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .expect("apply contract");

    let sender_id = Identifier::new([0xa3; 32]);
    let recipient_id = Identifier::new([0xa4; 32]);
    let doc_id = Identifier::new([0xa5; 32]);

    platform
        .drive
        .add_new_identity(
            Identity::V0(IdentityV0 {
                id: sender_id,
                public_keys: BTreeMap::new(),
                balance: 1_000_000,
                revision: 0,
            }),
            false,
            &BlockInfo::default(),
            true,
            None,
            version,
        )
        .expect("insert sender identity");

    let document_type = contract
        .document_type_for_name("token")
        .expect("token type");
    let transferred_doc = Document::V0(DocumentV0 {
        id: doc_id,
        owner_id: recipient_id,
        creator_id: Some(sender_id),
        properties: oversized_props,
        revision: Some(2),
        ..Default::default()
    });
    platform
        .drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((&transferred_doc, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            version,
        )
        .expect("insert transferred oversized document");

    let state_transition = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
        owner_id: sender_id,
        transitions: vec![BatchedTransition::Document(DocumentTransition::Transfer(
            DocumentTransferTransition::V0(DocumentTransferTransitionV0 {
                base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                    id: doc_id,
                    identity_contract_nonce: 1,
                    document_type_name: "token".to_string(),
                    data_contract_id: contract.id(),
                }),
                revision: 2,
                recipient_owner_id: recipient_id,
            }),
        ))],
        user_fee_increase: 0,
        signature_public_key_id: 0,
        signature: Default::default(),
    }));

    let grovedb_proof = platform
        .drive
        .prove_state_transition(&state_transition, None, version)
        .expect("prove transfer state transition")
        .into_data()
        .expect("proof bytes");

    let root = platform
        .drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .value
        .unwrap();

    // Direct Drive verification preserves the merged root on budget exhaustion.
    let direct_err = drive::drive::Drive::verify_state_transition_was_executed_with_proof(
        &state_transition,
        &BlockInfo::default(),
        &grovedb_proof,
        &|id| Ok((*id == contract.id()).then(|| Arc::new(contract.clone()))),
        version,
    )
    .expect_err("oversized transferred document must exceed decode budget");
    assert!(
        matches!(
            direct_err,
            drive::error::Error::Proof(ProofError::DecodedValueBudgetExceeded { root_hash, .. })
                if root_hash == root
        ),
        "expected ProofError::DecodedValueBudgetExceeded with matching root, got: {direct_err:?}"
    );

    let key = SecretKey::from_be_bytes(&[4; 32]).unwrap();
    let provider = Provider {
        contract: Arc::new(contract.clone()),
        key: key.public_key().to_bytes(),
    };
    state.last_committed_block_info = Some(
        ExtendedBlockInfoV0 {
            basic_info: BlockInfo {
                height: 45,
                core_height: 12,
                time_ms: 4000,
                epoch: Default::default(),
            },
            app_hash: [0; 32],
            quorum_hash: [9; 32],
            block_id_hash: [7; 32],
            proposer_pro_tx_hash: [0; 32],
            signature: [0; 96],
            round: 0,
        }
        .into(),
    );
    let metadata = platform.response_metadata_v0(&state, CheckpointUsed::Current);
    let quorum_type = platform.config.validator_set.quorum_type as u32;
    let request = BroadcastStateTransitionRequest {
        state_transition: state_transition
            .serialize_to_bytes()
            .expect("serialize state transition"),
    };

    let signed = signed_proof(grovedb_proof.clone(), root, &metadata, &key, quorum_type);
    let signed_response = WaitForStateTransitionResultResponse {
        version: Some(wait_for_state_transition_result_response::Version::V0(
            wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                result: Some(
                    wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(signed),
                ),
                metadata: Some(metadata.clone()),
            },
        )),
    };
    let verified_err = StateTransitionProofOutcome::maybe_from_proof_with_metadata(
        request.clone(),
        signed_response,
        Network::Testnet,
        version,
        &provider,
    )
    .expect_err("signed oversized transfer proof must surface as UnusableVerifiedResult");
    assert!(
        matches!(
            verified_err,
            drive_proof_verifier::Error::UnusableVerifiedResult { ref reason }
                if reason.contains("decoded value budget exceeded")
        ),
        "expected UnusableVerifiedResult, got: {verified_err:?}"
    );

    let mut wrong_root = root;
    wrong_root[0] ^= 1;
    let bad_sig = signed_proof(grovedb_proof, wrong_root, &metadata, &key, quorum_type);
    let bad_sig_response = WaitForStateTransitionResultResponse {
        version: Some(wait_for_state_transition_result_response::Version::V0(
            wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                result: Some(
                    wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(bad_sig),
                ),
                metadata: Some(metadata),
            },
        )),
    };
    let bad_sig_err = StateTransitionProofOutcome::maybe_from_proof_with_metadata(
        request,
        bad_sig_response,
        Network::Testnet,
        version,
        &provider,
    )
    .expect_err("wrong-root signature must fail as InvalidSignature");
    assert!(
        matches!(
            bad_sig_err,
            drive_proof_verifier::Error::InvalidSignature { .. }
        ),
        "forged signature must stay a node fault, got: {bad_sig_err:?}"
    );
}
