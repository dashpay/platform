use crate::Error;
use serde_json::Value;

pub mod document_types {
    /// A yes/no question put to the masternodes. Creating one opens its vote poll, which its
    /// owner funds; the poll's rule is read from the proposal and never changes. The owner may
    /// edit the title, description and url for a day, or a fifth of the time the proposal runs
    /// if that is shorter, and every version is kept. Undeletable.
    pub mod proposal {
        pub const NAME: &str = "proposal";

        pub mod properties {
            pub const TITLE: &str = "title";
            pub const DESCRIPTION: &str = "description";
            pub const URL: &str = "url";
            /// The proposal passes when yes is strictly more than this percent of yes plus no
            /// (50 to 80).
            pub const PASS_PERCENT: &str = "passPercent";
            /// The least yes plus no voting power the proposal needs, as a percent of the
            /// masternode list's total voting power when it closes, rounded up (0 to 50).
            pub const MINIMUM_TURNOUT_PERCENT: &str = "minimumTurnoutPercent";
            pub const END_DATE: &str = "endDate";
        }

        pub mod indexes {
            /// By end date, then creation time: the proposals closing soonest first.
            pub const BY_END_DATE: &str = "byEndDate";
            /// By owner, then creation time.
            pub const BY_OWNER: &str = "byOwner";
            /// By creation time: the newest proposals.
            pub const BY_CREATED_AT: &str = "byCreatedAt";
        }
    }

    /// A question put to the masternodes whose answers are the choices entered for it. Any
    /// identity may enter [`choice`]s when it is open, and only its owner when it is not; whoever
    /// may enter them may enter several. What is being decided is not text of the proposal:
    /// the proposal points at a document saying it, of a moderated document type of a contract
    /// whose moderators the masternodes elect, so that a description can be taken down. Its
    /// owner may edit the title and url, and point it at another description, for two hours
    /// after creating it, and every version is kept; whether it is open never changes.
    /// Undeletable.
    pub mod multiple_choice_proposal {
        pub const NAME: &str = "multipleChoiceProposal";

        pub mod properties {
            pub const TITLE: &str = "title";
            /// The id of the document saying what is being decided. Its document type is a
            /// moderated one holding a string or a byte array that a moderator's removal
            /// takes away and that its author writes.
            pub const DESCRIPTION_ID: &str = "descriptionId";
            /// The id of the contract the description document lives in, one declaring
            /// elected moderation.
            pub const DESCRIPTION_CONTRACT_ID: &str = "descriptionContractId";
            /// The name of the description document's document type.
            pub const DESCRIPTION_DOCUMENT_TYPE: &str = "descriptionDocumentType";
            pub const URL: &str = "url";
            /// Whether anyone may enter choices. Only the proposal's owner may when it is
            /// false.
            pub const OPEN: &str = "open";
        }

        pub mod indexes {
            /// By owner, then creation time.
            pub const BY_OWNER: &str = "byOwner";
            /// By creation time: the newest proposals.
            pub const BY_CREATED_AT: &str = "byCreatedAt";
        }
    }

    /// An answer to a multiple choice proposal. The choices entered for one proposal contend
    /// in a masternode vote on the contested `byProposal` index: entering the first opens the
    /// vote and a later one joins it. Each choice is a contender of its own, named by its
    /// document id, so an identity may enter several, each paying the contest fund. Anyone
    /// enters choices for an open proposal, and only its owner for one that is not. The
    /// choice with the most votes is kept, unless the lock votes (none of the choices)
    /// outnumber it. Immutable and undeletable.
    pub mod choice {
        pub const NAME: &str = "choice";

        pub mod rules {
            /// The `propertyConstraints` rule holding the choices of a proposal that is not
            /// open to its owner.
            pub const CLOSED_PROPOSAL_TAKES_ITS_OWNERS_CHOICES: &str =
                "closedProposalTakesItsOwnersChoices";
        }

        pub mod properties {
            /// The id of the multiple choice proposal the choice answers.
            pub const PROPOSAL_ID: &str = "proposalId";
            pub const TITLE: &str = "title";
            pub const DESCRIPTION: &str = "description";
        }

        pub mod indexes {
            /// The contested unique index the vote among a proposal's choices runs on.
            pub const BY_PROPOSAL: &str = "byProposal";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/decision-proposals-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
