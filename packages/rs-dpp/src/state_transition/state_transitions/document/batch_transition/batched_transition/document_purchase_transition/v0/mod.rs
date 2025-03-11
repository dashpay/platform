mod from_document;
pub mod v0_methods;

use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_base_transition::DocumentBaseTransition;

use bincode::{Decode, Encode};
use derive_more::Display;

use crate::balances::credits::Credits;
use crate::prelude::Revision;
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
pub struct DocumentPurchaseTransitionV0 {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: DocumentBaseTransition,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$revision")
    )]
    pub revision: Revision,
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "price"))]
    pub price: Credits,
}
