pub use tenderdash_proto::serializers::bytes::{
    base64string, hexstring, option_base64string, vec_base64string,
};

/// Serialize using [ToString] and deserialize using [FromStr](std::str::FromStr) trait implementations.
pub mod from_to_string {
    use std::{fmt::Display, str::FromStr};

    use serde::{Deserialize, Serializer};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::de::Deserializer<'de>,
        T: FromStr,
        T::Err: Display,
    {
        use serde::de::Error;

        String::deserialize(deserializer).and_then(|string| {
            string
                .parse::<T>()
                .map_err(|err| Error::custom(err.to_string()))
        })
    }

    /// Serialize from T into string
    pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: ToString,
    {
        serializer.serialize_str(&value.to_string())
    }
}
