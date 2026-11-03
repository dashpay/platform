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
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/decision-proposals-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
