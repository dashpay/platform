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
use drive::drive::document::history::DocumentHistoryProofV1;
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
        .calculate_sign_hash(&metadata.chain_id, 1, &[9; 32], metadata.height as i64, 0)
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
        quorum_type: 1,
    }
}

#[test]
fn should_round_trip_history_api_through_quorum_and_grove_proof_verification() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
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
    let metadata = ResponseMetadata {
        height: 42,
        core_chain_locked_height: 12,
        epoch: 0,
        time_ms: 3000,
        protocol_version: 14,
        chain_id: "history-test-chain".into(),
    };
    let root = platform
        .drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .value
        .unwrap();
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
            prove: false,
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
        let (
            expected,
            DocumentHistoryProofV1 {
                entries_proof,
                metadata_proof,
            },
        ) = platform
            .drive
            .prove_document_history_v1(&query, document_type, None, version)
            .unwrap();
        response.entries_proof =
            entries_proof.map(|bytes| signed_proof(bytes, root, &metadata, &key));
        response.metadata_proof = Some(signed_proof(metadata_proof, root, &metadata, &key));
        response.metadata = Some(metadata.clone());
        let request: GetDocumentHistoryRequest = GetDocumentHistoryRequestV1 {
            prove: true,
            ..request
        }
        .into();
        let verify = |response: GetDocumentHistoryResponseV1| {
            DocumentHistory::maybe_from_proof(
                request.clone(),
                GetDocumentHistoryResponse::from(response),
                Network::Testnet,
                version,
                &provider,
            )
        };
        let result = verify(response.clone()).unwrap().unwrap();
        assert_eq!(result.entries, expected.entries);
        assert_eq!(result.lifecycle, Some(expected.lifecycle));
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
