//! Multiple choice proposals (protocol version 15): what clients name them and their vote by.
//!
//! A multiple choice proposal is a `multipleChoiceProposal` document of the decision proposals
//! contract; its answers are `choice` documents, which contend in a masternode vote among
//! documents (`ContestedIndexResolution::MasternodeVoteOnDocuments`). The vote is keyed by the
//! proposal's id ([`multiple_choice_proposal_vote_poll`]), and a choice is named in it by its
//! own document id, wherever a contest among identities names a contender by an identity id
//! ([`MultipleChoiceVoteChoice`]).

use crate::decision_proposal::DECISION_PROPOSALS_CONTRACT_ID;
use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use crate::voting::vote_polls::VotePoll;
use crate::voting::votes::resource_vote::v0::ResourceVoteV0;
use crate::voting::votes::resource_vote::ResourceVote;
use crate::voting::votes::Vote;
use platform_value::{Identifier, Value};

/// The name of the choice document type: an answer to a multiple choice proposal.
pub const CHOICE_DOCUMENT_TYPE_NAME: &str = "choice";

/// The contested index every choice falls under, on the proposal it answers. A choice's create
/// states its contest fund under this name.
pub const CHOICE_CONTEST_INDEX_NAME: &str = "byProposal";

/// The properties of a multiple choice proposal.
pub mod proposal_properties {
    /// The question, in one line. Editable for two hours after the proposal is created.
    pub const TITLE: &str = "title";
    /// The document saying what is being decided; optional, given with the two properties
    /// naming where it lives and only with them. Editable for two hours.
    pub const DESCRIPTION_ID: &str = "descriptionId";
    /// The contract the description document lives in.
    pub const DESCRIPTION_CONTRACT_ID: &str = "descriptionContractId";
    /// The document type of the description document.
    pub const DESCRIPTION_DOCUMENT_TYPE: &str = "descriptionDocumentType";
    /// Where what is being decided is described or discussed; optional, required of a proposal
    /// that points at no description document. Editable for two hours.
    pub const URL: &str = "url";
    /// Whether anyone may enter choices; only the proposal's owner may when it is false.
    /// Never changes.
    pub const OPEN: &str = "open";
}

/// The properties of a choice.
pub mod choice_properties {
    /// The multiple choice proposal the choice answers.
    pub const PROPOSAL_ID: &str = "proposalId";
    /// The answer, in one line.
    pub const TITLE: &str = "title";
    /// What voting for the answer means; optional.
    pub const DESCRIPTION: &str = "description";
}

/// Whether a document of `document_type_name` in the contract `contract_id` is a choice: a
/// `choice` of the decision proposals contract. Its create joins the vote among the choices of
/// the proposal it answers, or opens it.
pub fn is_choice(contract_id: &Identifier, document_type_name: &str) -> bool {
    *contract_id == DECISION_PROPOSALS_CONTRACT_ID
        && document_type_name == CHOICE_DOCUMENT_TYPE_NAME
}

/// The vote among the choices of the multiple choice proposal `proposal_id`: the contest of
/// the `choice` type's `byProposal` index at that proposal, as a choice's create derives it
/// from its `proposalId`. The contested resource queries take it, and a masternode's vote
/// names it. Known from the proposal's id alone, whether or not a choice has opened the vote.
pub fn multiple_choice_proposal_vote_poll(
    proposal_id: Identifier,
) -> ContestedDocumentResourceVotePoll {
    ContestedDocumentResourceVotePoll {
        contract_id: DECISION_PROPOSALS_CONTRACT_ID,
        document_type_name: CHOICE_DOCUMENT_TYPE_NAME.to_string(),
        index_name: CHOICE_CONTEST_INDEX_NAME.to_string(),
        index_values: vec![Value::Identifier(proposal_id.to_buffer())],
    }
}

/// A masternode's answer in the vote among a multiple choice proposal's choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultipleChoiceVoteChoice {
    /// For the choice with this document id. The id of the identity that entered the choice
    /// names no contender here, and a vote carrying it is refused.
    Choice(Identifier),
    /// For no choice and against none.
    Abstain,
    /// None of the choices: when these votes outnumber the leading choice, no choice is kept
    /// and none can be entered for the proposal again.
    NoneOfTheChoices,
}

impl From<MultipleChoiceVoteChoice> for ResourceVoteChoice {
    /// The resource vote choice the contest takes: a choice's document id travels where a
    /// contest among identities carries an identity id, and none of the choices is a lock.
    fn from(vote_choice: MultipleChoiceVoteChoice) -> Self {
        match vote_choice {
            MultipleChoiceVoteChoice::Choice(choice_id) => {
                ResourceVoteChoice::TowardsIdentity(choice_id)
            }
            MultipleChoiceVoteChoice::Abstain => ResourceVoteChoice::Abstain,
            MultipleChoiceVoteChoice::NoneOfTheChoices => ResourceVoteChoice::Lock,
        }
    }
}

/// A masternode's vote `vote_choice` in the vote among the choices of the multiple choice
/// proposal `proposal_id`, to sign as a masternode vote transition.
pub fn multiple_choice_proposal_vote(
    proposal_id: Identifier,
    vote_choice: MultipleChoiceVoteChoice,
) -> Vote {
    Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
        vote_poll: VotePoll::ContestedDocumentResourceVotePoll(multiple_choice_proposal_vote_poll(
            proposal_id,
        )),
        resource_vote_choice: vote_choice.into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_proposal::{
        is_multiple_choice_proposal, MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
    };

    #[test]
    fn should_recognize_only_the_choice_of_the_decision_proposals_contract() {
        assert!(is_choice(
            &DECISION_PROPOSALS_CONTRACT_ID,
            CHOICE_DOCUMENT_TYPE_NAME
        ));
        assert!(!is_choice(
            &DECISION_PROPOSALS_CONTRACT_ID,
            MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME
        ));
        assert!(!is_choice(
            &Identifier::new([1; 32]),
            CHOICE_DOCUMENT_TYPE_NAME
        ));
        assert!(!is_multiple_choice_proposal(
            &DECISION_PROPOSALS_CONTRACT_ID,
            CHOICE_DOCUMENT_TYPE_NAME
        ));
    }

    #[test]
    fn should_vote_towards_a_choice_by_its_document_id() {
        let proposal_id = Identifier::new([7; 32]);
        let choice_id = Identifier::new([9; 32]);
        let answers = [
            (
                MultipleChoiceVoteChoice::Choice(choice_id),
                ResourceVoteChoice::TowardsIdentity(choice_id),
            ),
            (
                MultipleChoiceVoteChoice::Abstain,
                ResourceVoteChoice::Abstain,
            ),
            (
                MultipleChoiceVoteChoice::NoneOfTheChoices,
                ResourceVoteChoice::Lock,
            ),
        ];
        for (vote_choice, resource_vote_choice) in answers {
            assert_eq!(
                multiple_choice_proposal_vote(proposal_id, vote_choice),
                Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                    vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                        multiple_choice_proposal_vote_poll(proposal_id)
                    ),
                    resource_vote_choice,
                })),
                "{vote_choice:?}"
            );
        }
    }

    #[cfg(feature = "decision-proposals-contract")]
    #[test]
    fn should_name_the_types_and_properties_as_the_contract_crate_does() {
        use data_contracts::decision_proposals_contract::v1::document_types::{
            choice, multiple_choice_proposal,
        };

        assert_eq!(
            MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
            multiple_choice_proposal::NAME
        );
        assert_eq!(
            proposal_properties::TITLE,
            multiple_choice_proposal::properties::TITLE
        );
        assert_eq!(
            proposal_properties::DESCRIPTION_ID,
            multiple_choice_proposal::properties::DESCRIPTION_ID
        );
        assert_eq!(
            proposal_properties::DESCRIPTION_CONTRACT_ID,
            multiple_choice_proposal::properties::DESCRIPTION_CONTRACT_ID
        );
        assert_eq!(
            proposal_properties::DESCRIPTION_DOCUMENT_TYPE,
            multiple_choice_proposal::properties::DESCRIPTION_DOCUMENT_TYPE
        );
        assert_eq!(
            proposal_properties::URL,
            multiple_choice_proposal::properties::URL
        );
        assert_eq!(
            proposal_properties::OPEN,
            multiple_choice_proposal::properties::OPEN
        );
        assert_eq!(CHOICE_DOCUMENT_TYPE_NAME, choice::NAME);
        assert_eq!(CHOICE_CONTEST_INDEX_NAME, choice::indexes::BY_PROPOSAL);
        assert_eq!(
            choice_properties::PROPOSAL_ID,
            choice::properties::PROPOSAL_ID
        );
        assert_eq!(choice_properties::TITLE, choice::properties::TITLE);
        assert_eq!(
            choice_properties::DESCRIPTION,
            choice::properties::DESCRIPTION
        );
    }

    /// The poll named from the proposal's id alone is the one a choice's create derives from
    /// its `proposalId`, in whichever form the create writes the id.
    #[cfg(feature = "decision-proposals-contract")]
    #[test]
    fn should_name_the_vote_a_choice_joins_by_the_proposal_id_alone() {
        use crate::data_contract::accessors::v0::DataContractV0Getters;
        use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
        use crate::document::{Document, DocumentV0};
        use crate::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use platform_version::version::PlatformVersion;
        use std::collections::BTreeMap;

        let platform_version = PlatformVersion::latest();
        let contract =
            load_system_data_contract(SystemDataContract::DecisionProposals, platform_version)
                .expect("the contract loads at the latest version");
        let document_type = contract
            .document_type_for_name(CHOICE_DOCUMENT_TYPE_NAME)
            .expect("the choice type");
        let proposal_id = Identifier::new([7; 32]);
        let forms = [
            Value::Identifier(proposal_id.to_buffer()),
            Value::Bytes32(proposal_id.to_buffer()),
            Value::Bytes(proposal_id.to_vec()),
        ];
        for proposal_id_value in forms {
            let choice: Document = DocumentV0 {
                properties: BTreeMap::from([
                    (
                        choice_properties::PROPOSAL_ID.to_string(),
                        proposal_id_value.clone(),
                    ),
                    (
                        choice_properties::TITLE.to_string(),
                        Value::Text("Double it".to_string()),
                    ),
                ]),
                ..Default::default()
            }
            .into();
            assert_eq!(
                document_type
                    .contested_vote_poll_for_document(&choice, platform_version)
                    .expect("active"),
                Some(multiple_choice_proposal_vote_poll(proposal_id)),
                "{proposal_id_value:?}"
            );
        }
    }
}
