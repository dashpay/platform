use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use crate::state_transition::batch_transition::batched_transition::token_order_sell_limit_transition::v0::transition::TokenOrderSellLimitTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum TokenOrderSellLimitTransition {
    #[display("V0({})", "_0")]
    V0(TokenOrderSellLimitTransitionV0),
}

impl Default for TokenOrderSellLimitTransition {
    fn default() -> Self {
        TokenOrderSellLimitTransition::V0(TokenOrderSellLimitTransitionV0::default())
        // since only v0
    }
}
