use bincode::{Decode, DecodeUntrusted, Encode};
use std::fmt;

use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
use crate::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// A token transfer that can move a random amount (protocol version 15): every field of
/// [`TokenTransferTransitionV0`], and `random_minimum_amount`.
///
/// `amount` is the most the transfer moves, and what the sender must hold: the block that
/// carries the transfer takes it from the sender. With `random_minimum_amount` set, the amount
/// that reaches the recipient is drawn uniformly from `random_minimum_amount` to `amount`, both
/// included, from the block's random beacon once the block's transitions are accepted, and the
/// rest goes back to the sender. Without it the transfer moves `amount`, as version 0 does.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, DecodeUntrusted)]
// `json_safe_fields` auto-injects `json_safe_u64` on `amount`, `json_safe_option_u64` on
// `random_minimum_amount`, and `json_safe_option_encrypted_note` on the encrypted notes, as for
// version 0.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenTransferTransitionV1 {
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    /// The most the transfer moves, taken from the sender when the block executes it
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$amount"))]
    pub amount: u64,
    #[cfg_attr(feature = "serde-conversion", serde(rename = "recipientId"))]
    pub recipient_id: Identifier,
    /// The public note
    #[cfg_attr(feature = "serde-conversion", serde(rename = "publicNote"))]
    pub public_note: Option<String>,
    /// An optional shared encrypted note
    #[cfg_attr(feature = "serde-conversion", serde(rename = "sharedEncryptedNote"))]
    pub shared_encrypted_note: Option<SharedEncryptedNote>,
    /// An optional private encrypted note
    #[cfg_attr(feature = "serde-conversion", serde(rename = "privateEncryptedNote"))]
    pub private_encrypted_note: Option<PrivateEncryptedNote>,
    /// The least the transfer moves, when the amount is random: the recipient receives an
    /// amount drawn uniformly from this to `amount`. Structure validation requires it to be at
    /// least 1 and below `amount`.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$randomMinimumAmount"))]
    pub random_minimum_amount: Option<u64>,
}

impl fmt::Display for TokenTransferTransitionV1 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.random_minimum_amount {
            Some(minimum) => write!(
                f,
                "Base: {}, Amount: {} to {}, Recipient: {}",
                self.base, minimum, self.amount, self.recipient_id
            ),
            None => write!(
                f,
                "Base: {}, Amount: {}, Recipient: {}",
                self.base, self.amount, self.recipient_id
            ),
        }
    }
}

impl TokenTransferTransitionV1 {
    /// `v0` with `random_minimum_amount`.
    pub fn from_v0(v0: TokenTransferTransitionV0, random_minimum_amount: Option<u64>) -> Self {
        let TokenTransferTransitionV0 {
            base,
            amount,
            recipient_id,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
        } = v0;
        Self {
            base,
            amount,
            recipient_id,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
            random_minimum_amount,
        }
    }

    /// The version 0 fields, and the random minimum amount version 0 cannot hold.
    pub fn into_v0_and_random_minimum_amount(self) -> (TokenTransferTransitionV0, Option<u64>) {
        let TokenTransferTransitionV1 {
            base,
            amount,
            recipient_id,
            public_note,
            shared_encrypted_note,
            private_encrypted_note,
            random_minimum_amount,
        } = self;
        (
            TokenTransferTransitionV0 {
                base,
                amount,
                recipient_id,
                public_note,
                shared_encrypted_note,
                private_encrypted_note,
            },
            random_minimum_amount,
        )
    }
}

impl TokenBaseTransitionAccessors for TokenTransferTransitionV1 {
    fn base(&self) -> &TokenBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        self.base = base;
    }
}
