pub mod v0_methods;

use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, Encode};
use platform_value::Identifier;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// The Identifier fields in [`TokenUnfreezeTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenUnfreezeTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// The identity that we are freezing
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "frozenIdentityId")
    )]
    pub frozen_identity_id: Identifier,
    /// The public note
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "publicNote")
    )]
    pub public_note: Option<String>,
}

impl fmt::Display for TokenUnfreezeTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Format the base transition (assuming `TokenBaseTransition` implements Display)
        write!(f, "Base: {}, Froze: {}", self.base, self.frozen_identity_id)
    }
}
