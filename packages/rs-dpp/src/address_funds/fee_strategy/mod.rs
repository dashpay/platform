pub mod deduct_fee_from_inputs_and_outputs;

pub use deduct_fee_from_inputs_and_outputs::FeeDeductionResult;

use bincode::{Decode, Encode};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, Hash)]
pub enum AddressFundsFeeStrategyStep {
    /// Deduct fee from a specific input address by index.
    /// The input must have remaining balance after its contribution to outputs.
    DeductFromInput(u16),
    /// Reduce a specific output by the fee amount.
    /// The output amount will be reduced to cover the fee.
    ReduceOutput(u16),
}

impl Default for AddressFundsFeeStrategyStep {
    fn default() -> Self {
        AddressFundsFeeStrategyStep::DeductFromInput(0)
    }
}

pub type AddressFundsFeeStrategy = Vec<AddressFundsFeeStrategyStep>;

// Custom serde impls so JSON / wasm Object output uses the standard
// `{ "type": "...", "index": N }` discriminator shape used elsewhere in
// the DPP wasm bindings. The bincode `Encode` / `Decode` derives above are
// the consensus-critical binary format and are intentionally untouched.
#[cfg(feature = "serde-conversion")]
impl Serialize for AddressFundsFeeStrategyStep {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("AddressFundsFeeStrategyStep", 2)?;
        match self {
            AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                state.serialize_field("type", "deductFromInput")?;
                state.serialize_field("index", index)?;
            }
            AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                state.serialize_field("type", "reduceOutput")?;
                state.serialize_field("index", index)?;
            }
        }
        state.end()
    }
}

#[cfg(feature = "serde-conversion")]
impl<'de> Deserialize<'de> for AddressFundsFeeStrategyStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct StepVisitor;

        impl<'de> Visitor<'de> for StepVisitor {
            type Value = AddressFundsFeeStrategyStep;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an AddressFundsFeeStrategyStep struct with type and index")
            }

            fn visit_map<V>(self, mut map: V) -> Result<AddressFundsFeeStrategyStep, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut step_type: Option<String> = None;
                let mut index: Option<u16> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => {
                            if step_type.is_some() {
                                return Err(de::Error::duplicate_field("type"));
                            }
                            step_type = Some(map.next_value()?);
                        }
                        "index" => {
                            if index.is_some() {
                                return Err(de::Error::duplicate_field("index"));
                            }
                            index = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let step_type = step_type.ok_or_else(|| de::Error::missing_field("type"))?;
                let index = index.ok_or_else(|| de::Error::missing_field("index"))?;

                match step_type.as_str() {
                    "deductFromInput" => Ok(AddressFundsFeeStrategyStep::DeductFromInput(index)),
                    "reduceOutput" => Ok(AddressFundsFeeStrategyStep::ReduceOutput(index)),
                    other => Err(de::Error::unknown_variant(
                        other,
                        &["deductFromInput", "reduceOutput"],
                    )),
                }
            }
        }

        deserializer.deserialize_struct(
            "AddressFundsFeeStrategyStep",
            &["type", "index"],
            StepVisitor,
        )
    }
}

#[cfg(all(test, feature = "serde-conversion"))]
mod tests {
    use super::*;

    #[test]
    fn deduct_from_input_serializes_with_type_and_index() {
        let step = AddressFundsFeeStrategyStep::DeductFromInput(7);
        let json = serde_json::to_value(&step).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "deductFromInput", "index": 7 })
        );
    }

    #[test]
    fn reduce_output_serializes_with_type_and_index() {
        let step = AddressFundsFeeStrategyStep::ReduceOutput(3);
        let json = serde_json::to_value(&step).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "reduceOutput", "index": 3 })
        );
    }

    #[test]
    fn deserializes_from_type_and_index() {
        let step: AddressFundsFeeStrategyStep =
            serde_json::from_value(serde_json::json!({ "type": "deductFromInput", "index": 9 }))
                .unwrap();
        assert_eq!(step, AddressFundsFeeStrategyStep::DeductFromInput(9));

        let step: AddressFundsFeeStrategyStep =
            serde_json::from_value(serde_json::json!({ "type": "reduceOutput", "index": 2 }))
                .unwrap();
        assert_eq!(step, AddressFundsFeeStrategyStep::ReduceOutput(2));
    }

    #[test]
    fn rejects_unknown_variant() {
        let result: Result<AddressFundsFeeStrategyStep, _> =
            serde_json::from_value(serde_json::json!({ "type": "burn", "index": 0 }));
        assert!(result.is_err());
    }

    #[test]
    fn round_trips_through_json() {
        for original in [
            AddressFundsFeeStrategyStep::DeductFromInput(0),
            AddressFundsFeeStrategyStep::DeductFromInput(42),
            AddressFundsFeeStrategyStep::ReduceOutput(0),
            AddressFundsFeeStrategyStep::ReduceOutput(42),
        ] {
            let json = serde_json::to_string(&original).unwrap();
            let restored: AddressFundsFeeStrategyStep = serde_json::from_str(&json).unwrap();
            assert_eq!(original, restored);
        }
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for AddressFundsFeeStrategyStep {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for AddressFundsFeeStrategyStep {}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests_address_funds_fee_strategy_step {
    use super::*;

    fn each_variant() -> [AddressFundsFeeStrategyStep; 2] {
        [
            AddressFundsFeeStrategyStep::DeductFromInput(0),
            AddressFundsFeeStrategyStep::ReduceOutput(1),
        ]
    }

    #[test]
    fn json_round_trip_each_variant() {
        use crate::serialization::JsonConvertible;
        for original in each_variant() {
            let json = original.to_json().expect("to_json");
            let recovered = AddressFundsFeeStrategyStep::from_json(json).expect("from_json");
            assert_eq!(original, recovered, "variant: {:?}", original);
        }
    }

    #[test]
    fn value_round_trip_each_variant() {
        use crate::serialization::ValueConvertible;
        for original in each_variant() {
            let value = original.to_object().expect("to_object");
            let recovered = AddressFundsFeeStrategyStep::from_object(value).expect("from_object");
            assert_eq!(original, recovered, "variant: {:?}", original);
        }
    }
}
