//! Genuine signed document outcomes on either side of the owner-balance wire change.
//!
//! The signed producer version selects the execution-proof layout; the verifier's own version
//! keeps every security check.
mod signed_response;

use dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
    wait_for_state_transition_result_response_v0::Result as WaitResult, Version,
    WaitForStateTransitionResultResponseV0,
};
use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, ResponseMetadata, WaitForStateTransitionResultResponse,
};
use dapi_grpc::Message;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::TokenConfiguration;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier, Identity};
use dpp::serialization::{PlatformSerializable, PlatformSerializableWithPlatformVersion};
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_create_transition::{
    DocumentCreateTransition, DocumentCreateTransitionV0,
};
use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV0};
use dpp::state_transition::proof_result::StateTransitionProofOutcome;
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_dpns_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use drive_proof_verifier::{ContextProvider, ContextProviderError, FromProof};
use platform_version::version::mocks::v2_test::TEST_PROTOCOL_VERSION_2;
use signed_response::{signed, Provider, NEWER_PROTOCOL_VERSION};
use std::sync::Arc;

/// The first protocol whose document proofs carry the owner balance. Proofs made with
/// protocol 13 rules use a state built at this version: contracts written for later
/// protocols can use property types that protocol 13 cannot parse.
const FIRST_OWNER_BALANCE_VERSION: u32 = 14;

struct ContractProvider(Arc<DataContract>);
impl ContextProvider for ContractProvider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok((*id == self.0.id()).then(|| self.0.clone()))
    }
    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
    fn get_quorum_public_key(
        &self,
        kind: u32,
        hash: [u8; 32],
        height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Provider.get_quorum_public_key(kind, hash, height)
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}
fn fixture(
    producer: u32,
    proof_version: u32,
) -> (
    BroadcastStateTransitionRequest,
    WaitForStateTransitionResultResponse,
    ContractProvider,
) {
    fixture_signed_as(producer, proof_version, producer)
}
/// Like [`fixture`], but the quorum signs `signed_version`, which may be unknown here.
fn fixture_signed_as(
    producer: u32,
    proof_version: u32,
    signed_version: u32,
) -> (
    BroadcastStateTransitionRequest,
    WaitForStateTransitionResultResponse,
    ContractProvider,
) {
    let producer = PlatformVersion::get(producer).unwrap();
    let drive = setup_drive_with_initial_state_structure(Some(producer));
    let contract = get_dpns_data_contract_fixture(
        Some(Identifier::new([4; 32])),
        0,
        producer.protocol_version,
    )
    .data_contract_owned();
    drive
        .insert_contract(&contract, BlockInfo::default(), true, None, producer)
        .unwrap();
    let owner = Identity::random_identity(2, Some(17), producer).unwrap();
    drive
        .add_new_identity(
            owner.clone(),
            false,
            &BlockInfo::default(),
            true,
            None,
            producer,
        )
        .unwrap();
    let document_type = contract.document_type_for_name("preorder").unwrap();
    let mut document = document_type.random_document(Some(99), producer).unwrap();
    document.set_owner_id(owner.id());
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, None)),
                    owner_id: Some(owner.id().to_buffer()),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            producer,
            None,
        )
        .unwrap();
    let transition = StateTransition::Batch(BatchTransition::V0(BatchTransitionV0 {
        owner_id: owner.id(),
        transitions: vec![DocumentTransition::Create(DocumentCreateTransition::V0(
            DocumentCreateTransitionV0 {
                base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                    id: document.id(),
                    identity_contract_nonce: 1,
                    document_type_name: "preorder".into(),
                    data_contract_id: contract.id(),
                }),
                entropy: [100; 32],
                data: document.properties().clone(),
                prefunded_voting_balance: None,
            },
        ))],
        ..Default::default()
    }));
    let bytes = drive
        .prove_state_transition(
            &transition,
            None,
            PlatformVersion::get(proof_version).unwrap(),
        )
        .unwrap()
        .into_data()
        .unwrap();
    let metadata = ResponseMetadata {
        height: 10,
        core_chain_locked_height: 1,
        time_ms: 1_700_000_000_000,
        protocol_version: signed_version,
        chain_id: "dash-testnet-51".into(),
        ..Default::default()
    };
    let root = drive
        .grove
        .root_hash(None, &producer.drive.grove_version)
        .unwrap()
        .unwrap();
    let response = WaitForStateTransitionResultResponse {
        version: Some(Version::V0(WaitForStateTransitionResultResponseV0 {
            result: Some(WaitResult::Proof(signed(bytes, &metadata, root))),
            metadata: Some(metadata),
        })),
    };
    (
        BroadcastStateTransitionRequest {
            state_transition: transition.serialize_to_bytes().unwrap(),
        },
        response,
        ContractProvider(Arc::new(contract)),
    )
}
fn verify(
    request: BroadcastStateTransitionRequest,
    response: WaitForStateTransitionResultResponse,
    provider: &ContractProvider,
) -> Result<Option<StateTransitionProofOutcome>, Box<drive_proof_verifier::Error>> {
    StateTransitionProofOutcome::maybe_from_proof(
        request,
        response,
        Network::Testnet,
        PlatformVersion::latest(),
        provider,
    )
    .map_err(Box::new)
}
#[test]
fn should_verify_signed_older_document_outcome_with_current_verifier_security() {
    for version in [13, PlatformVersion::latest().protocol_version] {
        let (request, response, provider) = fixture(version, version);
        StateTransitionProofOutcome::maybe_from_proof(
            request.clone(),
            response.clone(),
            Network::Testnet,
            PlatformVersion::get(version).unwrap(),
            &provider,
        )
        .expect("same-version signed fixture verifies");
        let outcome = verify(request, response, &provider).unwrap().unwrap();
        assert!(outcome.is_execution_proved());
        assert_eq!(
            outcome.owner_balance().is_some(),
            version >= FIRST_OWNER_BALANCE_VERSION
        );
    }
}
/// Once a newer protocol activates, clients built before it keep verifying outcomes with
/// the newest rules they know instead of refusing every result until they upgrade.
#[test]
fn should_verify_document_outcome_signed_by_an_unknown_newer_version_with_newest_rules() {
    let latest = PlatformVersion::latest().protocol_version;
    let newer = latest + 1;
    let (request, response, provider) = fixture_signed_as(latest, latest, newer);
    let outcome = verify(request, response, &provider)
        .expect("newest known rules verify the newer producer's outcome")
        .unwrap();
    assert!(outcome.is_execution_proved());
    assert!(outcome.owner_balance().is_some());

    // The newest rules apply, not the oldest: the owner balance is still required.
    let (request, response, provider) = fixture_signed_as(FIRST_OWNER_BALANCE_VERSION, 13, newer);
    let error = verify(request, response, &provider).unwrap_err();
    assert!(
        matches!(*error, drive_proof_verifier::Error::GroveDBError { .. }),
        "{error}"
    );

    // The newer version is still bound by the signature.
    let (request, mut response, provider) = fixture_signed_as(latest, latest, newer);
    let Some(Version::V0(v0)) = &mut response.version else {
        panic!("v0")
    };
    v0.metadata.as_mut().unwrap().protocol_version = newer + 1;
    assert!(matches!(
        *verify(request, response, &provider).unwrap_err(),
        drive_proof_verifier::Error::InvalidSignature { .. }
    ));
}
#[test]
fn should_reject_current_document_outcome_missing_owner_balance() {
    let (request, response, provider) = fixture(FIRST_OWNER_BALANCE_VERSION, 13);
    // Correctly signed: the proof check itself refuses the missing balance.
    let error = verify(request, response, &provider).unwrap_err();
    assert!(
        matches!(*error, drive_proof_verifier::Error::GroveDBError { .. }),
        "{error}"
    );
}
#[test]
fn should_reject_document_outcome_producer_version_tampering() {
    let (request, mut response, provider) = fixture(FIRST_OWNER_BALANCE_VERSION, 13);
    let Some(Version::V0(v0)) = &mut response.version else {
        panic!("v0")
    };
    v0.metadata.as_mut().unwrap().protocol_version = 13;
    assert!(matches!(
        *verify(request, response, &provider).unwrap_err(),
        drive_proof_verifier::Error::InvalidSignature { .. }
    ));
}
/// The producer's version picks the layout only. Every check inside it keeps the verifier's
/// tables, so an older producer cannot bring back rules the verifier no longer applies.
#[test]
fn should_check_document_outcomes_with_the_verifier_tables_not_the_producers() {
    let (request, response, provider) = fixture(13, 13);
    let mut verifier = PlatformVersion::latest().clone();
    verifier.drive.methods.verify.single_document.verify_proof = 255;
    let error = StateTransitionProofOutcome::maybe_from_proof(
        request,
        response,
        Network::Testnet,
        &verifier,
        &provider,
    )
    .unwrap_err();
    assert!(
        matches!(
            &error,
            drive_proof_verifier::Error::DriveError { error }
                if error.contains("SingleDocumentDriveQuery::verify_proof")
                    && error.contains("received: 255")
        ),
        "{error}"
    );
}
#[test]
fn should_preserve_document_outcome_verifier_security_floor() {
    let (request, response, provider) = fixture(13, 13);
    let mut verifier = PlatformVersion::latest().clone();
    verifier
        .drive
        .methods
        .verify
        .state_transition
        .verify_state_transition_was_executed_with_proof = 255;
    let error = StateTransitionProofOutcome::maybe_from_proof(
        request.clone(),
        response.clone(),
        Network::Testnet,
        &verifier,
        &provider,
    )
    .unwrap_err();
    assert!(
        matches!(
            &error,
            drive_proof_verifier::Error::DriveError { error }
                if error.contains("verify_state_transition_was_executed_with_proof")
                    && error.contains("received: 255")
        ),
        "{error}"
    );
    for version in [
        0,
        PlatformVersion::latest().protocol_version + 1,
        0x10000000,
        TEST_PROTOCOL_VERSION_2,
    ] {
        let mut response = response.clone();
        let Some(Version::V0(v0)) = &mut response.version else {
            panic!("v0")
        };
        v0.metadata.as_mut().unwrap().protocol_version = version;
        // Version 0 names no table. A newer version, including the start of the range
        // reserved for mock versions and a mock version this build's tables hold, is
        // read with the newest rules, which need the owner balance this protocol 13
        // proof lacks, so the proof check refuses it before the signature, which
        // covers 13, is checked.
        let error = verify(request.clone(), response, &provider).unwrap_err();
        if version == 0 {
            assert!(
                matches!(
                    *error,
                    drive_proof_verifier::Error::ResponseDecodeError { .. }
                ),
                "{error}"
            );
        } else {
            assert!(
                matches!(*error, drive_proof_verifier::Error::GroveDBError { .. }),
                "version {version}: {error}"
            );
        }
    }
    let mut response = response;
    let Some(Version::V0(v0)) = &mut response.version else {
        panic!("v0")
    };
    let Some(WaitResult::Proof(proof)) = &mut v0.result else {
        panic!("proof")
    };
    proof.grovedb_proof[0] = 0;
    assert!(verify(request, response, &provider)
        .unwrap_err()
        .to_string()
        .contains("proof envelope"));
}
#[test]
#[ignore = "regenerates deterministic signed document-outcome SDK fixtures"]
fn should_export_document_outcome_sdk_fixtures() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rs-drive-proof-verifier/tests/vectors/document-outcome");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("quorum-key.bin"),
        Provider.get_quorum_public_key(106, [2; 32], 0).unwrap(),
    )
    .unwrap();
    // The versions on either side of the owner-balance change, fixed so that re-running on
    // a later branch rewrites the same files the SDK tests read.
    for version in [13, 14] {
        let (request, response, provider) = fixture(version, version);
        verify(request.clone(), response.clone(), &provider).unwrap();
        std::fs::write(
            directory.join(format!("pv{version}-request.bin")),
            request.encode_to_vec(),
        )
        .unwrap();
        std::fs::write(
            directory.join(format!("pv{version}-response.bin")),
            response.encode_to_vec(),
        )
        .unwrap();
        std::fs::write(
            directory.join(format!("pv{version}-contract.bin")),
            provider
                .0
                .serialize_to_bytes_with_platform_version(PlatformVersion::get(version).unwrap())
                .unwrap(),
        )
        .unwrap();
    }
    // A version newer than any build over the latest layout, so that re-running this after
    // a protocol changes the layout rewrites a vector the newest rules still read.
    let latest = PlatformVersion::latest();
    let (request, response, provider) = fixture_signed_as(
        latest.protocol_version,
        latest.protocol_version,
        NEWER_PROTOCOL_VERSION,
    );
    verify(request.clone(), response.clone(), &provider).unwrap();
    std::fs::write(directory.join("newer-request.bin"), request.encode_to_vec()).unwrap();
    std::fs::write(
        directory.join("newer-response.bin"),
        response.encode_to_vec(),
    )
    .unwrap();
    std::fs::write(
        directory.join("newer-contract.bin"),
        provider
            .0
            .serialize_to_bytes_with_platform_version(latest)
            .unwrap(),
    )
    .unwrap();
}
