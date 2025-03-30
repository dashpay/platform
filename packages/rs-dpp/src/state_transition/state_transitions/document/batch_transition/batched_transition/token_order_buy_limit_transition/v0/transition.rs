use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::state_transition::batch_transition::batched_transition::Entropy;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode_derive::{Decode, Encode};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenOrderBuyLimitTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub(super) base: TokenBaseTransition,
    /// Entropy generated to create order ID
    pub(super) order_id_entropy: Entropy,
    /// How many tokens to buy
    pub(super) token_amount: TokenAmount,
    /// What is the maximum price to pay for the tokens
    pub(super) token_max_price: Credits,
}

impl fmt::Display for TokenOrderBuyLimitTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Base: {}, Order ID Entropy: {}, Token Amount to Buy: {}, Max Price: {} credits",
            self.base,
            hex::encode(&self.order_id_entropy),
            self.token_amount,
            self.token_max_price
        )
    }
}
