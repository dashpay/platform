mod from_document;
pub mod v0_methods;

use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;

use bincode::{Decode, Encode};
use derive_more::Display;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display("Base: {}", "base")]
pub struct DocumentDeleteTransitionV0 {
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: DocumentBaseTransition,
}
