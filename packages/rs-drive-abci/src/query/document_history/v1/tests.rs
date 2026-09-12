use super::*;
use crate::query::tests::setup_platform;
use dapi_grpc::platform::v0::get_document_history_request::get_document_history_request_v1::Cursor;
use dapi_grpc::platform::v0::{
    GetDocumentHistoryRequest, GetDocumentHistoryResponse, Proof, ResponseMetadata,
};
use dpp::block::block_info::BlockInfo;
use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
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
    key: &SecretKey<Bls12381G2Impl>,
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
        signature: key
            .sign(SignatureSchemes::Basic, &digest)
            .unwrap()
            .as_raw_value()
            .to_compressed()
            .to_vec(),
        round: 0,
        block_id_hash: vec![7; 32],
        quorum_type,
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
                None,
            )
            .unwrap();
    }
    let key = SecretKey::<Bls12381G2Impl>::from_hash(b"document-history-quorum");
    let provider = Provider {
        contract: Arc::new(contract.clone()),
        key: key.public_key().to_bytes().try_into().unwrap(),
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
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Setters;
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
        for (wire_selector, selector) in [
            (Selector::Revision(3), DocumentHistorySelector::Revision(3)),
            (
                Selector::StartAtRevision(3),
                DocumentHistorySelector::StartAtRevision(3),
            ),
        ] {
            let request = GetDocumentHistoryRequestV1 {
                data_contract_id: contract.id().to_vec(),
                document_type_name: "profile".into(),
                document_id: document.id().to_vec(),
                limit: None,
                prove: true,
                selector: Some(wire_selector),
            };
            let mut time_request = request.clone();
            time_request.selector = Some(Selector::StartAtMs(0));
            let mut response = platform
                .query_document_history_v1(time_request, &state, version)
                .unwrap()
                .into_data()
                .unwrap();
            let query = DocumentHistoryQueryV1 {
                contract_id: contract.id().to_buffer(),
                document_type_name: "profile".into(),
                document_id: document.id().to_buffer(),
                selector,
                limit: None,
            };
            response.entries.clear();
            response.lifecycle.as_mut().unwrap().remaining_revisions = 3;
            response.metadata_proof.as_mut().unwrap().grovedb_proof =
                drive::util::test_helpers::history_proof::downgrade_history_count(
                    &response.metadata_proof.as_ref().unwrap().grovedb_proof,
                    Some(3),
                    version,
                );
            response.entries_proof.as_mut().unwrap().grovedb_proof = platform
                .drive
                .grove_get_proved_path_query(
                    &query.entries_query(version).unwrap(),
                    None,
                    &mut vec![],
                    &version.drive,
                )
                .unwrap();
            let error = DocumentHistory::maybe_from_proof(
                request.clone(),
                response.clone(),
                Network::Testnet,
                version,
                &provider,
            )
            .expect_err("history v1 requires GroveDB v1 even with a valid quorum signature");
            assert!(error.to_string().contains("unsupported proof version"));
            assert!(drive_proof_verifier::types::DocumentHistoryProofInfo::maybe_from_proof_with_metadata(
                request, response, Network::Testnet, version, &provider,
            ).is_err());
        }
        return;
    }
    let selections = [
        (
            document.id().to_buffer(),
            Selector::StartAtMs(2000),
            DocumentHistorySelector::StartAtTime(2000),
        ),
        (
            document.id().to_buffer(),
            Selector::StartAfter(Cursor {
                time_ms: 2000,
                revision: 1,
            }),
            DocumentHistorySelector::StartAfter {
                time_ms: 2000,
                revision: 1,
            },
        ),
        (
            document.id().to_buffer(),
            Selector::Revision(2),
            DocumentHistorySelector::Revision(2),
        ),
        (
            document.id().to_buffer(),
            Selector::StartAtRevision(65535),
            DocumentHistorySelector::StartAtRevision(65535),
        ),
        (
            [255; 32],
            Selector::Revision(2),
            DocumentHistorySelector::Revision(2),
        ),
    ];
    for (id, wire_selector, selector) in selections {
        let request = GetDocumentHistoryRequestV1 {
            data_contract_id: contract.id().to_vec(),
            document_type_name: "profile".into(),
            document_id: id.to_vec(),
            limit: None,
            prove: true,
            selector: Some(wire_selector),
        };
        let mut response = platform
            .query_document_history_v1(request.clone(), &state, version)
            .unwrap()
            .into_data()
            .unwrap();
        let query = DocumentHistoryQueryV1 {
            contract_id: contract.id().to_buffer(),
            document_type_name: "profile".into(),
            document_id: id,
            limit: None,
            selector,
        };
        let expected = platform
            .drive
            .fetch_document_history_v1(&query, document_type, None, version)
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
        assert_eq!(exported.response, response);
        assert_eq!(
            exported
                .verify(request.clone(), Network::Testnet, version, &provider)
                .unwrap(),
            Some(exported.history)
        );
        let result = verify(response.clone()).unwrap().unwrap();
        assert_eq!(result.entries, expected.entries);
        assert_eq!(result.lifecycle, Some(expected.lifecycle));
        let mut legacy_metadata = response.clone();
        let bytes = &mut legacy_metadata
            .metadata_proof
            .as_mut()
            .unwrap()
            .grovedb_proof;
        *bytes =
            drive::util::test_helpers::history_proof::downgrade_history_count(bytes, None, version);
        let error =
            verify(legacy_metadata).expect_err("metadata always requires a GroveDB v1 envelope");
        assert!(error.to_string().contains("unsupported proof version"));
        let mut bad_count = response.clone();
        bad_count.lifecycle.as_mut().unwrap().remaining_revisions += 1;
        assert!(
            verify(bad_count).is_err(),
            "wire counts must match the authenticated tree count"
        );
        let mut bad_state = response.clone();
        bad_state.lifecycle.as_mut().unwrap().state = 100;
        assert!(verify(bad_state).is_err());
        let mut bad_signature = response.clone();
        bad_signature.metadata_proof.as_mut().unwrap().signature[0] ^= 1;
        assert!(verify(bad_signature).is_err());
        if response.entries_proof.is_some() {
            let mut downgraded = response.clone();
            let bytes = &mut downgraded.entries_proof.as_mut().unwrap().grovedb_proof;
            *bytes = drive::util::test_helpers::history_proof::downgrade_history_count(
                bytes, None, version,
            );
            assert!(
                verify(downgraded).is_err(),
                "entries also require an authenticated V1 envelope"
            );
            let mut missing = response.clone();
            missing.entries_proof = None;
            assert!(verify(missing).is_err());
        }
        if !response.entries.is_empty() {
            response.entries[0].revision += 1;
            assert!(verify(response).is_err());
        }
    }
}

#[test]
fn should_reject_missing_selectors_before_reading_state() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let request = GetDocumentHistoryRequestV1 {
        data_contract_id: vec![1; 32],
        document_type_name: "note".into(),
        document_id: vec![2; 32],
        selector: None,
        limit: None,
        prove: false,
    };
    assert!(!platform
        .query_document_history_v1(request, &state, version)
        .unwrap()
        .is_valid());
}

/// A response that claims a lifecycle other than the one its proofs establish
/// is rejected field by field, for a deleted document and for one whose
/// erasure is under way, not only for a current one.
#[test]
fn should_reject_tampered_lifecycle_claims_for_deleted_and_erasing_histories() {
    use dapi_grpc::platform::v0::get_document_history_request::get_document_history_request_v1::Selector;
    use dapi_grpc::platform::v0::get_document_history_request::GetDocumentHistoryRequestV1;
    use dapi_grpc::platform::v0::get_document_history_response::GetDocumentHistoryResponseV1;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Setters;
    use drive_proof_verifier::types::DocumentHistoryState;

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
    // Enough revisions that one erase chunk leaves some behind.
    let chunk = version
        .system_limits
        .max_document_revisions_erased_per_transition
        .expect("protocol 14 bounds the erase chunk") as u64;
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
                None,
            )
            .unwrap();
    }
    let key = SecretKey::<Bls12381G2Impl>::from_hash(b"document-history-quorum");
    let provider = Provider {
        contract: Arc::new(contract.clone()),
        key: key.public_key().to_bytes().try_into().unwrap(),
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
    let apply = |batch| {
        platform
            .drive
            .apply_batch_low_level_drive_operations(None, None, batch, &mut vec![], &version.drive)
            .unwrap();
    };
    let v1_request = GetDocumentHistoryRequestV1 {
        data_contract_id: contract.id().to_vec(),
        document_type_name: "profile".into(),
        document_id: document.id().to_vec(),
        limit: None,
        prove: true,
        selector: Some(Selector::StartAtMs(0)),
    };
    let request: GetDocumentHistoryRequest = v1_request.clone().into();

    // Delete, then start an erasure; after each, sign the new root and check
    // that the honest response verifies while every altered claim does not.
    let stages: [(&str, Box<dyn Fn()>, DocumentHistoryState); 2] = [
        (
            "deleted",
            Box::new(|| {
                apply(
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
                        .unwrap(),
                )
            }),
            DocumentHistoryState::Deleted,
        ),
        (
            "erasing",
            Box::new(|| {
                apply(
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
                        .unwrap(),
                )
            }),
            DocumentHistoryState::Erasing,
        ),
    ];
    for (stage, advance, expected_state) in stages {
        advance();
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
            .query_document_history_v1(v1_request.clone(), &state, version)
            .unwrap()
            .into_data()
            .unwrap();
        let verify = |response: GetDocumentHistoryResponseV1| {
            DocumentHistory::maybe_from_proof(
                request.clone(),
                GetDocumentHistoryResponse::from(response),
                Network::Testnet,
                version,
                &provider,
            )
        };
        let honest = verify(response.clone())
            .unwrap_or_else(|error| panic!("{stage}: honest response must verify: {error}"))
            .unwrap();
        let lifecycle = honest
            .lifecycle
            .expect("history v1 always carries a lifecycle");
        assert_eq!(lifecycle.state, expected_state, "{stage}");
        assert_eq!(lifecycle.times.deleted_at_ms, 5_000, "{stage}");

        let tamperings: Vec<(&str, Box<dyn Fn(&mut GetDocumentHistoryResponseV1)>)> = vec![
            (
                "state",
                Box::new(|r| r.lifecycle.as_mut().unwrap().state = 0),
            ),
            (
                "remaining_revisions",
                Box::new(|r| r.lifecycle.as_mut().unwrap().remaining_revisions += 1),
            ),
            (
                "deleted_at_ms",
                Box::new(|r| r.lifecycle.as_mut().unwrap().deleted_at_ms += 1),
            ),
            (
                "erasing_started_at_ms",
                Box::new(|r| r.lifecycle.as_mut().unwrap().erasing_started_at_ms += 1),
            ),
            (
                "erasing_from_time_ms",
                Box::new(|r| r.lifecycle.as_mut().unwrap().erasing_from_time_ms += 1),
            ),
            (
                "erasing_from_revision",
                Box::new(|r| r.lifecycle.as_mut().unwrap().erasing_from_revision += 1),
            ),
        ];
        for (field, tamper) in tamperings {
            let mut tampered = response.clone();
            tamper(&mut tampered);
            assert!(
                verify(tampered).is_err(),
                "{stage}: a response whose {field} differs from its proof must be rejected"
            );
        }
    }
}
