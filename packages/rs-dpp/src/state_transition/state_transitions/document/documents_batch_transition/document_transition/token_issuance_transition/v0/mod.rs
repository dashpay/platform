pub mod v0_methods;

use crate::state_transition::documents_batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, Encode};
use derive_more::Display;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

mod property_names {
    pub const AMOUNT: &str = "$amount";
}
/// The Identifier fields in [`TokenIssuanceTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display("Base: {base}, Amount: {amount}")]
pub struct TokenIssuanceTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,

    /// How much should we issue
    pub amount: u64,
}