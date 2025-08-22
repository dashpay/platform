pub mod v0_methods;

use bincode::{Decode, Encode};
use derive_more::Display;

pub use super::super::token_base_transition::IDENTIFIER_FIELDS;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
use platform_value::Identifier;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

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
        serde(rename = "recipientId")
    )]
    pub recipient_id: Identifier,
    /// The public note
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "publicNote")
    )]
    pub public_note: Option<String>,
    /// An optional shared encrypted note
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "sharedEncryptedNote")
    )]
    pub shared_encrypted_note: Option<SharedEncryptedNote>,
    /// An optional private encrypted note
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "privateEncryptedNote")
    )]
    pub private_encrypted_note: Option<PrivateEncryptedNote>,
}
