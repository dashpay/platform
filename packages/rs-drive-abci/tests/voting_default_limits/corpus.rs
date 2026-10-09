//! Regenerate the committed client responses from deterministic server state.

use super::fixture;
use dapi_grpc::{
    platform::v0::{
        get_contested_resource_identity_votes_response as identity_response,
        get_vote_polls_by_end_date_response as polls_response,
        GetContestedResourceIdentityVotesResponse, GetVotePollsByEndDateResponse, Proof,
        ResponseMetadata,
    },
    Message,
};
use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
use dpp::prelude::Identifier;
use drive::config::DEFAULT_QUERY_LIMIT;
use drive::query::{
    contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery,
    VotePollsByEndDateDriveQuery,
};
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tenderdash_abci::{
    proto::types::{CanonicalVote, SignedMsgType, StateId},
    signatures::{Hashable, Signable},
};

#[derive(Serialize)]
struct Case {
    endpoint: &'static str,
    count: usize,
    ascending: bool,
    limit: u16,
    kind: &'static str,
    filename: String,
}

#[derive(Serialize)]
struct Manifest {
    protocol_version: u32,
    quorum_public_key: Vec<u8>,
    wrong_quorum_public_key: Vec<u8>,
    cases: Vec<Case>,
}

fn signing_key() -> SecretKey<Bls12381G2Impl> {
    SecretKey::from_hash(b"voting-default-limit-test")
}

fn authenticated(proof_bytes: Vec<u8>, root: [u8; 32]) -> (Proof, ResponseMetadata) {
    let metadata = ResponseMetadata {
        height: 100,
        core_chain_locked_height: 1,
        epoch: 0,
        time_ms: fixture::END_TIME,
        protocol_version: fixture::PROTOCOL_VERSION,
        chain_id: "test-chain".into(),
    };
    let mut proof = Proof {
        grovedb_proof: proof_bytes,
        quorum_hash: vec![2; 32],
        signature: vec![],
        round: 1,
        block_id_hash: vec![3; 32],
        quorum_type: 1,
    };
    let state = StateId {
        app_version: metadata.protocol_version as u64,
        core_chain_locked_height: metadata.core_chain_locked_height,
        time: metadata.time_ms,
        app_hash: root.to_vec(),
        height: metadata.height,
    };
    let vote = CanonicalVote {
        r#type: SignedMsgType::Precommit.into(),
        block_id: proof.block_id_hash.clone(),
        chain_id: metadata.chain_id.clone(),
        height: metadata.height as i64,
        round: proof.round as i64,
        state_id: state
            .calculate_msg_hash(
                &metadata.chain_id,
                metadata.height as i64,
                proof.round as i32,
            )
            .expect("signed state digest"),
    };
    let digest = vote
        .calculate_sign_hash(
            &metadata.chain_id,
            proof.quorum_type as u8,
            &[2; 32],
            metadata.height as i64,
            proof.round as i32,
        )
        .expect("vote digest");
    proof.signature = signing_key()
        .sign(SignatureSchemes::Basic, &digest)
        .expect("test signature")
        .as_raw_value()
        .to_compressed()
        .to_vec();
    (proof, metadata)
}

fn poll_response(proof: Proof, metadata: ResponseMetadata) -> GetVotePollsByEndDateResponse {
    GetVotePollsByEndDateResponse {
        version: Some(polls_response::Version::V0(
            polls_response::GetVotePollsByEndDateResponseV0 {
                result: Some(
                    polls_response::get_vote_polls_by_end_date_response_v0::Result::Proof(proof),
                ),
                metadata: Some(metadata),
            },
        )),
    }
}

fn vote_response(
    proof: Proof,
    metadata: ResponseMetadata,
) -> GetContestedResourceIdentityVotesResponse {
    GetContestedResourceIdentityVotesResponse { version: Some(identity_response::Version::V0(
        identity_response::GetContestedResourceIdentityVotesResponseV0 { result: Some(
            identity_response::get_contested_resource_identity_votes_response_v0::Result::Proof(proof)
        ), metadata: Some(metadata) }
    )) }
}

fn check_file(directory: &Path, filename: &str, expected: &[u8], update: bool) {
    let file = directory.join(filename);
    if update {
        fs::write(&file, expected).expect("write requested fixture");
    }
    let committed = fs::read(&file).expect("committed response fixture");
    assert_eq!(committed, expected, "fixture drift: {filename}");
}

#[test]
fn should_match_committed_voting_default_limit_responses() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../rs-sdk/tests/vectors/voting_default_limits");
    let update = std::env::var("UPDATE_VOTING_DEFAULT_LIMIT_FIXTURES").as_deref() == Ok("1");
    if update {
        fs::create_dir_all(&directory).expect("fixture corpus directory");
    }
    let version = fixture::version();
    let mut cases = Vec::new();
    for count in [
        3,
        DEFAULT_QUERY_LIMIT as usize,
        DEFAULT_QUERY_LIMIT as usize + 1,
    ] {
        let drive = setup_drive_with_initial_state_structure(None);
        fixture::populate(&drive, count, version);
        let root = drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .unwrap()
            .expect("root");
        for ascending in [true, false] {
            for limit in [DEFAULT_QUERY_LIMIT, 2] {
                let direction = if ascending { "asc" } else { "desc" };
                let query = VotePollsByEndDateDriveQuery {
                    start_time: None,
                    end_time: None,
                    limit: Some(limit),
                    offset: None,
                    order_ascending: ascending,
                };
                let (bytes, _) = query
                    .execute_with_proof(&drive, None, None, version)
                    .expect("poll proof");
                let (proof, metadata) = authenticated(bytes.clone(), root);
                let filename = format!("polls-{count}-{direction}-{limit}.bin");
                check_file(
                    &directory,
                    &filename,
                    &poll_response(proof, metadata).encode_to_vec(),
                    update,
                );
                cases.push(Case {
                    endpoint: "polls",
                    count,
                    ascending,
                    limit,
                    kind: "valid",
                    filename,
                });
                if count == DEFAULT_QUERY_LIMIT as usize + 1
                    && ascending
                    && limit == DEFAULT_QUERY_LIMIT
                {
                    for kind in ["wrong-root", "wrong-signature"] {
                        let (mut proof, metadata) = authenticated(
                            bytes.clone(),
                            if kind == "wrong-root" { [0; 32] } else { root },
                        );
                        if kind == "wrong-signature" {
                            proof.signature = signing_key()
                                .sign(SignatureSchemes::Basic, &[0; 32])
                                .expect("wrong-digest signature")
                                .as_raw_value()
                                .to_compressed()
                                .to_vec();
                        }
                        let filename = format!("polls-{count}-{direction}-{limit}-{kind}.bin");
                        check_file(
                            &directory,
                            &filename,
                            &poll_response(proof, metadata).encode_to_vec(),
                            update,
                        );
                        cases.push(Case {
                            endpoint: "polls",
                            count,
                            ascending,
                            limit,
                            kind,
                            filename,
                        });
                    }
                }
                let query = ContestedResourceVotesGivenByIdentityQuery {
                    identity_id: Identifier::from(fixture::VOTER),
                    limit: Some(limit),
                    offset: None,
                    start_at: None,
                    order_ascending: ascending,
                };
                let (bytes, _) = query
                    .execute_with_proof(&drive, None, None, version)
                    .expect("identity-vote proof");
                let (proof, metadata) = authenticated(bytes, root);
                let filename = format!("votes-{count}-{direction}-{limit}.bin");
                check_file(
                    &directory,
                    &filename,
                    &vote_response(proof, metadata).encode_to_vec(),
                    update,
                );
                cases.push(Case {
                    endpoint: "votes",
                    count,
                    ascending,
                    limit,
                    kind: "valid",
                    filename,
                });
            }
        }
    }
    let manifest = Manifest {
        protocol_version: fixture::PROTOCOL_VERSION,
        quorum_public_key: (&signing_key().public_key()).into(),
        wrong_quorum_public_key: (&SecretKey::<Bls12381G2Impl>::from_hash(
            b"unrelated-voting-fixture-key",
        )
        .public_key())
            .into(),
        cases,
    };
    assert_eq!(manifest.cases.len(), 26);
    let mut bytes = serde_json::to_vec_pretty(&manifest).expect("fixture manifest");
    bytes.push(b'\n');
    check_file(&directory, "manifest.json", &bytes, update);
    println!(
        "verified 26 deterministic responses and manifest for PV{}",
        fixture::PROTOCOL_VERSION
    );
}
