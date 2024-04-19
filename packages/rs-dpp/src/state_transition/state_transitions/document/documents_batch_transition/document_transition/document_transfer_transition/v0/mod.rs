mod from_document;
pub mod v0_methods;

use crate::prelude::Revision;
use bincode::{Decode, Encode};
use derive_more::Display;

use platform_value::Identifier;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

pub use super::super::document_base_transition::IDENTIFIER_FIELDS;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;

mod property_names {
    pub const REVISION: &str = "$revision";

    pub const RECIPIENT_OWNER_ID: &str = "recipientOwnerId";
}

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display(
    fmt = "Base: {}, Revision: {}, Recipient: {:?}",
    "base",
    "revision",
    "recipient_owner_id"
)]
pub struct DocumentTransferTransitionV0 {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: DocumentBaseTransition,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$revision")
    )]
    pub revision: Revision,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "recipientOwnerId")
    )]
    pub recipient_owner_id: Identifier,
}
