//! Generate deterministic signed responses for strict and affected-state waits.

use dapi_grpc::platform::v0::{
    wait_for_state_transition_result_request, wait_for_state_transition_result_response,
    BroadcastStateTransitionRequest, Proof, ResponseMetadata, WaitForStateTransitionResultRequest,
    WaitForStateTransitionResultResponse,
};
use dapi_grpc::Message;
use dpp::address_funds::AddressWitness;
use dpp::block::block_info::BlockInfo;
use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
use dpp::dashcore::{secp256k1, signer};
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, IdentityV0, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{platform_value, BinaryData, Value};
use dpp::serialization::{PlatformSerializable, PlatformSerializableWithPlatformVersion};
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::document_create_transition::DocumentFromCreateTransition;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::contract_user_moderation_transition::methods::ContractUserModerationTransitionMethodsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::Drive;
use drive::util::test_helpers::setup::{setup_document, setup_drive_with_initial_state_structure};
use serde_json::json;
use std::collections::BTreeMap;
use tenderdash_abci::{
    proto::types::{CanonicalVote, SignedMsgType, StateId},
    signatures::{Hashable, Signable},
};

const CONTRACT_ID: [u8; 32] = [0x41; 32];
const OWNER_ID: [u8; 32] = [0x42; 32];
const TARGET_ID: [u8; 32] = [0x53; 32];
const ENTROPY: [u8; 32] = [0x43; 32];

#[derive(Debug)]
struct FixtureSigner;

#[async_trait::async_trait]
impl Signer<IdentityPublicKey> for FixtureSigner {
    async fn sign(&self, _: &IdentityPublicKey, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        Ok(signer::sign(data, &[1; 32])?.to_vec().into())
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
    SecretKey::from_hash(b"authenticated state transition wait regression")
}

fn contract() -> DataContract {
    let mut contract = DataContract::from_value(
        platform_value!({
            "$formatVersion": "0",
            "id": CONTRACT_ID.to_vec(),
            "ownerId": OWNER_ID.to_vec(),
            "version": 1,
            "documentSchemas": {
                "note": {
                    "type": "object",
                    "documentsMutable": true,
                    "properties": {
                        "a": {"type": "string", "maxLength": 32, "position": 0}
                    },
                    "required": ["a"],
                    "additionalProperties": false
                }
            }
        }),
        false,
        PlatformVersion::latest(),
    )
    .expect("note contract");
    contract.set_config(contract.config().clone().with_moderation(Some(
        ContractModerationConfig {
            banlist: true,
            suspensions: false,
            warnings: false,
            moderators: ContractModerators::ContractOwner,
        },
    )));
    contract
}

fn identity_key() -> IdentityPublicKey {
    IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 1,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::CRITICAL,
        key_type: KeyType::ECDSA_SECP256K1,
        data: secp256k1::PublicKey::from_secret_key(
            &secp256k1::Secp256k1::new(),
            &secp256k1::SecretKey::from_slice(&[1; 32]).expect("identity secret"),
        )
        .serialize()
        .to_vec()
        .into(),
        ..Default::default()
    })
}

fn root(drive: &Drive) -> Vec<u8> {
    drive
        .grove
        .root_hash(None, &PlatformVersion::latest().drive.grove_version)
        .unwrap()
        .expect("GroveDB root")
        .to_vec()
}

fn wait_response(
    drive: &Drive,
    transition: &StateTransition,
) -> WaitForStateTransitionResultResponse {
    let pv = PlatformVersion::latest();
    let metadata = ResponseMetadata {
        height: 100,
        core_chain_locked_height: 1,
        time_ms: 1_000_000,
        protocol_version: pv.protocol_version,
        chain_id: "dash-test-chain".into(),
        ..Default::default()
    };
    let state_id = StateId {
        app_version: u64::from(metadata.protocol_version),
        core_chain_locked_height: metadata.core_chain_locked_height,
        time: metadata.time_ms,
        app_hash: root(drive),
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
    let proof = Proof {
        grovedb_proof: drive
            .prove_state_transition(transition, None, pv)
            .expect("transition proof")
            .data
            .expect("valid proof query"),
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
    };
    WaitForStateTransitionResultResponse {
        version: Some(wait_for_state_transition_result_response::Version::V0(
            wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                result: Some(
                    wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(proof),
                ),
                metadata: Some(metadata),
            },
        )),
    }
}

fn case(drive: &Drive, transition: &StateTransition) -> serde_json::Value {
    let bytes = transition
        .serialize_to_bytes()
        .expect("signed transition bytes");
    let broadcast = BroadcastStateTransitionRequest {
        state_transition: bytes.clone(),
    };
    let wait = WaitForStateTransitionResultRequest {
        version: Some(wait_for_state_transition_result_request::Version::V0(
            wait_for_state_transition_result_request::WaitForStateTransitionResultRequestV0 {
                state_transition_hash: transition
                    .transaction_id()
                    .expect("transition hash")
                    .to_vec(),
                prove: true,
                request_user_balance: false,
            },
        )),
    };
    let response = wait_response(drive, transition);
    json!({
        "transition_bytes": bytes,
        "broadcast_request_bytes": broadcast.encode_to_vec(),
        "wait_request_bytes": wait.encode_to_vec(),
        "response": response,
        "response_bytes": response.encode_to_vec(),
        "state_root": root(drive)
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .ok_or("supply the fixture output path")?;
    let pv = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(pv));
    let contract = contract();
    drive.insert_contract(&contract, BlockInfo::default(), true, None, pv)?;
    let owner = Identity::V0(IdentityV0 {
        id: OWNER_ID.into(),
        public_keys: BTreeMap::from([(1, identity_key())]),
        balance: 1_000_000,
        revision: 1,
    });
    drive.add_new_identity(owner.clone(), false, &BlockInfo::default(), true, None, pv)?;
    let unban = ContractUserModerationTransition::try_from_identity_with_signer(
        &owner,
        &1,
        CONTRACT_ID.into(),
        ContractUserModerationAction::Unban {
            identity_id: TARGET_ID.into(),
        },
        1,
        0,
        &FixtureSigner,
        pv,
        None,
    )
    .await?;
    // The list already lacks the user. Its proof cannot establish an unban executed.
    let snapshot = case(&drive, &unban);
    let document = Document::V0(DocumentV0 {
        id: [0x44; 32].into(),
        owner_id: OWNER_ID.into(),
        revision: Some(1),
        properties: BTreeMap::from([("a".into(), Value::Text("alpha".into()))]),
        ..Default::default()
    });
    let create = BatchTransition::new_document_creation_transition_from_document(
        document,
        contract.document_type_for_name("note")?,
        ENTROPY,
        &identity_key(),
        2,
        0,
        None,
        &FixtureSigner,
        pv,
        None,
    )
    .await?;
    let StateTransition::Batch(ref batch) = create else {
        panic!("document batch")
    };
    let Some(BatchedTransitionRef::Document(DocumentTransition::Create(transition))) =
        batch.first_transition()
    else {
        panic!("document create")
    };
    let committed = Document::try_from_create_transition(
        transition,
        OWNER_ID.into(),
        &BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 1,
            ..Default::default()
        },
        &contract,
        &contract.document_type_for_name("note")?,
        pv,
    )?;
    setup_document(
        &drive,
        &committed,
        &contract,
        contract.document_type_for_name("note")?,
        None,
    );
    let quorum_public_key: Vec<u8> = (&quorum_secret().public_key()).into();
    let vectors = json!({
        "fixture_format_version": 1,
        "protocol_version": pv.protocol_version,
        "generator": "packages/rs-drive-abci/examples/generate_wasm_state_transition_wait_vectors.rs",
        "quorum_public_key": quorum_public_key,
        "inputs": { "height": 100, "time_ms": 1_000_000, "core_chain_locked_height": 1 },
        "contract_bytes": contract.serialize_to_bytes_with_platform_version(pv)?,
        "snapshot": snapshot,
        "execution": case(&drive, &create)
    });
    std::fs::write(
        output,
        format!("{}\n", serde_json::to_string_pretty(&vectors)?),
    )?;
    Ok(())
}
