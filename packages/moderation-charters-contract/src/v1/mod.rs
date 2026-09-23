use crate::Error;
use serde_json::Value;

pub mod document_types {
    /// A team's application to moderate a contract, and once elected the terms it moderates
    /// under. Immutable and undeletable.
    pub mod charter {
        pub const NAME: &str = "charter";

        pub mod properties {
            pub const TARGET_CONTRACT_ID: &str = "targetContractId";
            pub const DESCRIPTION: &str = "description";
            pub const ABILITIES: &str = "abilities";
            pub const MEMBERS: &str = "members";
            pub const REASON_CODES: &str = "reasonCodes";
            pub const MODERATORS_SHARE: &str = "moderatorsShare";
            pub const SPLIT: &str = "split";

            /// The keys of the `abilities` object, each holding the power the ability needs.
            pub mod abilities {
                pub const DELETE_DOCUMENTS: &str = "deleteDocuments";
                pub const BAN: &str = "ban";
                pub const SUSPEND: &str = "suspend";
                pub const WARN: &str = "warn";
            }

            /// The keys of the `split` object, three percentages summing to 100.
            pub mod split {
                pub const LEADER: &str = "leader";
                pub const EQUAL: &str = "equal";
                pub const ACTIONS: &str = "actions";
            }
        }

        pub mod indexes {
            /// The contested unique index keyed by the target contract: a create on it is
            /// the team's application, and opens or joins the election.
            pub const BY_TARGET_CONTRACT: &str = "byTargetContract";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/moderation-charters-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
