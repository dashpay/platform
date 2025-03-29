use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use crate::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::v0::transition::TokenOrderBuyLimitTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum TokenOrderBuyLimitTransition {
    #[display("V0({})", "_0")]
    V0(TokenOrderBuyLimitTransitionV0),
}

impl Default for TokenOrderBuyLimitTransition {
    fn default() -> Self {
        TokenOrderBuyLimitTransition::V0(TokenOrderBuyLimitTransitionV0::default())
        // since only v0
    }
}
