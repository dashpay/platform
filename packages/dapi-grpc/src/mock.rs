//! Mocking support for messages.
//!
//! Contains [Mockable] trait that should be implemented by any object that can be used in the DAPI.
//!
//! Note that this trait is defined even if mocks are not supported, but it should always return `None` on serialization.
use tonic::Streaming;

/// Mocking support for messages.
///
/// This trait should be implemented by any object that can be used in the DAPI.
///
/// We use serde_json to serialize/deserialize messages.
//  TODO: Move to a different crate where it can be easily shared by dapi-grpc, dash-sdk, and rs-dapi-client.
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

/// Mocking of gRPC streaming responses is not supported.
///
/// This will return `None` on serialization,
/// effectively disabling mocking of streaming responses.
impl<T: Mockable> Mockable for Streaming<T> {}
