//! Generate deterministic signed responses for the WASM contract-cache regressions.

use dapi_grpc::platform::v0::{
    get_data_contract_response, get_identity_contract_nonce_response,
    wait_for_state_transition_result_response, GetDataContractResponse,
    GetIdentityContractNonceResponse, Proof, ResponseMetadata,
    WaitForStateTransitionResultResponse,
};
use dpp::address_funds::AddressWitness;
use dpp::block::block_info::BlockInfo;
use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
use dpp::dashcore::{secp256k1, signer};
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::{
    Identity, IdentityPublicKey, IdentityV0, KeyType, PartialIdentity, Purpose, SecurityLevel,
};
use dpp::platform_value::{platform_value, BinaryData, Value};
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::document_create_transition::DocumentFromCreateTransition;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::Drive;
use drive::util::test_helpers::setup::{setup_document, setup_drive_with_initial_state_structure};
use serde_json::{json, Map};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
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

fn root(drive: &Drive) -> Vec<u8> {
    drive
        .grove
        .root_hash(None, &PlatformVersion::latest().drive.grove_version)
        .unwrap()
        .expect("state root")
        .to_vec()
}

async fn generate_scenario(non_owner: bool) -> (Map<String, serde_json::Value>, Vec<u8>, Vec<u8>) {
    let pv = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(pv));
    let updated = contract(2);
    drive
        .insert_contract(&updated, BlockInfo::default(), true, None, pv)
        .expect("v2 contract");
    let mut note = document();
    if non_owner {
        note.set_owner_id([0x52; 32].into());
    }
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
            .expect("identity balance");
    }
    let mut responses = Map::new();
    let prefix = if non_owner { "non_owner" } else { "owner" };
    let update = update_transition(contract(2)).await;
    responses.insert(
        format!("{prefix}_update"),
        json!(wait_response(&drive, &update)),
    );
    if !non_owner {
        let mismatch = update_transition(contract(3)).await;
        responses.insert(
            "submitted_v3_update".into(),
            json!(wait_response(&drive, &mismatch)),
        );
    }
    let update_root = root(&drive);
    let key = if non_owner {
        identity_key_with_seed(2)
    } else {
        identity_key()
    };
    let transition = BatchTransition::new_document_creation_transition_from_document(
        note,
        updated.document_type_for_name("note").expect("note type"),
        ENTROPY,
        &key,
        if non_owner { 1 } else { 2 },
        0,
        None,
        &CountingSigner::default(),
        pv,
        None,
    )
    .await
    .expect("document transition");
    let StateTransition::Batch(ref batch) = transition else {
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
    responses.insert(
        format!("{prefix}_write"),
        json!(wait_response(&drive, &transition)),
    );
    let write_root = root(&drive);
    for id in identities {
        let metadata = ResponseMetadata {
            height: 100,
            core_chain_locked_height: 1,
            time_ms: 1_000_000,
            protocol_version: pv.protocol_version,
            chain_id: "dash-test-chain".into(),
            ..Default::default()
        };
        let proof = drive
            .prove_identity_contract_nonce(id, CONTRACT_ID, None, &pv.drive)
            .expect("nonce absence proof");
        let response = GetIdentityContractNonceResponse {
            version: Some(get_identity_contract_nonce_response::Version::V0(
                get_identity_contract_nonce_response::GetIdentityContractNonceResponseV0 {
                    result: Some(get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0::Result::Proof(
                        signed_proof(&drive, proof, &metadata)
                    )), metadata: Some(metadata),
                }
            )),
        };
        let name = if non_owner {
            if id == OWNER_ID {
                "non_owner_owner_nonce"
            } else {
                "non_owner_writer_nonce"
            }
        } else {
            "owner_nonce"
        };
        responses.insert(name.into(), json!(response));
    }
    (responses, update_root, write_root)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .ok_or("supply the fixture output path")?;
    let pv = PlatformVersion::latest();
    let (mut responses, owner_update, owner_write) = generate_scenario(false).await;
    let (second, non_owner_update, non_owner_write) = generate_scenario(true).await;
    responses.extend(second);
    let drive = setup_drive_with_initial_state_structure(Some(pv));
    drive
        .insert_contract(&contract(1), BlockInfo::default(), true, None, pv)
        .expect("older contract");
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
        .expect("older contract proof");
    responses.insert(
        "older_contract_fetch".into(),
        json!(GetDataContractResponse {
            version: Some(get_data_contract_response::Version::V0(
                get_data_contract_response::GetDataContractResponseV0 {
                    result: Some(
                        get_data_contract_response::get_data_contract_response_v0::Result::Proof(
                            signed_proof(&drive, proof, &metadata)
                        )
                    ),
                    metadata: Some(metadata),
                }
            )),
        }),
    );
    assert_eq!(responses.len(), 9);
    let public_key: Vec<u8> = (&quorum_secret().public_key()).into();
    let fixture = json!({
        "fixture_format_version": 1,
        "protocol_version": pv.protocol_version,
        "generator": "packages/rs-drive-abci/examples/generate_wasm_contract_cache_vectors.rs",
        "quorum_public_key": public_key,
        "inputs": {
            "contract_id": CONTRACT_ID.to_vec(), "owner_id": OWNER_ID.to_vec(),
            "entropy": ENTROPY.to_vec(), "note_id": vec![0x44u8;32], "writer_id": vec![0x52u8;32],
            "identity_key_seeds": [1,2], "quorum_seed": "authenticated contract update regression",
            "contract_versions": [1,2,3], "identity_balance": 1_000_000, "identity_revision": 1,
            "update_nonce": 1, "owner_write_nonce": 2, "non_owner_write_nonce": 1,
            "chain_id": "dash-test-chain", "core_height": 1, "height": 100, "time_ms": 1_000_000,
            "older_height": 99, "older_time_ms": 990_000,
            "block_id": vec![0x45u8;32], "quorum_hash": vec![0x46u8;32], "quorum_type": 106,
        },
        "roots": { "owner_update": owner_update, "owner_write": owner_write,
            "non_owner_update": non_owner_update, "non_owner_write": non_owner_write, "older_contract_fetch": root(&drive) },
        "responses": responses,
    });
    let mut encoded = serde_json::to_vec(&fixture)?;
    encoded.push(b'\n');
    std::fs::write(output, encoded)?;
    Ok(())
}
