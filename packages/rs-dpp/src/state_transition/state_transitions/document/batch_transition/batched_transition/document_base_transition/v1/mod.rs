pub mod from_document;
pub mod v1_methods;

#[cfg(feature = "state-transition-value-conversion")]
use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use derive_more::Display;

#[cfg(feature = "state-transition-value-conversion")]
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::identifier::Identifier;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::batch_transition::document_base_transition::property_names;
use crate::tokens::token_payment_info::TokenPaymentInfo;
#[cfg(any(
    feature = "state-transition-json-conversion",
    feature = "state-transition-value-conversion"
))]
use crate::{data_contract::DataContract, errors::ProtocolError};
#[cfg(feature = "state-transition-value-conversion")]
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
#[cfg(feature = "state-transition-value-conversion")]
use platform_value::Value;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, Default, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display(
    "ID: {}, Type: {}, Contract ID: {}",
    "id",
    "document_type_name",
    "data_contract_id"
)]
pub struct DocumentBaseTransitionV1 {
    /// The document ID
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "$id"))]
    pub id: Identifier,
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$identityContractNonce")
    )]
    pub identity_contract_nonce: IdentityNonce,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "$type"))]
    pub document_type_name: String,
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$dataContractId")
    )]
    pub data_contract_id: Identifier,
    /// An optional Token Payment Info
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(default, rename = "$tokenPaymentInfo")
    )]
    pub token_payment_info: Option<TokenPaymentInfo>,
}

#[cfg(feature = "state-transition-value-conversion")]
impl DocumentBaseTransitionV1 {
    pub fn from_value_map_consume(
        map: &mut BTreeMap<String, Value>,
        data_contract: DataContract,
        identity_contract_nonce: IdentityNonce,
    ) -> Result<DocumentBaseTransitionV1, ProtocolError> {
        let inner_token_payment_info_map: Option<BTreeMap<String, Value>> = map
            .remove_optional_map_as_btree_map_keep_values_as_platform_value(
                property_names::TOKEN_PAYMENT_INFO,
            )
            .ok()
            .flatten();
        Ok(DocumentBaseTransitionV1 {
            id: Identifier::from(map.remove_hash256_bytes(property_names::ID)?),
            identity_contract_nonce,
            document_type_name: map.remove_string(property_names::DOCUMENT_TYPE)?,
            data_contract_id: Identifier::new(
                map.remove_optional_hash256_bytes(property_names::DATA_CONTRACT_ID)?
                    .unwrap_or(data_contract.id().to_buffer()),
            ),
            token_payment_info: inner_token_payment_info_map
                .map(|map| map.try_into())
                .transpose()?,
        })
    }
}
