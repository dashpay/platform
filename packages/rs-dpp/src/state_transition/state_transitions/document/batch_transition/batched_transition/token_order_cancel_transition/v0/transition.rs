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
#[display("Base: {base}, Order ID: {order_id}, Order Revision: {order_revision} credits")]
pub struct TokenOrderCancelTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub(super) base: TokenBaseTransition,
    /// Order ID to cancel
    pub(super) order_id: Identifier,
    /// Order revision to cancel
    pub(super) order_revision: Revision,
}
