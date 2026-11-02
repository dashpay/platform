pub mod from_document;
pub mod v2_methods;

use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::Display;

use crate::data_contract::document_type::action_fees::agreement::DocumentActionFeeAgreement;
use crate::identifier::Identifier;
use crate::prelude::IdentityNonce;
use crate::tokens::token_payment_info::TokenPaymentInfo;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// The base of a document transition from protocol version 14: version 1, and what the
/// transition agrees to pay in action fees.
#[derive(Debug, Clone, Encode, Decode, Default, PartialEq, Display, DecodeUntrusted)]
// See `DocumentBaseTransitionV0` for json_safe_fields rationale.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display(
    "ID: {}, Type: {}, Contract ID: {}",
    id,
    document_type_name,
    data_contract_id
)]
pub struct DocumentBaseTransitionV2 {
    /// The document ID
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$id"))]
    pub id: Identifier,
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$identityContractNonce"))]
    pub identity_contract_nonce: IdentityNonce,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$type"))]
    pub document_type_name: String,
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$dataContractId"))]
    pub data_contract_id: Identifier,
    /// An optional Token Payment Info
    #[cfg_attr(
        feature = "serde-conversion",
        serde(default, rename = "$tokenPaymentInfo")
    )]
    pub token_payment_info: Option<TokenPaymentInfo>,
    /// The action fees the transition agrees to pay. Required when the document type charges a
    /// fee for the action: the amounts must be the ones it declares.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(default, rename = "$actionFeeAgreement")
    )]
    pub action_fee_agreement: Option<DocumentActionFeeAgreement>,
}
