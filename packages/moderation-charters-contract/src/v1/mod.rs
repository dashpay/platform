use crate::Error;
use serde_json::Value;

pub mod document_types {
    /// A ground for a moderation action, keyed by its owner and a three-letter code. Immutable
    /// and undeletable.
    pub mod reason {
        pub const NAME: &str = "reason";

        pub mod properties {
            pub const CODE: &str = "code";
            pub const LABEL: &str = "label";
            pub const DESCRIPTION: &str = "description";
        }

        pub mod indexes {
            /// Unique on the owner and the code.
            pub const BY_OWNER_CODE: &str = "byOwnerCode";
        }
    }

    /// A leader's proposal to moderate one contract. Immutable and undeletable.
    pub mod submitted_charter {
        pub const NAME: &str = "submittedCharter";

        pub mod properties {
            pub const TARGET_CONTRACT_ID: &str = "targetContractId";
            pub const DESCRIPTION: &str = "description";
            pub const REASONS: &str = "reasons";
            pub const MODERATORS_SHARE: &str = "moderatorsShare";
            pub const REWARD_SPLIT: &str = "rewardSplit";

            /// The keys of the `rewardSplit` object, three percentages summing to 100.
            pub mod reward_split {
                pub const LEADER: &str = "leader";
                pub const EQUAL: &str = "equal";
                pub const ACTIONS: &str = "actions";
            }
        }

        pub mod indexes {
            /// The proposals for a contract in filing order.
            pub const BY_TARGET_CONTRACT: &str = "byTargetContract";
            /// A leader's proposals.
            pub const BY_OWNER: &str = "byOwner";
        }
    }

    /// An identity's offer to serve on the team of a proposal, with a message only the
    /// leader can read. Immutable and undeletable.
    pub mod join_request {
        pub const NAME: &str = "joinRequest";

        pub mod properties {
            pub const SUBMITTED_CHARTER_ID: &str = "submittedCharterId";
            pub const RECIPIENT_ID: &str = "recipientId";
            pub const RECIPIENT_KEY_ID: &str = "recipientKeyId";
            pub const SENDER_KEY_ID: &str = "senderKeyId";
            pub const ENCRYPTED_MESSAGE: &str = "encryptedMessage";
        }

        pub mod indexes {
            /// Unique on the proposal and the owner: one offer per identity per proposal.
            pub const BY_SUBMITTED_CHARTER: &str = "bySubmittedCharter";
            /// An identity's offers.
            pub const BY_OWNER: &str = "byOwner";
        }
    }

    /// A proposal put to the vote with its team. Immutable and undeletable.
    pub mod elected_charter {
        pub const NAME: &str = "electedCharter";

        pub mod properties {
            pub const TARGET_CONTRACT_ID: &str = "targetContractId";
            pub const SUBMITTED_CHARTER_ID: &str = "submittedCharterId";
            pub const MEMBERS: &str = "members";
        }

        pub mod indexes {
            /// The contested unique index keyed by the target contract: a create on it opens
            /// or joins the contest for the target's seat, and only the winner is stored.
            pub const BY_TARGET_CONTRACT: &str = "byTargetContract";
            /// The elected charters of a proposal.
            pub const BY_SUBMITTED_CHARTER: &str = "bySubmittedCharter";
        }
    }

    /// A member the leader of a seated charter adds after the election. Immutable and
    /// undeletable.
    pub mod added_moderator {
        pub const NAME: &str = "addedModerator";

        pub mod properties {
            pub const ELECTED_CHARTER_ID: &str = "electedCharterId";
            pub const SUBMITTED_CHARTER_ID: &str = "submittedCharterId";
            pub const MEMBER_ID: &str = "memberId";
        }

        pub mod indexes {
            /// Unique on the charter and the member.
            pub const BY_ELECTED_CHARTER_MEMBER: &str = "byElectedCharterMember";
        }
    }

    /// A member the leader of a seated charter removes; final. Immutable and undeletable.
    pub mod removed_moderator {
        pub const NAME: &str = "removedModerator";

        pub mod properties {
            pub const ELECTED_CHARTER_ID: &str = "electedCharterId";
            pub const MEMBER_ID: &str = "memberId";
        }

        pub mod indexes {
            /// Unique on the charter and the member.
            pub const BY_ELECTED_CHARTER_MEMBER: &str = "byElectedCharterMember";
        }
    }

    /// A member asking to leave a seated team, with a message only the leader can read.
    /// Immutable; deletable, which withdraws the request.
    pub mod resignation_request {
        pub const NAME: &str = "resignationRequest";

        pub mod properties {
            pub const ELECTED_CHARTER_ID: &str = "electedCharterId";
            pub const RECIPIENT_ID: &str = "recipientId";
            pub const RECIPIENT_KEY_ID: &str = "recipientKeyId";
            pub const SENDER_KEY_ID: &str = "senderKeyId";
            pub const ENCRYPTED_MESSAGE: &str = "encryptedMessage";
        }

        pub mod indexes {
            /// Unique on the charter and the owner.
            pub const BY_ELECTED_CHARTER_OWNER: &str = "byElectedCharterOwner";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/moderation-charters-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
