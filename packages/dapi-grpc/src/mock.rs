//! Mocking support for messages.
//!
//! Contains [Mockable] trait that should be implemented by any object that can be used in the DAPI.
//!
//! Note that this trait is defined even if mocks are not supported, but it should always return `None` on serialization.

#[cfg(feature = "mocks")]
pub mod serde_mockable;

#[cfg(feature = "mocks")]
use tonic::metadata::{KeyAndValueRef, MetadataKey, MetadataValue};
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    binary_metadata: Vec<(String, Vec<u8>)>,
}
impl Mockable for crate::tonic::Status {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        let mut binary_metadata = Vec::new();
        for entry in self.metadata().iter() {
            if let KeyAndValueRef::Binary(key, value) = entry {
                let Ok(bytes) = value.to_bytes() else {
                    continue;
                };
                binary_metadata.push((key.to_string(), bytes.to_vec()));
            }
        }
        let mockable = MockableStatus {
            code: self.code().into(),
            message: self.message().as_bytes().to_vec(),
            binary_metadata,
        };

        Some(serde_json::to_vec(&mockable).expect("unable to serialize tonic::Status"))
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        let MockableStatus {
            code,
            message,
            binary_metadata,
        } = serde_json::from_slice(data).expect("unable to deserialize tonic::Status");
        let message = std::str::from_utf8(&message).expect("invalid utf8 message in tonic::Status");
        let mut status = Self::new(code.into(), message);
        for (key, value) in binary_metadata {
            status.metadata_mut().append_bin(
                MetadataKey::from_bytes(key.as_bytes()).ok()?,
                MetadataValue::from_bytes(&value),
            );
        }
        Some(status)
    }
}

/// Mocking of gRPC streaming responses is not supported.
///
/// This will return `None` on serialization,
/// effectively disabling mocking of streaming responses.
impl<T: Mockable> Mockable for Streaming<T> {}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::Mockable;
    use tonic::codegen::http::{HeaderMap, HeaderValue};
    use tonic::metadata::{MetadataMap, MetadataValue};
    use tonic::{Code, Status};

    #[test]
    fn should_preserve_binary_status_metadata_in_mock_recordings() {
        let mut status = Status::invalid_argument("consensus rejection");
        for bytes in [b"\x00\xff\x80".as_slice(), b"second value".as_slice()] {
            status.metadata_mut().append_bin(
                "dash-serialized-consensus-error-bin",
                MetadataValue::from_bytes(bytes),
            );
        }
        let restored = Status::mock_deserialize(&status.mock_serialize().unwrap()).unwrap();
        assert_eq!(restored.code(), Code::InvalidArgument);
        assert_eq!(restored.message(), status.message());
        let values: Vec<_> = restored
            .metadata()
            .get_all_bin("dash-serialized-consensus-error-bin")
            .iter()
            .map(|value| value.to_bytes().unwrap())
            .collect();
        assert_eq!(
            values,
            vec![b"\x00\xff\x80".as_slice(), b"second value".as_slice()]
        );
    }

    #[test]
    fn should_skip_malformed_binary_metadata_in_mock_recordings() {
        let mut headers = HeaderMap::new();
        headers.insert("malformed-bin", HeaderValue::from_static("!"));
        let mut status = Status::with_metadata(
            Code::InvalidArgument,
            "consensus rejection",
            MetadataMap::from_headers(headers),
        );
        let valid = MetadataValue::from_bytes(b"valid value");
        status.metadata_mut().insert_bin("valid-bin", valid.clone());

        let restored = Status::mock_deserialize(&status.mock_serialize().unwrap()).unwrap();
        assert_eq!(restored.code(), status.code());
        assert_eq!(restored.message(), status.message());
        assert!(restored.metadata().get_bin("malformed-bin").is_none());
        assert_eq!(restored.metadata().get_bin("valid-bin"), Some(&valid));
    }

    #[test]
    fn should_read_legacy_status_recordings_without_metadata() {
        let restored = Status::mock_deserialize(br#"{"code":3,"message":[111,108,100]}"#).unwrap();
        assert_eq!(restored.code(), Code::InvalidArgument);
        assert_eq!(restored.message(), "old");
        assert!(restored.metadata().is_empty());
    }
}
