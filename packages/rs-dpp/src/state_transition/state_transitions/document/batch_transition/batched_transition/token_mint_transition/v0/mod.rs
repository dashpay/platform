pub mod v0_methods;

use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use bincode::{Decode, Encode};
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

mod property_names {
    pub const AMOUNT: &str = "$amount";
}
/// The Identifier fields in [`TokenMintTransition`]
pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenMintTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "issuedToIdentityId")
    )]
    /// Who should we issue the token to? If this is not set then we issue to the identity set in
    /// contract settings. If such an operation is allowed.
    pub issued_to_identity_id: Option<Identifier>,

    /// How much should we issue
    pub amount: u64,
    /// The public note
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "publicNote")
    )]
    pub public_note: Option<String>,
}

impl fmt::Display for TokenMintTransitionV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Format the base transition (assuming `TokenBaseTransition` implements Display)
        write!(
            f,
            "Base: {}, Amount: {}, To: {}",
            self.base, // Assuming `TokenBaseTransition` implements `Display`
            self.amount,
            self.issued_to_identity_id
                .as_ref()
                .map_or("(Identity Set By Contract)".to_string(), |id| id
                    .to_string(Encoding::Base58))
        )
    }
}
