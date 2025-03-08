pub mod v0_methods;

/// The Identifier fields in [`TokenClaimTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;
use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, Encode};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenClaimTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// The type of distribution we are targeting
    pub distribution_type: TokenDistributionType,
    /// A public note, this will only get saved to the state if we are using a historical contract
    pub public_note: Option<String>,
}

impl fmt::Display for TokenClaimTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "TokenClaimTransitionV0 {{ base: {}, distribution_type: {}, public_note: {} }}",
            self.base, // Assuming TokenBaseTransition implements Display
            self.distribution_type,
            self.public_note.as_deref().unwrap_or("None")
        )
    }
}
