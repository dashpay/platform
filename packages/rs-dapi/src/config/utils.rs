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
