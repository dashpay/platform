use std::fmt;
use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::batched_transition::Entropy;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode_derive::{Decode, Encode};
use platform_value::Identifier;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use crate::state_transition::batch_transition::batched_transition::token_order_buy_limit_transition::v0::transition::TokenOrderBuyLimitTransitionV0;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenOrderSellLimitTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// Entropy generated to create order ID
    pub order_id_entropy: Entropy,
    /// How many tokens to sell
    pub token_amount: TokenAmount,
    /// What is the minimum price to be paid for the tokens
    pub token_min_price: Credits,
}

impl fmt::Display for TokenOrderSellLimitTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Base: {}, Order ID Entropy: {}, Token Amount to Sell: {}, Min Price: {} credits",
            self.base,
            hex::encode(&self.order_id_entropy),
            self.token_amount,
            self.token_min_price
        )
    }
}
