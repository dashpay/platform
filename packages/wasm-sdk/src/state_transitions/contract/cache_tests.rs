use super::*;
use crate::context_provider::WasmTrustedContext;
use crate::error::WasmSdkErrorKind;
use dapi_grpc::platform::v0::{
    get_data_contract_response, get_identity_contract_nonce_response,
    wait_for_state_transition_result_response, BroadcastStateTransitionRequest,
    BroadcastStateTransitionResponse, GetDataContractRequest, GetDataContractResponse,
    GetIdentityContractNonceRequest, GetIdentityContractNonceResponse, Proof, ResponseMetadata,
    WaitForStateTransitionResultResponse,
};
use dash_sdk::dpp::address_funds::AddressWitness;
use dash_sdk::dpp::block::block_info::BlockInfo;
use dash_sdk::dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
use dash_sdk::dpp::dashcore::{secp256k1, signer, Network};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Setters;
use dash_sdk::dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dash_sdk::dpp::data_contract::TokenConfiguration;
use dash_sdk::dpp::document::{DocumentV0, DocumentV0Setters};
use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dash_sdk::dpp::identity::{IdentityV0, KeyType, PartialIdentity, Purpose, SecurityLevel};
use dash_sdk::dpp::platform_value::{platform_value, BinaryData, Value};
use dash_sdk::dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dash_sdk::dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dash_sdk::dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dash_sdk::dpp::state_transition::batch_transition::document_create_transition::DocumentFromCreateTransition;
use dash_sdk::dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dash_sdk::dpp::state_transition::batch_transition::BatchTransition;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use dash_sdk::platform::transition::put_document::PutDocument;
use dash_sdk::platform::{ContextProvider, Query};
use dash_sdk::{error::ContextProviderError, SdkBuilder};
use drive::drive::Drive;
use drive::util::test_helpers::setup::{setup_document, setup_drive_with_initial_state_structure};
use rs_dapi_client::{transport::TransportRequest, DumpData, ExecutionResponse, RequestSettings};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tenderdash_abci::{
    proto::types::{CanonicalVote, SignedMsgType, StateId},
    signatures::{Hashable, Signable},
};

const CONTRACT_ID: [u8; 32] = [0x41; 32];
const OWNER_ID: [u8; 32] = [0x42; 32];
const ENTROPY: [u8; 32] = [0x43; 32];

#[derive(Debug, Default)]
struct CountingSigner(AtomicUsize);

#[async_trait::async_trait]
impl Signer<IdentityPublicKey> for CountingSigner {
    async fn sign(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(signer::sign(data, &[key.id() as u8; 32])?.to_vec().into())
    }

    async fn sign_create_witness(
        &self,
        key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Ok(AddressWitness::P2pkh {
            signature: self.sign(key, data).await?,
        })
    }

    fn can_sign_with(&self, _: &IdentityPublicKey) -> bool {
        true
    }
}

struct Provider(WasmTrustedContext);

impl ContextProvider for Provider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(self.0.get_known_contract(id))
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
        let key = quorum_secret().public_key();
        let bytes: Vec<u8> = (&key).into();
        Ok(bytes.try_into().expect("BLS public key"))
    }

    fn get_platform_activation_height(&self) -> Result<u32, ContextProviderError> {
        Ok(1)
    }
}

fn quorum_secret() -> SecretKey<Bls12381G2Impl> {
    SecretKey::from_hash(b"authenticated contract update regression")
}

fn contract(version: u32) -> DataContract {
    let mut schema = platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "a": {"type": "string", "maxLength": 32, "position": 0},
        },
        "required": ["a"],
        "additionalProperties": false,
    });
    if version > 1 {
        schema
            .get_value_mut("properties")
            .expect("properties")
            .insert(
                "b".into(),
                platform_value!({
                    "type": "string", "maxLength": 32, "position": 1,
                }),
            )
            .expect("additional property");
    }
    let mut contract = DataContract::from_value(
        platform_value!({
            "$formatVersion": "0",
            "id": CONTRACT_ID.to_vec(),
            "ownerId": OWNER_ID.to_vec(),
            "version": version,
            "documentSchemas": {"note": schema},
        }),
        false,
        PlatformVersion::latest(),
    )
    .expect("mutable note contract");
    contract.set_id(CONTRACT_ID.into());
    contract
}

fn identity_key() -> IdentityPublicKey {
    identity_key_with_seed(1)
}

fn identity_key_with_seed(seed: u8) -> IdentityPublicKey {
    IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: u32::from(seed),
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::CRITICAL,
        key_type: KeyType::ECDSA_SECP256K1,
        data: secp256k1::PublicKey::from_secret_key(
            &secp256k1::Secp256k1::new(),
            &secp256k1::SecretKey::from_slice(&[seed; 32]).expect("identity secret"),
        )
        .serialize()
        .to_vec()
        .into(),
        ..Default::default()
    })
}

fn document() -> Document {
    Document::V0(DocumentV0 {
        id: [0x44; 32].into(),
        owner_id: OWNER_ID.into(),
        revision: Some(1),
        properties: BTreeMap::from([
            ("a".into(), Value::Text("alpha".into())),
            ("b".into(), Value::Text("beta".into())),
        ]),
        ..Default::default()
    })
}

fn signed_proof(drive: &Drive, bytes: Vec<u8>, metadata: &ResponseMetadata) -> Proof {
    let root = drive
        .grove
        .root_hash(None, &PlatformVersion::latest().drive.grove_version)
        .unwrap()
        .expect("GroveDB root");
    let state_id = StateId {
        app_version: metadata.protocol_version as u64,
        core_chain_locked_height: metadata.core_chain_locked_height,
        time: metadata.time_ms,
        app_hash: root.to_vec(),
        height: metadata.height,
    };
    let vote = CanonicalVote {
        r#type: SignedMsgType::Precommit.into(),
        block_id: vec![0x45; 32],
        chain_id: metadata.chain_id.clone(),
        height: metadata.height as i64,
        round: 0,
        state_id: state_id
            .calculate_msg_hash(&metadata.chain_id, metadata.height as i64, 0)
            .expect("state ID hash"),
    };
    let digest = vote
        .calculate_sign_hash(
            &metadata.chain_id,
            106,
            &[0x46; 32],
            metadata.height as i64,
            0,
        )
        .expect("Tenderdash vote hash");
    Proof {
        grovedb_proof: bytes,
        quorum_hash: vec![0x46; 32],
        signature: quorum_secret()
            .sign(SignatureSchemes::Basic, &digest)
            .expect("BLS signature")
            .as_raw_value()
            .to_compressed()
            .to_vec(),
        round: 0,
        block_id_hash: vote.block_id,
        quorum_type: 106,
    }
}

fn dump<R: TransportRequest>(directory: &Path, request: &R, response: R::Response) {
    let result = Ok(ExecutionResponse {
        inner: response,
        retries: 0,
        address: "http://127.0.0.1:9000".parse().expect("transport address"),
    });
    let data = DumpData::new(request, &result);
    data.save(&directory.join(data.filename().expect("request filename")))
        .expect("raw transport fixture");
}

fn wait_response(
    drive: &Drive,
    transition: &StateTransition,
) -> WaitForStateTransitionResultResponse {
    let metadata = ResponseMetadata {
        height: 100,
        core_chain_locked_height: 1,
        time_ms: 1_000_000,
        protocol_version: PlatformVersion::latest().protocol_version,
        chain_id: "dash-test-chain".into(),
        ..Default::default()
    };
    let proof = drive
        .prove_state_transition(transition, None, PlatformVersion::latest())
        .expect("transition proof")
        .data
        .expect("valid proof query");
    WaitForStateTransitionResultResponse {
        version: Some(wait_for_state_transition_result_response::Version::V0(
            wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                result: Some(
                    wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(
                        signed_proof(drive, proof, &metadata),
                    ),
                ),
                metadata: Some(metadata),
            },
        )),
    }
}

async fn update_transition(contract: DataContract) -> StateTransition {
    DataContractUpdateTransition::new_from_data_contract(
        contract,
        &PartialIdentity {
            id: OWNER_ID.into(),
            loaded_public_keys: BTreeMap::from([(1, identity_key())]),
            balance: None,
            revision: None,
            not_found_public_keys: Default::default(),
        },
        1,
        1,
        0,
        &CountingSigner::default(),
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("signed update")
}

async fn fixture(
    mutate_update: impl FnOnce(&mut WaitForStateTransitionResultResponse),
) -> (
    WasmSdk,
    BroadcastStateTransitionRequest,
    BroadcastStateTransitionRequest,
) {
    fixture_with_document(document(), 2, mutate_update, |_| {}).await
}

fn add_older_contract_fetch(sdk: &WasmSdk) -> GetDataContractRequest {
    let pv = PlatformVersion::latest();
    let directory = tempfile::TempDir::new().expect("contract fetch proof");
    let drive = setup_drive_with_initial_state_structure(Some(pv));
    drive
        .insert_contract(&contract(1), BlockInfo::default(), true, None, pv)
        .expect("older committed contract");
    let metadata = ResponseMetadata {
        height: 99,
        core_chain_locked_height: 1,
        time_ms: 990_000,
        protocol_version: pv.protocol_version,
        chain_id: "dash-test-chain".into(),
        ..Default::default()
    };
    let proof = drive
        .prove_contract(CONTRACT_ID, None, pv)
        .expect("contract proof");
    let request: GetDataContractRequest = Identifier::from(CONTRACT_ID)
        .query(&sdk.inner_sdk().query_settings())
        .expect("contract request");
    dump(
        directory.path(),
        &request,
        GetDataContractResponse {
            version: Some(get_data_contract_response::Version::V0(
                get_data_contract_response::GetDataContractResponseV0 {
                    result: Some(
                        get_data_contract_response::get_data_contract_response_v0::Result::Proof(
                            signed_proof(&drive, proof, &metadata),
                        ),
                    ),
                    metadata: Some(metadata),
                },
            )),
        },
    );
    sdk.inner_sdk()
        .clone()
        .mock()
        .load_expectations_sync(directory.path())
        .expect("raw older contract response");
    request
}

async fn fixture_with_document(
    note: Document,
    submitted_version: u32,
    mutate_update: impl FnOnce(&mut WaitForStateTransitionResultResponse),
    mutate_write: impl FnOnce(&mut WaitForStateTransitionResultResponse),
) -> (
    WasmSdk,
    BroadcastStateTransitionRequest,
    BroadcastStateTransitionRequest,
) {
    let pv = PlatformVersion::latest();
    let directory = tempfile::TempDir::new().expect("raw proof fixtures");
    let context = WasmTrustedContext::for_testing(vec![]);
    context.add_known_contract(contract(1));
    let mut sdk = SdkBuilder::new_mock()
        .with_version(pv)
        .with_network(Network::Testnet)
        .with_context_provider(Provider(context.clone()))
        .with_time_tolerance(None)
        .with_settings(RequestSettings {
            retries: Some(0),
            ..Default::default()
        })
        .build()
        .expect("SDK");
    let drive = setup_drive_with_initial_state_structure(Some(pv));
    let updated = contract(2);
    drive
        .insert_contract(&updated, BlockInfo::default(), true, None, pv)
        .expect("v2 contract");
    let writer = note.owner_id();
    let identities = BTreeSet::from([OWNER_ID, writer.to_buffer()]);
    for id in &identities {
        let key = if *id == OWNER_ID {
            identity_key()
        } else {
            identity_key_with_seed(2)
        };
        drive
            .add_new_identity(
                Identity::V0(IdentityV0 {
                    id: (*id).into(),
                    public_keys: BTreeMap::from([(key.id(), key)]),
                    balance: 1_000_000,
                    revision: 1,
                }),
                false,
                &BlockInfo::default(),
                true,
                None,
                pv,
            )
            .expect("owner balance");
    }
    let update = update_transition(contract(submitted_version)).await;
    let mut response = wait_response(&drive, &update);
    mutate_update(&mut response);
    let update_request = update
        .broadcast_request_for_state_transition()
        .expect("update request");
    dump(
        directory.path(),
        &update_request,
        BroadcastStateTransitionResponse {},
    );
    dump(
        directory.path(),
        &update
            .wait_for_state_transition_result_request()
            .expect("update wait"),
        response,
    );

    let document_transition = BatchTransition::new_document_creation_transition_from_document(
        note,
        updated.document_type_for_name("note").expect("note type"),
        ENTROPY,
        &if writer == Identifier::from(OWNER_ID) {
            identity_key()
        } else {
            identity_key_with_seed(2)
        },
        if writer == Identifier::from(OWNER_ID) {
            2
        } else {
            1
        },
        0,
        None,
        &CountingSigner::default(),
        pv,
        None,
    )
    .await
    .expect("signed document create");
    let StateTransition::Batch(ref batch) = document_transition else {
        panic!("document batch")
    };
    let Some(BatchedTransitionRef::Document(DocumentTransition::Create(create))) =
        batch.first_transition()
    else {
        panic!("document create")
    };
    let committed = Document::try_from_create_transition(
        create,
        writer,
        &BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 1,
            ..Default::default()
        },
        &updated,
        &updated.document_type_for_name("note").expect("note type"),
        pv,
    )
    .expect("committed document");
    setup_document(
        &drive,
        &committed,
        &updated,
        updated.document_type_for_name("note").expect("note type"),
        None,
    );
    let write_request = document_transition
        .broadcast_request_for_state_transition()
        .expect("write request");
    let mut write_response = wait_response(&drive, &document_transition);
    mutate_write(&mut write_response);
    dump(
        directory.path(),
        &write_request,
        BroadcastStateTransitionResponse {},
    );
    dump(
        directory.path(),
        &document_transition
            .wait_for_state_transition_result_request()
            .expect("write wait"),
        write_response,
    );

    for id in identities {
        let nonce_request: GetIdentityContractNonceRequest =
            (Identifier::from(id), Identifier::from(CONTRACT_ID))
                .query(&sdk.query_settings())
                .expect("nonce request");
        let metadata = ResponseMetadata {
            height: 100,
            core_chain_locked_height: 1,
            time_ms: 1_000_000,
            protocol_version: pv.protocol_version,
            chain_id: "dash-test-chain".into(),
            ..Default::default()
        };
        let nonce_proof = drive
            .prove_identity_contract_nonce(id, CONTRACT_ID, None, &pv.drive)
            .expect("nonce absence proof");
        dump(directory.path(), &nonce_request, GetIdentityContractNonceResponse {
        version: Some(get_identity_contract_nonce_response::Version::V0(get_identity_contract_nonce_response::GetIdentityContractNonceResponseV0 {
            result: Some(get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0::Result::Proof(signed_proof(&drive, nonce_proof, &metadata))),
            metadata: Some(metadata),
        })),
    });
    }
    sdk.mock()
        .load_expectations_sync(directory.path())
        .expect("raw DAPI responses");
    (
        WasmSdk::new_for_testing(sdk, Some(context)),
        update_request,
        write_request,
    )
}

#[tokio::test]
async fn should_verify_schema_changed_write_after_authenticated_update_in_same_context() {
    let (sdk, update_request, write_request) = fixture(|_| {}).await;
    let signer = CountingSigner::default();
    sdk.update_contract(contract(2), identity_key(), &signer, None)
        .await
        .expect("authenticated contract update");
    let cached = sdk
        .get_or_fetch_contract(CONTRACT_ID.into())
        .await
        .expect("cached contract");
    let result = document()
        .put_to_platform_and_wait_for_response(
            sdk.inner_sdk(),
            cached
                .document_type_for_name("note")
                .expect("cached note type")
                .to_owned_document_type(),
            Some(ENTROPY),
            identity_key(),
            None,
            &signer,
            None,
        )
        .await;
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&update_request)
            .await,
        1
    );
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&write_request)
            .await,
        1,
        "write reached broadcast exactly once"
    );
    assert_eq!(
        signer.0.load(Ordering::SeqCst),
        2,
        "one signature per intended operation"
    );
    let confirmed = result.expect("new schema write must verify after a successful update");
    assert_eq!(
        confirmed.properties().get("b"),
        Some(&Value::Text("beta".into()))
    );
    assert_eq!(cached.version(), 2);
    assert_eq!(
        sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("retained contract")
            .version(),
        2
    );
}

#[tokio::test]
async fn should_not_promote_update_with_invalid_quorum_signature() {
    let (sdk, update_request, _) = fixture(|response| {
        let Some(wait_for_state_transition_result_response::Version::V0(v0)) = response.version.as_mut() else { panic!("response version") };
        let Some(wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(proof)) = v0.result.as_mut() else { panic!("proof") };
        proof.signature = vec![0; 96];
    }).await;
    let signer = CountingSigner::default();
    let error = sdk
        .update_contract(contract(2), identity_key(), &signer, None)
        .await
        .expect_err("invalid signature must fail");
    assert_eq!(error.kind(), WasmSdkErrorKind::Proof);
    assert!(error.is_retriable());
    assert_eq!(
        sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("old contract")
            .version(),
        1
    );
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&update_request)
            .await,
        1
    );
    assert_eq!(signer.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn should_not_promote_update_with_corrupt_grovedb_proof() {
    let (sdk, update_request, _) = fixture(|response| {
        let Some(wait_for_state_transition_result_response::Version::V0(v0)) = response.version.as_mut() else { panic!("response version") };
        let Some(wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(proof)) = v0.result.as_mut() else { panic!("proof") };
        proof.grovedb_proof = vec![1, 255];
    }).await;
    let signer = CountingSigner::default();
    let error = sdk
        .update_contract(contract(2), identity_key(), &signer, None)
        .await
        .expect_err("corrupt proof must fail");
    assert_eq!(error.kind(), WasmSdkErrorKind::Proof);
    assert_eq!(
        sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("old contract")
            .version(),
        1
    );
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&update_request)
            .await,
        1
    );
    assert_eq!(signer.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn should_verify_non_owner_write_through_the_promoted_shared_context() {
    let mut note = document();
    note.set_owner_id([0x52; 32].into());
    let (sdk, update_request, write_request) =
        fixture_with_document(note.clone(), 2, |_| {}, |_| {}).await;
    let signer = CountingSigner::default();
    sdk.update_contract(contract(2), identity_key(), &signer, None)
        .await
        .expect("authenticated update");
    let cached = sdk
        .get_or_fetch_contract(CONTRACT_ID.into())
        .await
        .expect("promoted contract");
    let confirmed = note
        .put_to_platform_and_wait_for_response(
            sdk.inner_sdk(),
            cached
                .document_type_for_name("note")
                .expect("note")
                .to_owned_document_type(),
            Some(ENTROPY),
            identity_key_with_seed(2),
            None,
            &signer,
            None,
        )
        .await
        .expect("non-owner write uses authenticated new schema");
    assert_eq!(
        confirmed.properties().get("b"),
        Some(&Value::Text("beta".into()))
    );
    assert_eq!(signer.0.load(Ordering::SeqCst), 2);
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&update_request)
            .await,
        1
    );
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&write_request)
            .await,
        1
    );
}

#[tokio::test]
async fn should_reject_invalid_document_signature_after_verified_update_without_resubmission() {
    let (sdk, update_request, write_request) = fixture_with_document(document(), 2, |_| {}, |response| {
        let Some(wait_for_state_transition_result_response::Version::V0(v0)) = response.version.as_mut() else { panic!("response version") };
        let Some(wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(proof)) = v0.result.as_mut() else { panic!("proof") };
        proof.signature = vec![0; 96];
    }).await;
    let signer = CountingSigner::default();
    sdk.update_contract(contract(2), identity_key(), &signer, None)
        .await
        .expect("authenticated update");
    let cached = sdk
        .get_or_fetch_contract(CONTRACT_ID.into())
        .await
        .expect("promoted contract");
    let error = document()
        .put_to_platform_and_wait_for_response(
            sdk.inner_sdk(),
            cached
                .document_type_for_name("note")
                .expect("note")
                .to_owned_document_type(),
            Some(ENTROPY),
            identity_key(),
            None,
            &signer,
            None,
        )
        .await
        .expect_err("invalid document signature must remain an error");
    assert!(error.to_string().contains("empty signature"), "{error}");
    assert_eq!(signer.0.load(Ordering::SeqCst), 2);
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&update_request)
            .await,
        1
    );
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&write_request)
            .await,
        1
    );
}

#[tokio::test]
async fn should_reject_a_signed_contract_proof_differing_from_the_submitted_update() {
    let (sdk, update_request, _) = fixture_with_document(document(), 3, |_| {}, |_| {}).await;
    let signer = CountingSigner::default();
    let error = sdk
        .update_contract(contract(3), identity_key(), &signer, None)
        .await
        .expect_err("different proved contract body");
    assert_eq!(error.kind(), WasmSdkErrorKind::Proof);
    assert!(
        error.to_string().contains("exact expected contract"),
        "{error}"
    );
    assert_eq!(
        sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("old contract")
            .version(),
        1
    );
    assert_eq!(signer.0.load(Ordering::SeqCst), 1);
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&update_request)
            .await,
        1
    );
}

#[tokio::test]
async fn should_not_downgrade_an_existing_newer_contract_after_verified_update() {
    let (sdk, update_request, _) = fixture(|_| {}).await;
    sdk.cache_contract(contract(3));
    let signer = CountingSigner::default();
    sdk.update_contract(contract(2), identity_key(), &signer, None)
        .await
        .expect("authenticated v2 update");
    assert_eq!(
        sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("retained v3")
            .version(),
        3
    );
    assert_eq!(signer.0.load(Ordering::SeqCst), 1);
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&update_request)
            .await,
        1
    );
}

#[tokio::test]
async fn should_return_fetched_snapshot_without_regressing_promoted_cache() {
    let context = WasmTrustedContext::for_testing(vec![]);
    let mut inner = SdkBuilder::new_mock()
        .with_version(PlatformVersion::latest())
        .with_network(Network::Testnet)
        .with_context_provider(Provider(context.clone()))
        .build()
        .expect("fetch control SDK");
    // This control observes insertion and return-value policy. The update/write
    // regression above authenticates real proofs without object expectations.
    inner
        .mock()
        .expect_fetch(Identifier::from(CONTRACT_ID), Some(contract(1)))
        .await
        .expect("older fetch snapshot");
    let sdk = WasmSdk::new_for_testing(inner, Some(context));
    sdk.cache_contract(contract(2));
    let fetched = sdk
        .refresh_contract(CONTRACT_ID.into())
        .await
        .expect("older fetch completes");
    assert_eq!(
        fetched.version(),
        1,
        "explicit fetch returns its own snapshot"
    );
    assert_eq!(
        sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("retained contract")
            .version(),
        2
    );
    assert_eq!(
        crate::contract_store::load_all(Network::Testnet, PlatformVersion::latest())
            .iter()
            .find(|c| c.id() == Identifier::from(CONTRACT_ID))
            .expect("durable contract")
            .version(),
        2
    );
}

#[tokio::test]
async fn should_use_updated_schema_when_an_older_shared_fetch_finishes_late() {
    let (sdk, update_request, write_request) = fixture(|_| {}).await;
    let fetch_request = add_older_contract_fetch(&sdk);
    sdk.remove_cached_contract(&CONTRACT_ID.into());
    let fetch_sdk =
        WasmSdk::new_for_testing(sdk.inner_sdk().clone(), sdk.trusted_context().cloned());
    let (release, gate) = futures::channel::oneshot::channel::<()>();
    let mut producer = Box::pin(sdk.join_or_start_contract_fetch(
        CONTRACT_ID.into(),
        move || async move {
            gate.await.expect("release older fetch");
            fetch_sdk.refresh_contract(CONTRACT_ID.into()).await
        },
    ));
    assert!(futures::poll!(producer.as_mut()).is_pending());
    let mut first = Box::pin(sdk.get_or_fetch_contract(CONTRACT_ID.into()));
    let mut second = Box::pin(sdk.get_or_fetch_contract(CONTRACT_ID.into()));
    assert!(futures::poll!(first.as_mut()).is_pending());
    assert!(futures::poll!(second.as_mut()).is_pending());
    assert_eq!(sdk.inflight_contract_fetch_count(), 1);
    assert!(sdk.get_cached_contract(&CONTRACT_ID.into()).is_none());

    let signer = CountingSigner::default();
    sdk.update_contract(contract(2), identity_key(), &signer, None)
        .await
        .expect("authenticated newer update");
    release.send(()).expect("old fetch is still waiting");
    let (producer, first, second) = futures::join!(producer, first, second);
    assert_eq!(
        producer
            .expect("authenticated older fetch snapshot")
            .version(),
        1
    );
    assert_eq!(sdk.inflight_contract_fetch_count(), 0);
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&fetch_request)
            .await,
        1,
        "both cold waiters must share one authenticated fetch"
    );
    let first = first.expect("first cold waiter");
    let second = second.expect("second cold waiter");
    assert_eq!(
        first.version(),
        2,
        "a waiting writer must receive the promoted schema, not the older snapshot"
    );
    assert_eq!(second, first);
    let confirmed = document()
        .put_to_platform_and_wait_for_response(
            sdk.inner_sdk(),
            first
                .document_type_for_name("note")
                .expect("new note type")
                .to_owned_document_type(),
            Some(ENTROPY),
            identity_key(),
            None,
            &signer,
            None,
        )
        .await
        .expect("write using returned new schema");
    assert_eq!(
        confirmed.properties().get("b"),
        Some(&Value::Text("beta".into()))
    );
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&update_request)
            .await,
        1
    );
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&write_request)
            .await,
        1
    );
    assert_eq!(signer.0.load(Ordering::SeqCst), 2);
    assert_eq!(
        sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("retained cache")
            .version(),
        2
    );
    assert_eq!(
        crate::contract_store::load_all(Network::Testnet, PlatformVersion::latest())
            .iter()
            .find(|c| c.id() == Identifier::from(CONTRACT_ID))
            .expect("durable newer contract")
            .version(),
        2
    );
}

#[tokio::test]
async fn should_keep_shared_fetch_error_after_a_verified_update_populates_cache() {
    let (sdk, update_request, _) = fixture(|_| {}).await;
    sdk.remove_cached_contract(&CONTRACT_ID.into());
    let (release, gate) = futures::channel::oneshot::channel::<()>();
    let mut producer = Box::pin(sdk.join_or_start_contract_fetch(
        CONTRACT_ID.into(),
        move || async move {
            gate.await.expect("release failed fetch");
            Err(WasmSdkError::not_found("contract fetch failed"))
        },
    ));
    assert!(futures::poll!(producer.as_mut()).is_pending());
    let mut waiter = Box::pin(sdk.get_or_fetch_contract(CONTRACT_ID.into()));
    assert!(futures::poll!(waiter.as_mut()).is_pending());
    let signer = CountingSigner::default();
    sdk.update_contract(contract(2), identity_key(), &signer, None)
        .await
        .expect("authenticated update");
    release.send(()).expect("failed fetch waiter remains");
    let (producer, waiter) = futures::join!(producer, waiter);
    assert!(producer.is_err());
    let error = waiter.expect_err("a failed fetch must stay visible despite the cache");
    assert_eq!(error.kind(), WasmSdkErrorKind::NotFound);
    assert_eq!(error.message(), "contract fetch failed");
    assert_eq!(sdk.inflight_contract_fetch_count(), 0);
    assert_eq!(
        sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("promoted cache")
            .version(),
        2
    );
    assert_eq!(
        sdk.inner_sdk()
            .clone()
            .mock()
            .request_count(&update_request)
            .await,
        1
    );
    assert_eq!(signer.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn should_return_authenticated_fetch_when_no_trusted_cache_is_available() {
    let (original, _, _) = fixture(|_| {}).await;
    let sdk = WasmSdk::new_for_testing(original.inner_sdk().clone(), None);
    let request = add_older_contract_fetch(&sdk);
    let fetched = sdk
        .get_or_fetch_contract(CONTRACT_ID.into())
        .await
        .expect("proved fetch");
    assert_eq!(fetched.version(), 1);
    assert!(sdk.get_cached_contract(&CONTRACT_ID.into()).is_none());
    assert_eq!(sdk.inflight_contract_fetch_count(), 0);
    assert_eq!(
        sdk.inner_sdk().clone().mock().request_count(&request).await,
        1
    );
}

fn cache_sdk(context: &WasmTrustedContext) -> WasmSdk {
    let inner = SdkBuilder::new_mock()
        .with_version(PlatformVersion::latest())
        .with_network(Network::Testnet)
        .with_context_provider(Provider(context.clone()))
        .build()
        .expect("cache SDK");
    WasmSdk::new_for_testing(inner, Some(context.clone()))
}

#[test]
fn should_keep_newer_contract_in_shared_memory_and_persistence_after_old_completion() {
    let context = WasmTrustedContext::for_testing(vec![]);
    let sdk = cache_sdk(&context);
    let shared = cache_sdk(&context);
    sdk.cache_contract(contract(2));
    shared.cache_contract(contract(1));
    assert_eq!(
        sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("shared contract")
            .version(),
        2
    );
    let persisted = crate::contract_store::load_all(Network::Testnet, PlatformVersion::latest());
    assert_eq!(
        persisted
            .iter()
            .find(|c| c.id() == Identifier::from(CONTRACT_ID))
            .expect("persisted contract")
            .version(),
        2
    );
}

#[test]
fn should_keep_existing_equal_version_body_for_internal_insertions() {
    let context = WasmTrustedContext::for_testing(vec![]);
    let sdk = cache_sdk(&context);
    let existing = contract(2);
    let mut equal = existing.clone();
    equal.set_owner_id([0x47; 32].into());
    sdk.cache_contract(existing.clone());
    sdk.cache_contract(equal);
    assert_eq!(
        *sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("cached body"),
        existing
    );
}

#[test]
fn should_preserve_explicit_manual_replacement_with_older_and_equal_versions() {
    let context = WasmTrustedContext::for_testing(vec![]);
    let sdk = cache_sdk(&context);
    sdk.cache_contract(contract(2));
    assert!(sdk.add_known_contract_js(&contract(1).into()));
    let mut equal = contract(1);
    equal.set_owner_id([0x48; 32].into());
    assert!(sdk.add_known_contract_js(&equal.clone().into()));
    assert_eq!(
        *sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("manual body"),
        equal
    );
    assert!(
        crate::contract_store::load_all(Network::Testnet, PlatformVersion::latest())
            .contains(&equal)
    );
    assert!(sdk.remove_cached_contract_js(&CONTRACT_ID.into()));
    assert!(sdk.get_cached_contract(&CONTRACT_ID.into()).is_none());
}

#[test]
fn should_hydrate_shared_context_without_downgrading_newer_contract() {
    let context = WasmTrustedContext::for_testing(vec![]);
    context.add_known_contract(contract(3));
    crate::contract_store::store(Network::Testnet, &contract(1), PlatformVersion::latest());
    let sdk = crate::sdk::WasmSdkBuilder::new_testnet()
        .with_trusted_context(&context)
        .build()
        .expect("SDK from older durable snapshot");
    assert_eq!(
        sdk.get_cached_contract(&CONTRACT_ID.into())
            .expect("shared newer contract")
            .version(),
        3
    );
}

#[test]
fn should_preserve_durable_authority_when_an_independent_stale_context_inserts() {
    let promoted = cache_sdk(&WasmTrustedContext::for_testing(vec![]));
    let stale = cache_sdk(&WasmTrustedContext::for_testing(vec![]));
    let existing = contract(2);
    promoted.cache_contract(existing.clone());
    stale.cache_contract(contract(1));
    assert_eq!(
        stale
            .get_cached_contract(&CONTRACT_ID.into())
            .expect("independent old memory")
            .version(),
        1
    );
    let mut equal = existing.clone();
    equal.set_owner_id([0x49; 32].into());
    stale.cache_contract(equal);
    let persisted = crate::contract_store::load_all(Network::Testnet, PlatformVersion::latest());
    assert!(
        persisted.contains(&existing),
        "lower and equal internal writes preserve the durable body"
    );
    let fresh = crate::sdk::WasmSdkBuilder::new_testnet()
        .with_trusted_context(&WasmTrustedContext::for_testing(vec![]))
        .build()
        .expect("fresh SDK from persisted contract");
    assert_eq!(
        *fresh
            .get_cached_contract(&CONTRACT_ID.into())
            .expect("fresh authenticated body"),
        existing
    );
}
