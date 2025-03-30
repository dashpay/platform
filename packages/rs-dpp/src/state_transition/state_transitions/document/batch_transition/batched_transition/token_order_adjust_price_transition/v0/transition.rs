use crate::fee::Credits;
use crate::prelude::Revision;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode_derive::{Decode, Encode};
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
#[display(
    "Base: {base}, Order ID: {order_id}, Order Revision: {order_revision}, Price: {token_price} credits"
)]
pub struct TokenOrderAdjustPriceTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub(super) base: TokenBaseTransition,
    /// Order ID
    pub(super) order_id: Identifier,
    /// Order revision
    pub(super) order_revision: Revision,
    /// New price for specified order and revision
    pub(super) token_price: Credits,
}
