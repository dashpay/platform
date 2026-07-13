use crate::consensus::basic::data_contract::UnknownGasFeesPaidByError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::Display;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Display)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum GasFeesPaidBy {
    /// The user pays the gas fees
    #[default]
    DocumentOwner = 0,
    /// The contract owner pays the gas fees
    ContractOwner = 1,
    /// The user is stating his willingness to pay the gas fee if the Contract owner's balance is
    /// insufficient.
    PreferContractOwner = 2,
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for GasFeesPaidBy {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for GasFeesPaidBy {}

impl From<GasFeesPaidBy> for u8 {
    fn from(value: GasFeesPaidBy) -> Self {
        match value {
            GasFeesPaidBy::DocumentOwner => 0,
            GasFeesPaidBy::ContractOwner => 1,
            GasFeesPaidBy::PreferContractOwner => 2,
        }
    }
}

impl TryFrom<u8> for GasFeesPaidBy {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(GasFeesPaidBy::DocumentOwner),
            1 => Ok(GasFeesPaidBy::ContractOwner),
            2 => Ok(GasFeesPaidBy::PreferContractOwner),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownGasFeesPaidByError(
                    UnknownGasFeesPaidByError::new(vec![0, 1, 2], value as u64),
                ))
                .into(),
            )),
        }
    }
}

impl TryFrom<u64> for GasFeesPaidBy {
    type Error = ProtocolError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        u8::try_from(value)
            .map_err(|_| {
                ProtocolError::ConsensusError(
                    ConsensusError::BasicError(BasicError::UnknownGasFeesPaidByError(
                        UnknownGasFeesPaidByError::new(vec![0, 1, 2], value),
                    ))
                    .into(),
                )
            })?
            .try_into()
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    // `GasFeesPaidBy` is a unit-only enum without `rename_all`, so each variant
    // (de)serializes as its PascalCase Rust name in both JSON and platform_value.

    #[test]
    fn json_round_trip_document_owner() {
        use crate::serialization::JsonConvertible;
        let original = GasFeesPaidBy::DocumentOwner;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("DocumentOwner"));
        let recovered = GasFeesPaidBy::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_contract_owner() {
        use crate::serialization::JsonConvertible;
        let original = GasFeesPaidBy::ContractOwner;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("ContractOwner"));
        let recovered = GasFeesPaidBy::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_prefer_contract_owner() {
        use crate::serialization::JsonConvertible;
        let original = GasFeesPaidBy::PreferContractOwner;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("PreferContractOwner"));
        let recovered = GasFeesPaidBy::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_document_owner() {
        use crate::serialization::ValueConvertible;
        let original = GasFeesPaidBy::DocumentOwner;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("DocumentOwner"));
        let recovered = GasFeesPaidBy::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_contract_owner() {
        use crate::serialization::ValueConvertible;
        let original = GasFeesPaidBy::ContractOwner;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("ContractOwner"));
        let recovered = GasFeesPaidBy::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_prefer_contract_owner() {
        use crate::serialization::ValueConvertible;
        let original = GasFeesPaidBy::PreferContractOwner;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("PreferContractOwner"));
        let recovered = GasFeesPaidBy::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
