use crate::utils::deserialize_string_or_number;
use serde::Deserializer;
use serde::de::{Error as DeError, Visitor};
use std::fmt;
use std::str::FromStr;

/// Custom deserializer that handles both string and numeric representations
/// This is useful for environment variables which are always strings but need to be parsed as numbers
pub fn from_str_or_number<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    deserialize_string_or_number(deserializer)
}

/// Custom deserializer for boolean values that handles both string and boolean representations
/// Accepts: "true", "false", "1", "0", "yes", "no" (case insensitive)
pub fn from_str_or_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    struct BoolOrStringVisitor;

    impl<'de> Visitor<'de> for BoolOrStringVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a boolean or a string representing a boolean")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
            Ok(value)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            parse_bool(value).map_err(E::custom)
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            self.visit_str(&value)
        }
    }

    fn parse_bool(input: &str) -> Result<bool, String> {
        let normalized = input.to_lowercase();
        match normalized.as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => input
                .parse::<bool>()
                .map_err(|err| format!("failed to parse bool '{}': {}", input, err)),
        }
    }

    deserializer.deserialize_any(BoolOrStringVisitor)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct BoolConfig {
        #[serde(deserialize_with = "super::from_str_or_bool")]
        value: bool,
    }

    #[derive(Deserialize)]
    struct NumConfig {
        #[serde(deserialize_with = "super::from_str_or_number")]
        value: u16,
    }

    // -- from_str_or_bool --

    #[test]
    fn from_str_or_bool_true_values() {
        for input in &["true", "1", "yes", "on", "TRUE", "Yes", "ON"] {
            let json = format!(r#"{{"value": "{}"}}"#, input);
            let c: BoolConfig =
                serde_json::from_str(&json).unwrap_or_else(|_| panic!("failed for {}", input));
            assert!(c.value, "expected true for input '{}'", input);
        }
    }

    #[test]
    fn from_str_or_bool_false_values() {
        for input in &["false", "0", "no", "off", "FALSE", "No", "OFF"] {
            let json = format!(r#"{{"value": "{}"}}"#, input);
            let c: BoolConfig =
                serde_json::from_str(&json).unwrap_or_else(|_| panic!("failed for {}", input));
            assert!(!c.value, "expected false for input '{}'", input);
        }
    }

    #[test]
    fn from_str_or_bool_native_bool() {
        let json = r#"{"value": true}"#;
        let c: BoolConfig = serde_json::from_str(json).expect("deserialize native true");
        assert!(c.value);

        let json = r#"{"value": false}"#;
        let c: BoolConfig = serde_json::from_str(json).expect("deserialize native false");
        assert!(!c.value);
    }

    #[test]
    fn from_str_or_bool_invalid_string() {
        let json = r#"{"value": "maybe"}"#;
        let result: Result<BoolConfig, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // -- from_str_or_number --

    #[test]
    fn from_str_or_number_from_string() {
        let json = r#"{"value": "3005"}"#;
        let c: NumConfig = serde_json::from_str(json).expect("deserialize string as number");
        assert_eq!(c.value, 3005);
    }

    #[test]
    fn from_str_or_number_from_number() {
        let json = r#"{"value": 9090}"#;
        let c: NumConfig = serde_json::from_str(json).expect("deserialize native number");
        assert_eq!(c.value, 9090);
    }

    #[test]
    fn from_str_or_number_invalid_string() {
        let json = r#"{"value": "not_a_number"}"#;
        let result: Result<NumConfig, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
