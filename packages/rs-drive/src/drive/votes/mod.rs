use crate::drive::document::paths::contract_document_type_path;
use crate::drive::votes::paths::{
    RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32, RESOURCE_LOCK_VOTE_TREE_KEY_U8_32,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::Vote;
use dpp::ProtocolError;

#[cfg(feature = "server")]
mod cleanup;

#[cfg(feature = "server")]
mod insert;

/// Paths important for the module
#[cfg(any(feature = "server", feature = "verify"))]
pub mod paths;

#[cfg(feature = "server")]
mod setup;

#[cfg(any(feature = "server", feature = "verify"))]
/// Resolve contested document resource vote poll module
pub mod resolved;
#[cfg(any(feature = "server", feature = "verify"))]
/// Storage form
pub mod storage_form;
#[cfg(any(feature = "server", feature = "verify"))]
/// Tree path storage form
pub mod tree_path_storage_form;

#[cfg(feature = "server")]
mod fetch;

/// A trait to convert the vote to a tree path usable in grovedb
pub trait TreePath {
    /// The tree path function
    fn tree_path<'a>(&'a self, contract: &'a DataContract) -> Result<Vec<&'a [u8]>, ProtocolError>;
}

impl TreePath for Vote {
    fn tree_path<'a>(&'a self, contract: &'a DataContract) -> Result<Vec<&'a [u8]>, ProtocolError> {
        match self {
            Vote::ResourceVote(resource_vote) => resource_vote.tree_path(contract),
        }
    }
}

impl TreePath for ResourceVote {
    fn tree_path<'a>(&'a self, contract: &'a DataContract) -> Result<Vec<&'a [u8]>, ProtocolError> {
        let vote_poll = self.vote_poll();

        match vote_poll {
            VotePoll::ContestedDocumentResourceVotePoll(contested_document_vote_poll) => {
                if contract.id() != contested_document_vote_poll.contract_id {
                    return Err(ProtocolError::VoteError(format!(
                        "contract id of votes {} does not match supplied contract {}",
                        contested_document_vote_poll.contract_id,
                        contract.id()
                    )));
                }
                let document_type = contract.document_type_borrowed_for_name(
                    &contested_document_vote_poll.document_type_name,
                )?;
                let index = document_type
                    .indexes()
                    .get(&contested_document_vote_poll.index_name)
                    .ok_or(ProtocolError::UnknownContestedIndexResolution(format!(
                        "no index named {} for document type {} on contract with id {}",
                        &contested_document_vote_poll.index_name,
                        document_type.name(),
                        contract.id()
                    )))?;
                let mut path = contract_document_type_path(
                    contested_document_vote_poll.contract_id.as_bytes(),
                    &contested_document_vote_poll.document_type_name,
                )
                .to_vec();

                // at this point the path only contains the parts before the index

                let properties_iter = index.properties.iter();

                for index_part in properties_iter {
                    path.push(index_part.name.as_bytes());
                }
                Ok(path)
            }
        }
    }
}

/// A helper trait to get the key for a resource vote
pub trait ResourceVoteChoiceToKeyTrait {
    /// A helper function to get the key for a resource vote
    fn to_key(&self) -> Vec<u8>;
}

impl ResourceVoteChoiceToKeyTrait for ResourceVoteChoice {
    fn to_key(&self) -> Vec<u8> {
        match self {
            ResourceVoteChoice::TowardsIdentity(identity_id) => identity_id.to_vec(),
            ResourceVoteChoice::Abstain => RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.to_vec(),
            ResourceVoteChoice::Lock => RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.to_vec(),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "server")]
mod tests {
    use super::*;
    use dpp::identifier::Identifier;
    use dpp::platform_value::Value;
    use dpp::tests::fixtures::get_dpns_data_contract_fixture;
    use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
    use platform_version::version::PlatformVersion;

    /// Helper: construct a ResourceVote referencing a given contract id / document type
    /// / index name.
    fn make_resource_vote(
        contract_id: Identifier,
        document_type_name: &str,
        index_name: &str,
    ) -> ResourceVote {
        ResourceVote::V0(ResourceVoteV0 {
            vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                ContestedDocumentResourceVotePoll {
                    contract_id,
                    document_type_name: document_type_name.to_string(),
                    index_name: index_name.to_string(),
                    index_values: vec![
                        Value::Text("dash".to_string()),
                        Value::Text("label".to_string()),
                    ],
                },
            ),
            resource_vote_choice: ResourceVoteChoice::Abstain,
        })
    }

    #[test]
    fn resource_vote_choice_to_key_towards_identity_returns_identity_bytes() {
        let id_bytes = [0xAAu8; 32];
        let choice = ResourceVoteChoice::TowardsIdentity(Identifier::from(id_bytes));
        let key = choice.to_key();
        assert_eq!(key, id_bytes.to_vec());
    }

    #[test]
    fn resource_vote_choice_to_key_abstain_matches_constant() {
        assert_eq!(
            ResourceVoteChoice::Abstain.to_key(),
            RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.to_vec()
        );
    }

    #[test]
    fn resource_vote_choice_to_key_lock_matches_constant() {
        assert_eq!(
            ResourceVoteChoice::Lock.to_key(),
            RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.to_vec()
        );
    }

    #[test]
    fn resource_vote_choice_to_key_abstain_and_lock_differ() {
        assert_ne!(
            ResourceVoteChoice::Abstain.to_key(),
            ResourceVoteChoice::Lock.to_key()
        );
    }

    #[test]
    fn tree_path_resource_vote_errors_on_contract_id_mismatch() {
        let pv = PlatformVersion::latest();
        let contract =
            get_dpns_data_contract_fixture(None, 0, pv.protocol_version).data_contract_owned();

        // Use a deliberately *wrong* contract id in the vote poll.
        let wrong_contract_id = Identifier::from([0xEEu8; 32]);
        assert_ne!(wrong_contract_id, contract.id());
        let vote = make_resource_vote(wrong_contract_id, "domain", "parentNameAndLabel");

        let err = vote.tree_path(&contract).expect_err("expected VoteError");
        match err {
            ProtocolError::VoteError(msg) => {
                assert!(
                    msg.contains("does not match supplied contract"),
                    "msg: {msg}"
                );
            }
            other => panic!("expected VoteError, got {other:?}"),
        }
    }

    #[test]
    fn tree_path_resource_vote_errors_on_unknown_document_type() {
        let pv = PlatformVersion::latest();
        let contract =
            get_dpns_data_contract_fixture(None, 0, pv.protocol_version).data_contract_owned();

        let vote = make_resource_vote(contract.id(), "not_a_real_type", "parentNameAndLabel");

        let err = vote.tree_path(&contract).expect_err("expected error");
        // Unknown document type surfaces as a DataContractError wrapped in ProtocolError.
        // Accept any ProtocolError variant -- the important property is that it fails
        // *before* reaching the indexes.get step.
        assert!(
            !matches!(err, ProtocolError::UnknownContestedIndexResolution(_)),
            "should fail on doc_type, not on index lookup: {err:?}"
        );
    }

    #[test]
    fn tree_path_resource_vote_errors_on_unknown_index_name() {
        let pv = PlatformVersion::latest();
        let contract =
            get_dpns_data_contract_fixture(None, 0, pv.protocol_version).data_contract_owned();

        // Valid document type but bogus index name.
        let vote = make_resource_vote(contract.id(), "domain", "no_such_index");

        let err = vote.tree_path(&contract).expect_err("expected error");
        match err {
            ProtocolError::UnknownContestedIndexResolution(msg) => {
                assert!(msg.contains("no index named no_such_index"), "msg: {msg}");
            }
            other => panic!("expected UnknownContestedIndexResolution, got {other:?}"),
        }
    }

    #[test]
    fn tree_path_resource_vote_ok_for_dpns_parent_name_and_label() {
        let pv = PlatformVersion::latest();
        let contract =
            get_dpns_data_contract_fixture(None, 0, pv.protocol_version).data_contract_owned();

        let vote = make_resource_vote(contract.id(), "domain", "parentNameAndLabel");

        let path = vote.tree_path(&contract).expect("tree_path should succeed");
        // The path must be non-empty; we don't hard-code its exact contents so the
        // test doesn't need to be updated whenever dpns changes its contract layout.
        assert!(!path.is_empty());
    }

    #[test]
    fn tree_path_vote_dispatches_to_resource_vote() {
        let pv = PlatformVersion::latest();
        let contract =
            get_dpns_data_contract_fixture(None, 0, pv.protocol_version).data_contract_owned();

        let vote = Vote::ResourceVote(make_resource_vote(
            contract.id(),
            "domain",
            "parentNameAndLabel",
        ));
        let path = vote.tree_path(&contract).expect("tree_path should succeed");
        assert!(!path.is_empty());
    }
}
