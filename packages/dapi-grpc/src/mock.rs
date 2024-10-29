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
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>>;

    /// Deserialize the message serialized with [mock_serialize()].
    ///
    /// Returns None if the message is not serializable or mocking is disabled.
    ///
    /// # Panics
    ///
    /// Panics on any error.
    #[cfg(feature = "mocks")]
    fn mock_deserialize(_data: &[u8]) -> Option<Self>;
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
        let data = match self {
            None => vec![0],
            Some(value) => {
                let mut data = vec![1]; // we return None if value does not support serialization
                let mut serialized = value.mock_serialize()?;
                data.append(&mut serialized);

                data
            }
        };

        Some(data)
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            panic!("empty data");
        }

        match data[0] {
            0 => Some(None),
            // Mind double Some - first says mock_deserialize is implemented, second is deserialized value
            1 => Some(Some(
                T::mock_deserialize(&data[1..]).expect("unable to deserialize Option<T>"),
            )),
            _ => panic!(
                "unsupported first byte for Option<T>::mock_deserialize: {:x}",
                data[0]
            ),
        }
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

impl<T: Mockable> Mockable for Vec<T> {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        let data: Vec<Vec<u8>> = self
            .iter()
            .map(|d| d.mock_serialize())
            .collect::<Option<Vec<Vec<u8>>>>()?;

        Some(serde_json::to_vec(&data).expect("unable to serialize Vec<T>"))
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        let data: Vec<Vec<u8>> =
            serde_json::from_slice(data).expect("unable to deserialize Vec<T>");

        data.into_iter()
            .map(|d| T::mock_deserialize(&d))
            .collect::<Option<Vec<T>>>()
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
impl<T: Mockable> Mockable for Streaming<T> {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        unimplemented!("mocking of streaming messages is not supported")
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(_data: &[u8]) -> Option<Self> {
        unimplemented!("mocking of streaming messages is not supported")
    }
}

/// Mocking of primitive types - just serialize them as little-endian bytes.
///
/// This is useful for mocking of messages that contain primitive types.
macro_rules! mockable_number {
    ($($t:ty),*) => {
        $(
            impl Mockable for $t {
                #[cfg(feature = "mocks")]
                fn mock_serialize(&self) -> Option<Vec<u8>> {
                    (*self).to_le_bytes().to_vec().mock_serialize()
                }

                #[cfg(feature = "mocks")]
                fn mock_deserialize(data: &[u8]) -> Option<Self> {
                    let data: Vec<u8> = Mockable::mock_deserialize(data)?;
                    Some(Self::from_le_bytes(
                        data.try_into().expect("invalid serialized data"),
                    ))
                }
            }
        )*
    };
}

// No `u8` as it would cause conflict between Vec<u8> and Vec<T: Mockable>  impls.
mockable_number!(usize, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128);
