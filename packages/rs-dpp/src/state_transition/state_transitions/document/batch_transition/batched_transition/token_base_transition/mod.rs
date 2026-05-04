mod fields;
pub mod token_base_transition_accessors;
pub mod v0;
mod v0_methods;

#[cfg(any(feature = "value-conversion", feature = "json-conversion"))]
use crate::data_contract::DataContract;
use crate::state_transition::batch_transition::document_base_transition::v0::DocumentTransitionObjectLike;
use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
#[cfg(any(feature = "value-conversion", feature = "json-conversion"))]
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::{Display, From};
pub use fields::*;
#[cfg(any(feature = "value-conversion", feature = "json-conversion"))]
use platform_value::Value;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "json-conversion")]
use serde_json::Value as JsonValue;
#[cfg(feature = "value-conversion")]
use std::collections::BTreeMap;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum TokenBaseTransition {
    #[display("V0({})", "_0")]
    V0(TokenBaseTransitionV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenBaseTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenBaseTransition {}

impl Default for TokenBaseTransition {
    fn default() -> Self {
        TokenBaseTransition::V0(TokenBaseTransitionV0::default()) // since only v0
    }
}

impl DocumentTransitionObjectLike for TokenBaseTransition {
    #[cfg(feature = "json-conversion")]
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
    #[cfg(feature = "value-conversion")]
    fn from_object(
        raw_transition: Value,
        _data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::from_value(raw_transition).map_err(ProtocolError::ValueError)
    }
    #[cfg(feature = "value-conversion")]
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

    #[cfg(feature = "value-conversion")]
    fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }
    #[cfg(feature = "value-conversion")]
    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let value = platform_value::to_value(self)?;
        value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "json-conversion")]
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "value-conversion")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_value_map()?.into())
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::state_transition::batch_transition::batched_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use platform_value::{platform_value, Identifier};
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    pub(super) fn fixture() -> TokenBaseTransition {
        TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 13,
            token_contract_position: 2,
            data_contract_id: Identifier::new([0xa1; 32]),
            token_id: Identifier::new([0xb2; 32]),
            using_group_info: None,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        // `TokenBaseTransition` has two `to_json`/`from_json`-style impls
        // (one from `DocumentTransitionObjectLike`, one from
        // `JsonConvertible`); use fully-qualified syntax to disambiguate.
        let json = <TokenBaseTransition as JsonConvertible>::to_json(&original).expect("to_json");
        // Externally-tagged enum: outer `V0`. Note the hyphenated rename
        // `$identity-contract-nonce` (not camelCase) is intentional here —
        // it's the explicit `serde(rename = "$identity-contract-nonce")` on
        // the field. `$tokenContractPosition` is `u16`; JSON erases that
        // size — see the Value-path assertion for `2u16`.
        // `using_group_info` is `Option<GroupStateTransitionInfo>` flattened;
        // when `None`, it contributes no keys to the wire shape.
        assert_eq!(
            json,
            json!({
                "V0": {
                    "$identity-contract-nonce": 13,
                    "$tokenContractPosition": 2,
                    "$dataContractId": Identifier::new([0xa1; 32]),
                    "$tokenId": Identifier::new([0xb2; 32]),
                }
            })
        );
        let recovered =
            <TokenBaseTransition as JsonConvertible>::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        // Same disambiguation as the JSON test above; both `to_object` and
        // `from_object` are provided by two overlapping traits.
        let value =
            <TokenBaseTransition as ValueConvertible>::to_object(&original).expect("to_object");
        // `13u64`: `IdentityNonce` is a `u64` alias. `2u16`:
        // `token_contract_position` is `u16`; explicit suffix locks in
        // `Value::U16`. `Identifier`s interpolate via `Serialize` ->
        // `Value::Identifier`.
        assert_eq!(
            value,
            platform_value!({
                "V0": {
                    "$identity-contract-nonce": 13u64,
                    "$tokenContractPosition": 2u16,
                    "$dataContractId": Identifier::new([0xa1; 32]),
                    "$tokenId": Identifier::new([0xb2; 32]),
                }
            })
        );
        let recovered =
            <TokenBaseTransition as ValueConvertible>::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
