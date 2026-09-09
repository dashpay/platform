use anyhow::bail;

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Eq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum GroupActionStatus {
    ActionActive,
    ActionClosed,
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for GroupActionStatus {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for GroupActionStatus {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::Value;
    use serde_json::json;

    // Externally tagged unit-only enum with `rename_all = "camelCase"`:
    // serializes as a bare string (`"actionActive"` / `"actionClosed"`).

    #[test]
    fn json_round_trip_action_active() {
        use crate::serialization::JsonConvertible;
        let original = GroupActionStatus::ActionActive;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("actionActive"));
        let recovered = GroupActionStatus::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_action_closed() {
        use crate::serialization::JsonConvertible;
        let original = GroupActionStatus::ActionClosed;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("actionClosed"));
        let recovered = GroupActionStatus::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_action_active() {
        use crate::serialization::ValueConvertible;
        let original = GroupActionStatus::ActionActive;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, Value::Text("actionActive".to_string()));
        let recovered = GroupActionStatus::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_action_closed() {
        use crate::serialization::ValueConvertible;
        let original = GroupActionStatus::ActionClosed;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, Value::Text("actionClosed".to_string()));
        let recovered = GroupActionStatus::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}

impl TryFrom<u8> for GroupActionStatus {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ActionActive),
            1 => Ok(Self::ActionClosed),
            value => bail!("unrecognized action status: {}", value),
        }
    }
}

impl TryFrom<i32> for GroupActionStatus {
    type Error = anyhow::Error;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ActionActive),
            1 => Ok(Self::ActionClosed),
            value => bail!("unrecognized action status: {}", value),
        }
    }
}
