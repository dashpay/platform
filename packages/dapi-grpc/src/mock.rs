//! Mocking support for messages.
//!
//! Contains [Mockable] trait that should be implemented by any object that can be used in the DAPI.
//!
//! Note that this trait is defined even if mocks are not supported, but it should always return `None` on serialization.

#[cfg(feature = "mocks")]
pub mod serde_mockable;

use tonic::Streaming;

/// Mocking support for messages.
///
/// This trait should be implemented by any object that can be used in the DAPI.
///
/// We use serde_json to serialize/deserialize messages.
//  TODO: Move to a different crate where it can be easily shared by dapi-grpc, dash-platform-sdk, and rs-dapi-client.
pub trait Mockable
where
    Self: std::marker::Sized,
{
    /// Serialize the message to bytes for mocking purposes.
    ///
    /// Returns None if the message is not serializable or mocking is disabled.
    ///
    /// # Panics
    ///
    /// Panics on any error.
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        None
    }
    /// Deserialize the message serialized with [mock_serialize()].
    ///
    /// Returns None if the message is not serializable or mocking is disabled.
    ///
    /// # Panics
    ///
    /// Panics on any error.
    fn mock_deserialize(_data: &[u8]) -> Option<Self> {
        None
    }
}
#[cfg(feature = "mocks")]
#[derive(serde::Serialize, serde::Deserialize)]
enum SerializableResult {
    Ok(Vec<u8>),
    Err(Vec<u8>),
}
impl<T, E> Mockable for Result<T, E>
where
    T: Mockable,
    E: Mockable,
{
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        let serializable = match self {
            Ok(value) => SerializableResult::Ok(value.mock_serialize()?),
            Err(error) => SerializableResult::Err(error.mock_serialize()?),
        };
        serde_json::to_vec(&serializable).ok()
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let deser: SerializableResult =
            serde_json::from_slice(data).expect("unable to deserialize mock data");
        Some(match deser {
            SerializableResult::Ok(data) => Ok(T::mock_deserialize(&data)?),
            SerializableResult::Err(data) => Err(E::mock_deserialize(&data)?),
        })
    }
}

impl<T: Mockable> Mockable for Option<T> {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        self.as_ref().and_then(|value| value.mock_serialize())
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        T::mock_deserialize(data).map(Some)
    }
}

impl Mockable for Vec<u8> {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        serde_json::to_vec(self).ok()
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        serde_json::from_slice(data).ok()
    }
}
#[cfg(feature = "mocks")]
#[derive(serde::Serialize, serde::Deserialize)]
struct MockableStatus {
    code: i32,
    message: Vec<u8>,
}
impl Mockable for crate::tonic::Status {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        let mockable = MockableStatus {
            code: self.code().into(),
            message: self.message().as_bytes().to_vec(),
        };

        Some(serde_json::to_vec(&mockable).expect("unable to serialize tonic::Status"))
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        let MockableStatus { code, message } =
            serde_json::from_slice(data).expect("unable to deserialize tonic::Status");
        let message = std::str::from_utf8(&message).expect("invalid utf8 message in tonic::Status");
        Some(Self::new(code.into(), message))
    }
}

/// Mocking of gRPC streaming responses is not supported.
///
/// This will return `None` on serialization,
/// effectively disabling mocking of streaming responses.
impl<T: Mockable> Mockable for Streaming<T> {}
