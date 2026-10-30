use crate::Error;
use serde_json::Value;

pub mod document_types {
    /// A yes/no question put to the masternodes. Creating one opens its vote poll, which its
    /// owner funds; the poll's rule is read from the proposal. Immutable and undeletable, so
    /// masternodes vote on what the proposal said when it was created.
    pub mod proposal {
        pub const NAME: &str = "proposal";

        pub mod properties {
            pub const TITLE: &str = "title";
            pub const DESCRIPTION: &str = "description";
            pub const URL: &str = "url";
            pub const SUPERMAJORITY_NUMERATOR: &str = "supermajorityNumerator";
            pub const SUPERMAJORITY_DENOMINATOR: &str = "supermajorityDenominator";
            /// A fixed minimum voting power. A proposal sets this or `MINIMUM_SHARE`.
            pub const MINIMUM_VOTING_POWER: &str = "minimumVotingPower";
            /// A minimum voting power as a share of the masternode list's total when the poll
            /// closes. A proposal sets this or `MINIMUM_VOTING_POWER`.
            pub const MINIMUM_SHARE: &str = "minimumShare";
            pub const END_DATE: &str = "endDate";

            /// The keys of the `minimumShare` object.
            pub mod minimum_share {
                pub const NUMERATOR: &str = "numerator";
                pub const DENOMINATOR: &str = "denominator";
                /// One of `ROUNDINGS`.
                pub const ROUNDING: &str = "rounding";

                /// The values of `rounding`, as `VotingPowerRounding` names them in JSON.
                pub const ROUNDINGS: [&str; 4] = ["down", "up", "downPlusOne", "upMinusOne"];
            }
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
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/decision-proposals-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
