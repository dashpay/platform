//! Serde serialization and deserialization for Mockable objects.
//!
//! This module provides a custom serialization and deserialization implementation for Mockable objects.
//!
//! /// ## Example
///
/// ```rust
/// struct SomeObject {
///    field: u32,
/// }
///
/// impl dapi_grpc::mock::Mockable for SomeObject {
///   fn mock_serialize(&self) -> Option<Vec<u8>> {
///       Some(self.field.to_be_bytes().to_vec())
///   }
///
///   fn mock_deserialize(bytes: &[u8]) -> Option<Self> {
///      if bytes.len() != 4 {
///         return None;
///      }
///
///      Some(SomeObject {   
///         field: u32::from_be_bytes(bytes.try_into().expect("4 bytes")),
///      })
///   }
/// }
///
/// #[derive(serde::Serialize,serde::Deserialize)]
/// struct TestStruct {
///     #[serde(with="dapi_grpc::mock::serde_mockable")]
///     field: SomeObject,
/// }
/// ```
use super::Mockable;

use serde::{
    de::{self, Visitor},
    Deserializer, Serializer,
};
use serde_bytes::Deserialize;
use std::fmt;
use std::marker::PhantomData;

/// Serialize any Mockable object to bytes.
///
/// ## Example
///
/// `#[serde(with="dapi_grpc::mock::serde_mockable")]`
pub fn serialize<T: Mockable, S>(data: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match data.mock_serialize() {
        Some(bytes) => serializer.serialize_bytes(bytes.as_slice()),
        None => Err(serde::ser::Error::custom(
            "Mockable object is not serializable",
        )),
    }
}

struct MockableVisitor<T> {
    marker: PhantomData<T>,
}

impl<'de, T> Visitor<'de> for MockableVisitor<T>
where
    T: Mockable,
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a byte array")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::mock_deserialize(v).ok_or_else(|| E::custom("Failed to deserialize Mockable object"))
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let bytes = <Vec<u8>>::deserialize(de::value::SeqAccessDeserializer::new(seq))?;
        T::mock_deserialize(&bytes).ok_or_else(|| {
            serde::de::Error::custom("Failed to deserialize Mockable object from seq")
        })
    }
}

/// Deserialize any Mockable object from bytes.
///
/// ## Example
///
/// `#[serde(with="dapi_grpc::mock::serde_mockable")]`
pub fn deserialize<'de, T: Mockable, D>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_bytes(MockableVisitor {
        marker: PhantomData,
    })
}
