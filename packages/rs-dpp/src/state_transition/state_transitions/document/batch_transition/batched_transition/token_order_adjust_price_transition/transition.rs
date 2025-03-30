use bincode::{Decode, Encode};
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use crate::state_transition::batch_transition::batched_transition::token_order_adjust_price_transition::v0::transition::TokenOrderAdjustPriceTransitionV0;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum TokenOrderAdjustPriceTransition {
    #[display("V0({})", "_0")]
    V0(TokenOrderAdjustPriceTransitionV0),
}

impl Default for TokenOrderAdjustPriceTransition {
    fn default() -> Self {
        TokenOrderAdjustPriceTransition::V0(TokenOrderAdjustPriceTransitionV0::default())
        // since only v0
    }
}
