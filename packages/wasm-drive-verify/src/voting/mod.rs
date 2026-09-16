// Generic functions (with Vec and BTreeMap variants)
pub mod verify_identity_votes_given_proof;
pub mod verify_vote_polls_end_date_query;

// Non-generic functions
pub mod verify_contests_proof;
pub mod verify_masternode_vote;
pub mod verify_specialized_balance;
pub mod verify_vote_poll_vote_state_proof;
pub mod verify_vote_poll_votes_proof;

use dpp::identifier::Identifier;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;

fn bind_vote_poll_context(
    vote_poll: ContestedDocumentResourceVotePoll,
    expected_vote_poll_id: Identifier,
    contract_id: Identifier,
    document_type_name: &str,
    index_name: &str,
) -> Result<ContestedDocumentResourceVotePoll, String> {
    let actual_vote_poll_id = vote_poll
        .unique_id()
        .map_err(|e| format!("Invalid vote poll: {e:?}"))?;
    if vote_poll.contract_id != contract_id
        || vote_poll.document_type_name != document_type_name
        || vote_poll.index_name != index_name
        || actual_vote_poll_id != expected_vote_poll_id
    {
        return Err(
            "vote poll does not match the supplied identifier, contract, and index context"
                .to_string(),
        );
    }

    Ok(vote_poll)
}

// Re-export all public items
pub use verify_contests_proof::*;
pub use verify_identity_votes_given_proof::*;
pub use verify_masternode_vote::*;
pub use verify_specialized_balance::*;
pub use verify_vote_poll_vote_state_proof::*;
pub use verify_vote_poll_votes_proof::*;
pub use verify_vote_polls_end_date_query::*;

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::platform_value::Value;

    #[test]
    fn vote_poll_context_binds_identifier_and_path_fields() {
        let contract_id = Identifier::new([0x11; 32]);
        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![Value::Text("dash".to_string())],
        };
        let expected_id = vote_poll.unique_id().expect("poll ID");

        assert!(bind_vote_poll_context(
            vote_poll.clone(),
            expected_id,
            contract_id,
            "domain",
            "parentNameAndLabel",
        )
        .is_ok());
        assert!(bind_vote_poll_context(
            vote_poll.clone(),
            expected_id,
            Identifier::new([0x22; 32]),
            "domain",
            "parentNameAndLabel",
        )
        .is_err());

        let mut different_poll = vote_poll;
        different_poll.index_values = vec![Value::Text("alice".to_string())];
        assert!(bind_vote_poll_context(
            different_poll,
            expected_id,
            contract_id,
            "domain",
            "parentNameAndLabel",
        )
        .is_err());
    }
}
