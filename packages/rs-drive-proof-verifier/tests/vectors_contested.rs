//! Contested-resource (DPNS name contest) proof-vector regression tests:
//! an active contest, a finished contest with a winner, and proof of
//! absence. Requests are built through [`TryFromRequest::try_to_request`]
//! so the gRPC round-trip the SDK relies on is exercised too.

#![cfg(feature = "mocks")]

mod common;

use common::{identifier, load_case, Case, Expected, RequestSpec, NETWORK};
use dapi_grpc::platform::v0::{self as platform, get_contested_resource_vote_state_response};
use dpp::platform_value::Value;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use drive_proof_verifier::from_request::TryFromRequest;
use drive_proof_verifier::types::Contenders;
use drive_proof_verifier::FromProof;

fn contested_pair(
    case: &Case,
) -> (
    platform::GetContestedResourceVoteStateRequest,
    platform::GetContestedResourceVoteStateResponse,
) {
    let RequestSpec::ContestedVoteState {
        contract_id,
        document_type_name,
        index_name,
        index_values,
        count,
    } = &case.manifest.request
    else {
        panic!("{}: expected a contested_vote_state request", case.name);
    };
    let query = ContestedDocumentVotePollDriveQuery {
        vote_poll: ContestedDocumentResourceVotePoll {
            contract_id: identifier(contract_id),
            document_type_name: document_type_name.clone(),
            index_name: index_name.clone(),
            index_values: index_values
                .iter()
                .map(|value| Value::Text(value.clone()))
                .collect(),
        },
        result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
        offset: None,
        limit: Some(*count),
        start_at: None,
        allow_include_locked_and_abstaining_vote_tally: true,
    };
    let request = query
        .try_to_request()
        .expect("contested query must convert to a gRPC request");
    let response = platform::GetContestedResourceVoteStateResponse {
        version: Some(get_contested_resource_vote_state_response::Version::V0(
            get_contested_resource_vote_state_response::GetContestedResourceVoteStateResponseV0 {
                metadata: Some(case.metadata()),
                result: Some(get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::Result::Proof(
                    case.grpc_proof(),
                )),
            },
        )),
    };
    (request, response)
}

fn verify_contested(case: &Case) -> Option<Contenders> {
    let (request, response) = contested_pair(case);
    Contenders::maybe_from_proof(
        request,
        response,
        NETWORK,
        case.platform_version(),
        &case.provider(),
    )
    .unwrap_or_else(|e| panic!("{}: contested proof must verify: {e}", case.name))
}

fn assert_contenders(case: &Case, contenders: &Contenders) {
    let Expected::Contested {
        contenders: expected_contenders,
        abstain_votes,
        lock_votes,
        ..
    } = &case.manifest.expected
    else {
        panic!("{}: manifest expectation mismatch", case.name);
    };
    assert_eq!(
        contenders.contenders.len(),
        expected_contenders.len(),
        "{}: contender count",
        case.name
    );
    for expected in expected_contenders {
        let id = identifier(&expected.identity_id);
        let contender = contenders
            .contenders
            .get(&id)
            .unwrap_or_else(|| panic!("{}: contender {} missing", case.name, expected.identity_id));
        assert_eq!(
            contender.vote_tally(),
            expected.votes,
            "{}: votes for {}",
            case.name,
            expected.identity_id
        );
    }
    assert_eq!(
        contenders.abstain_vote_tally, *abstain_votes,
        "{}: abstain votes",
        case.name
    );
    assert_eq!(
        contenders.lock_vote_tally, *lock_votes,
        "{}: lock votes",
        case.name
    );
}

#[test]
fn contested_vote_state_active() {
    let case = load_case("contested-vote-state-active");
    let contenders = verify_contested(&case).expect("active contest must be found");
    assert_contenders(&case, &contenders);
    assert!(
        contenders.winner.is_none(),
        "active contest must have no winner yet"
    );
}

#[test]
fn contested_vote_state_finished() {
    let case = load_case("contested-vote-state-finished");
    let contenders = verify_contested(&case).expect("finished contest must be found");
    assert_contenders(&case, &contenders);
    let Expected::Contested {
        finished,
        winner_identity_id,
        ..
    } = &case.manifest.expected
    else {
        panic!("manifest expectation mismatch");
    };
    assert!(*finished);
    let (winner_info, finalization_block) = contenders
        .winner
        .as_ref()
        .expect("finished contest must carry winner info");
    let expected_winner = winner_identity_id
        .as_deref()
        .map(identifier)
        .expect("finished fixture names a winner");
    assert_eq!(
        *winner_info,
        ContestedDocumentVotePollWinnerInfo::WonByIdentity(expected_winner)
    );
    assert!(
        finalization_block.time_ms > 0,
        "finalization block info must be present"
    );
}

#[test]
fn contested_vote_state_absent() {
    let case = load_case("contested-vote-state-absent");
    let Expected::ContestedAbsent = case.manifest.expected else {
        panic!("manifest expectation mismatch");
    };
    assert!(
        verify_contested(&case).is_none(),
        "absent contest must verify as proof of non-existence (Ok(None))"
    );
}
