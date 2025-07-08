mod fields;
pub mod token_base_transition_accessors;
pub mod v0;
mod v0_methods;

#[cfg(any(
    feature = "state-transition-value-conversion",
    feature = "state-transition-json-conversion"
))]
use crate::data_contract::DataContract;
use crate::state_transition::batch_transition::document_base_transition::v0::DocumentTransitionObjectLike;
use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
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
pub enum TokenBaseTransition {
    #[display("V0({})", "_0")]
    V0(TokenBaseTransitionV0),
}

impl Default for TokenBaseTransition {
    fn default() -> Self {
        TokenBaseTransition::V0(TokenBaseTransitionV0::default()) // since only v0
    }
}

impl DocumentTransitionObjectLike for TokenBaseTransition {
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
