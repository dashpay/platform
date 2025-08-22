pub mod v0_methods;

/// The Identifier fields in [`TokenConfigUpdateTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, Encode};
use derive_more::Display;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display("Base: {base}, change: {update_token_configuration_item}")]
pub struct TokenConfigUpdateTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// Updated token configuration item
    pub update_token_configuration_item: TokenConfigurationChangeItem,
    /// The public note
    pub public_note: Option<String>,
}
