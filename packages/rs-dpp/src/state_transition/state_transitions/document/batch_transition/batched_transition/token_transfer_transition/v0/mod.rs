pub mod v0_methods;

use bincode::{Decode, Encode};
use derive_more::Display;

pub use super::super::token_base_transition::IDENTIFIER_FIELDS;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

mod property_names {
    pub const AMOUNT: &str = "$amount";
    pub const RECIPIENT_OWNER_ID: &str = "recipientOwnerId";
}

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
// `#[json_safe_fields]` would require `JsonSafeFields` for the
// `Option<SharedEncryptedNote>` / `Option<PrivateEncryptedNote>` fields
// whose inner types are tuples containing `Vec<u8>` (e.g. `(u32, u32,
// Vec<u8>)`). Those tuples can't be cleanly auto-routed by the macro and
// would also need a custom serde helper to base64-encode the byte payload
// in JSON. Tracked as future work; for now apply the JS-safe u64 wrapper
// directly to the `amount` field via `serde(with)` instead of the macro.
#[cfg_attr(
    feature = "serde-conversion",
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
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: TokenBaseTransition,
    #[cfg_attr(
        feature = "serde-conversion",
        serde(rename = "$amount", with = "crate::serialization::json_safe_u64")
    )]
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
}
