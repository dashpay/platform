use serde::{Deserialize, Deserializer};

/// Custom deserializer that handles both string and numeric representations
/// This is useful for environment variables which are always strings but need to be parsed as numbers
pub fn from_str_or_number<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: serde::Deserialize<'de> + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;
    s.parse::<T>().map_err(Error::custom)
}

/// Custom deserializer for boolean values that handles both string and boolean representations
/// Accepts: "true", "false", "1", "0", "yes", "no" (case insensitive)
pub fn from_str_or_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => s.parse::<bool>().map_err(Error::custom),
    }
}
