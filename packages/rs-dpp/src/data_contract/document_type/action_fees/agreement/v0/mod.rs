use crate::balances::credits::Credits;
use crate::data_contract::document_type::action_fees::agreement::AgreedFeeMultiplier;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::Display;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// What a document transition agrees to pay in action fees, version 0
#[derive(Debug, Clone, Copy, Encode, Decode, DecodeUntrusted, PartialEq, Eq, Display)]
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display(
    "Owner: {}, Moderators: {}, Fee Multiplier: {:?}",
    owner,
    moderators,
    fee_multiplier
)]
pub struct DocumentActionFeeAgreementV0 {
    /// The owner part the document type declares for the action, in credits. It must be the
    /// declared amount, before any fee multiplier.
    pub owner: Credits,
    /// The moderators part the document type declares for the action, in credits. It must be
    /// the declared amount, before any fee multiplier.
    pub moderators: Credits,
    /// The fee multiplier the signer knew and the increase they accept. Named for a fee priced
    /// by the fee multiplier, left out for a fixed one: the agreement must say how the document
    /// type prices the fee.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub fee_multiplier: Option<AgreedFeeMultiplier>,
}
