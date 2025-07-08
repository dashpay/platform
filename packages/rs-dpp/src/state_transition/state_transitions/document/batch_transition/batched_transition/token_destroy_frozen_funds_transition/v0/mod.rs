pub mod v0_methods;

use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, Encode};
use derive_more::Display;
use platform_value::Identifier;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display("Base: {base}, Destroyed Account Identity ID: {frozen_identity_id}")]
pub struct TokenDestroyFrozenFundsTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// The identity id of the account whose balance should be destroyed
    pub frozen_identity_id: Identifier,
    /// The public note
    pub public_note: Option<String>,
}
