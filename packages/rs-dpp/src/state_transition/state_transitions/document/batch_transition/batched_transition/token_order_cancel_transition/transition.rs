use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use crate::state_transition::batch_transition::batched_transition::token_order_cancel_transition::v0::transition::TokenOrderCancelTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum TokenOrderCancelTransition {
    #[display("V0({})", "_0")]
    V0(TokenOrderCancelTransitionV0),
}

impl Default for TokenOrderCancelTransition {
    fn default() -> Self {
        TokenOrderCancelTransition::V0(TokenOrderCancelTransitionV0::default())
        // since only v0
    }
}
