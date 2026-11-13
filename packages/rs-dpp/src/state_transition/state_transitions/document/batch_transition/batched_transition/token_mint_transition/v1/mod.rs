pub mod v1_methods;

use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_mint_transition::v0::TokenMintTransitionV0;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// A token mint that can issue a random amount (protocol version 15): every field of
/// [`TokenMintTransitionV0`], and `random_minimum_amount`.
///
/// `amount` is the most the mint issues, and what the token's maximum supply must have room
/// for: the block that carries the mint adds it to the total supply. With
/// `random_minimum_amount` set, the amount the recipient receives is drawn uniformly from
/// `random_minimum_amount` to `amount`, both included, from the block's random beacon once the
/// block's transitions are accepted, and the rest is taken off the total supply again. Without
/// it the mint issues `amount`, as version 0 does.
///
/// A random mint names its recipient (`issued_to_identity_id`) and is not a group action:
/// structure validation refuses either, so the payout depends on nothing read after the block
/// executes it, and group members never weigh an amount nobody knows.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, DecodeUntrusted)]
// Auto-injects `json_safe_u64` on `amount` and `json_safe_option_u64` on
// `random_minimum_amount`.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenMintTransitionV1 {
    /// Document Base Transition
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    #[cfg_attr(feature = "serde-conversion", serde(rename = "issuedToIdentityId"))]
    /// Who should we issue the token to? If this is not set then we issue to the identity set in
    /// contract settings. If such an operation is allowed. A random mint must set it.
    pub issued_to_identity_id: Option<Identifier>,

    /// The most the mint issues, added to the total supply when the block executes it
    pub amount: u64,
    /// The public note
    #[cfg_attr(feature = "serde-conversion", serde(rename = "publicNote"))]
    pub public_note: Option<String>,
    /// The least the mint issues, when the amount is random: the recipient receives an amount
    /// drawn uniformly from this to `amount`. Structure validation requires it to be at least 1
    /// and below `amount`.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$randomMinimumAmount"))]
    pub random_minimum_amount: Option<u64>,
}

impl TokenMintTransitionV1 {
    /// `v0` with `random_minimum_amount`.
    pub fn from_v0(v0: TokenMintTransitionV0, random_minimum_amount: Option<u64>) -> Self {
        let TokenMintTransitionV0 {
            base,
            issued_to_identity_id,
            amount,
            public_note,
        } = v0;
        Self {
            base,
            issued_to_identity_id,
            amount,
            public_note,
            random_minimum_amount,
        }
    }

    /// The version 0 fields, and the random minimum amount version 0 cannot hold.
    pub fn into_v0_and_random_minimum_amount(self) -> (TokenMintTransitionV0, Option<u64>) {
        let TokenMintTransitionV1 {
            base,
            issued_to_identity_id,
            amount,
            public_note,
            random_minimum_amount,
        } = self;
        (
            TokenMintTransitionV0 {
                base,
                issued_to_identity_id,
                amount,
                public_note,
            },
            random_minimum_amount,
        )
    }
}

impl fmt::Display for TokenMintTransitionV1 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let recipient = self
            .issued_to_identity_id
            .as_ref()
            .map_or("(Identity Set By Contract)".to_string(), |id| {
                id.to_string(Encoding::Base58)
            });
        match self.random_minimum_amount {
            Some(minimum) => write!(
                f,
                "Base: {}, Amount: {} to {}, To: {}",
                self.base, minimum, self.amount, recipient
            ),
            None => write!(
                f,
                "Base: {}, Amount: {}, To: {}",
                self.base, self.amount, recipient
            ),
        }
    }
}
