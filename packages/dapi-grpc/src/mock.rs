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
const RESULT_OK: u8 = 0;
#[cfg(feature = "mocks")]
const RESULT_ERR: u8 = 1;

impl<T, E> Mockable for Result<T, E>
where
    T: Mockable,
    E: Mockable,
{
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        let (typ, mut buf) = match self {
            Ok(value) => (RESULT_OK, value.mock_serialize()?),
            Err(error) => (RESULT_ERR, error.mock_serialize()?),
        };

        let mut result = vec![typ];
        result.append(&mut buf);
        Some(result)
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        use core::panic;

        if data.is_empty() {
            return None;
        }

        Some(match data[0] {
            RESULT_OK => Ok(T::mock_deserialize(&data[1..])?),
            RESULT_ERR => Err(E::mock_deserialize(&data[1..])?),
            d => panic!("cannot deserialize mock result: invalid result type {}", d),
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

impl Mockable for crate::tonic::Status {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        let code: i32 = self.code().into();
        let message = self.message();

        let mut buf = Vec::new();
        buf.extend(&code.to_be_bytes());
        buf.extend(message.as_bytes());

        Some(buf)
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        let code = i32::from_be_bytes(data[0..4].try_into().expect("invalid code"));
        let message = String::from_utf8_lossy(&data[2..]).to_string();

        Some(Self::new(code.into(), message))
    }
}

/// Mocking of gRPC streaming responses is not supported.
///
/// This will return `None` on serialization,
/// effectively disabling mocking of streaming responses.
impl<T: Mockable> Mockable for Streaming<T> {}
