use std::{fmt::Display, marker::PhantomData, str::FromStr};

use base64::Engine;
use serde::de::{Deserialize, SeqAccess, Visitor};

type BytesSeq = prost::alloc::vec::Vec<prost::alloc::vec::Vec<u8>>;

/// Deserialize sequence of base64-encoded bytes into [BytesSeq]
pub fn from_seq_base64<'de, D>(deserializer: D) -> Result<BytesSeq, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    struct SeqVisitor(PhantomData<fn() -> BytesSeq>);
    impl<'de> Visitor<'de> for SeqVisitor {
        type Value = BytesSeq;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("sequence of strings containing base64-encoded bytes")
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<BytesSeq, S::Error>
        where
            S: SeqAccess<'de>,
        {
            let mut ret: BytesSeq = BytesSeq::new();
            let b64 = base64::engine::general_purpose::STANDARD;

            while let Some(value) = seq.next_element::<String>()? {
                let decoded = b64
                    .decode(value)
                    .map_err(|err| serde::de::Error::custom(err.to_string()))?;
                ret.push(decoded);
            }

            Ok(ret)
        }
    }

    // Create the visitor and ask the deserializer to drive it. The
    // deserializer will call visitor.visit_seq() if a seq is present in
    // the input data.
    let visitor = SeqVisitor(Default::default());

    let d = deserializer.deserialize_seq::<SeqVisitor>(visitor)?;

    Ok(d)
}

pub fn from_base64<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: TryFrom<Vec<u8>>,
    T::Error: Display,
{
    use serde::de::Error;

    let b64 = base64::engine::general_purpose::STANDARD;

    String::deserialize(deserializer)
        .and_then(|string| {
            b64.decode(&string)
                .map_err(|err| Error::custom(err.to_string()))
        })
        .and_then(|bytes| T::try_from(bytes).map_err(|err| Error::custom(err.to_string())))
}

pub fn from_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
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

pub fn from_hex<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: TryFrom<Vec<u8>>,
    T::Error: Display,
{
    use serde::de::Error;

    String::deserialize(deserializer)
        .and_then(|string| hex::decode(&string).map_err(|err| Error::custom(err.to_string())))
        .and_then(|bytes| T::try_from(bytes).map_err(|err| Error::custom(err.to_string())))
}
