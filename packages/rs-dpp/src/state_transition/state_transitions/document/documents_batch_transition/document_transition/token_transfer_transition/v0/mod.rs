pub mod v0_methods;

use bincode::{Decode, Encode};
use derive_more::Display;

use platform_value::Identifier;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

pub use super::super::token_base_transition::IDENTIFIER_FIELDS;
use crate::state_transition::documents_batch_transition::token_base_transition::TokenBaseTransition;

mod property_names {
    pub const AMOUNT: &str = "$amount";
    pub const RECIPIENT_OWNER_ID: &str = "recipientOwnerId";
}

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display(
    "Base: {}, Amount: {}, Recipient: {:?}",
    "base",
    "amount",
    "recipient_owner_id"
)]
pub struct TokenTransferTransitionV0 {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$amount")
    )]
    pub amount: u64,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "recipientOwnerId")
    )]
    pub recipient_owner_id: Identifier,
}
