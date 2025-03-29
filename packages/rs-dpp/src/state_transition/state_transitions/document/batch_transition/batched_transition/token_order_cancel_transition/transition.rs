use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use crate::state_transition::batch_transition::batched_transition::token_order_cancel_transition::v0::transition::TokenOrderCancelLimitTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum TokenOrderCancelLimitTransition {
    #[display("V0({})", "_0")]
    V0(TokenOrderCancelLimitTransitionV0),
}

impl Default for TokenOrderCancelLimitTransition {
    fn default() -> Self {
        TokenOrderCancelLimitTransition::V0(TokenOrderCancelLimitTransitionV0::default())
        // since only v0
    }
}
