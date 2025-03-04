mod from_document;
pub mod v0_methods;

use crate::state_transition::state_transitions::document::batch_transition::document_base_transition::DocumentBaseTransition;

use bincode::{Decode, Encode};
use derive_more::Display;

#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display("Base: {}", "base")]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct DocumentDeleteTransitionV0 {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: DocumentBaseTransition,
}
