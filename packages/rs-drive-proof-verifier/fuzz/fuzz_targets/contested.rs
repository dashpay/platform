//! Contested DPNS name vote state for an active contest, a finished contest
//! and an absent one. Each must verify to what the recorded proof of the
//! signed root yields for the same name.

#![no_main]

use std::sync::LazyLock;

use dapi_grpc::platform::v0::{
    get_contested_resource_vote_state_response, GetContestedResourceVoteStateRequest,
    GetContestedResourceVoteStateResponse,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use drive_proof_verifier::from_request::TryFromRequest;
use drive_proof_verifier::types::Contenders;
use drive_proof_verifier::Error;
use drive_proof_verifier_fuzz::{
    assert_consistent, decode, metadata, proof, recorded, verify, CONTESTED_ABSENT_PROOF,
    CONTESTED_ACTIVE_PROOF, CONTESTED_FINISHED_PROOF, DPNS_CONTRACT, PLATFORM_VERSIONS,
};
use libfuzzer_sys::fuzz_target;

struct Contest {
    request: GetContestedResourceVoteStateRequest,
    recorded: Option<Contenders>,
}

impl Contest {
    fn new(label: &str, recorded_proof: &str) -> Self {
        let request = ContestedDocumentVotePollDriveQuery {
            vote_poll: ContestedDocumentResourceVotePoll {
                contract_id: DPNS_CONTRACT.id(),
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![
                    Value::Text("dash".to_string()),
                    Value::Text(label.to_string()),
                ],
            },
            result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
            offset: None,
            limit: Some(100),
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally: true,
        }
        .try_to_request()
        .expect("contested query must convert to a gRPC request");
        let recorded_proof = decode(recorded_proof);
        let recorded =
            recorded(|platform_version| contenders(platform_version, &request, &recorded_proof));
        Self { request, recorded }
    }
}

static CONTESTS: LazyLock<[Contest; 3]> = LazyLock::new(|| {
    [
        Contest::new("alice", CONTESTED_ACTIVE_PROOF),
        Contest::new("bob", CONTESTED_FINISHED_PROOF),
        Contest::new("carol", CONTESTED_ABSENT_PROOF),
    ]
});

fn contenders(
    platform_version: &PlatformVersion,
    request: &GetContestedResourceVoteStateRequest,
    grovedb_proof: &[u8],
) -> Result<Option<Contenders>, Error> {
    verify::<GetContestedResourceVoteStateRequest, Contenders>(
        platform_version,
        request.clone(),
        GetContestedResourceVoteStateResponse {
            version: Some(get_contested_resource_vote_state_response::Version::V0(
                get_contested_resource_vote_state_response::GetContestedResourceVoteStateResponseV0 {
                    metadata: Some(metadata()),
                    result: Some(
                        get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::Result::Proof(
                            proof(grovedb_proof),
                        ),
                    ),
                },
            )),
        },
    )
}

fuzz_target!(|data: &[u8]| {
    for platform_version in PLATFORM_VERSIONS {
        for contest in CONTESTS.iter() {
            assert_consistent(
                contenders(platform_version, &contest.request, data),
                &contest.recorded,
            );
        }
    }
});
