pub mod document_base_transition_trait;
mod fields;
mod from_document;
pub mod v0;
mod v0_methods;
pub mod v1;
mod v1_methods;

#[cfg(any(
    feature = "state-transition-value-conversion",
    feature = "state-transition-json-conversion"
))]
use crate::data_contract::DataContract;
use crate::state_transition::batch_transition::document_base_transition::v0::{
    DocumentBaseTransitionV0, DocumentTransitionObjectLike,
};
use crate::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
#[cfg(any(
    feature = "state-transition-value-conversion",
    feature = "state-transition-json-conversion"
))]
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::{Display, From};
pub use fields::*;
#[cfg(any(
    feature = "state-transition-value-conversion",
    feature = "state-transition-json-conversion"
))]
use platform_value::Value;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "state-transition-json-conversion")]
use serde_json::Value as JsonValue;
#[cfg(feature = "state-transition-value-conversion")]
use std::collections::BTreeMap;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum DocumentBaseTransition {
    #[display("V0({})", "_0")]
    V0(DocumentBaseTransitionV0),
    #[display("V1({})", "_1")]
    V1(DocumentBaseTransitionV1),
}

impl Default for DocumentBaseTransition {
    fn default() -> Self {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0::default()) // since only v0
    }
}

impl DocumentTransitionObjectLike for DocumentBaseTransition {
    #[cfg(feature = "state-transition-json-conversion")]
    fn from_json_object(
        json_str: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let value: Value = json_str.into();
        Self::from_object(value, data_contract)
    }
    #[cfg(feature = "state-transition-value-conversion")]
    fn from_object(
        raw_transition: Value,
        _data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::from_value(raw_transition).map_err(ProtocolError::ValueError)
    }
    #[cfg(feature = "state-transition-value-conversion")]
    fn from_value_map(
        map: BTreeMap<String, Value>,
        _data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let value: Value = map.into();
        platform_value::from_value(value).map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "state-transition-value-conversion")]
    fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }
    #[cfg(feature = "state-transition-value-conversion")]
    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let value = platform_value::to_value(self)?;
        value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "state-transition-json-conversion")]
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "state-transition-value-conversion")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_value_map()?.into())
    }
}
