//! Mock implementation of rs-dapi-client for testing
//!
//! rs-dapi-client provides `mocks` feature that makes it possible to mock the transport layer.
//! Core concept of the mocks is a [MockDapiClient] that mimics [DapiClient](crate::DapiClient) behavior and allows
//! to define expectations for requests and responses using [`MockDapiClient::expect`] function.
//!
//! In order to use the mocking feature, you need to:
//!
//! 1. Define your requests and responses.
//! 2. Create a [MockDapiClient] and use it instead of [DapiClient](crate::DapiClient) in your tests.
//!
//! See `tests/mock_dapi_client.rs` for an example.

use crate::{
    transport::{TransportClient, TransportRequest},
    Dapi, DapiClientError, RequestSettings,
};
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::async_trait;
use hex::ToHex;
use sha2::Digest;
use std::{any::type_name, collections::HashMap, fmt::Display};

/// Mock DAPI client.
///
/// This is a mock implmeneation of [Dapi] that can be used for testing.
///
/// See `tests/mock_dapi_client.rs` for an example.
#[derive(Default)]
pub struct MockDapiClient {
    expectations: Expectations,
}
impl MockDapiClient {
    /// Create a new mock client
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new expectation for a request
    pub fn expect<R>(&mut self, request: &R, response: &R::Response) -> &mut Self
    where
        R: TransportRequest + Mockable,
        R::Response: Mockable,
    {
        let key = self.expectations.add(request, response);

        tracing::trace!(
            %key,
            request_type = std::any::type_name::<R>(),
            response_typr = std::any::type_name::<R::Response>(),
            "mock added expectation"
        );

        self
    }

    /// Load expectation from file.
    ///
    /// The file must contain JSON structure.
    /// See [DumpData](crate::DumpData) and [DapiClient::dump_dir()](crate::DapiClient::dump_dir()) more for details.
    ///
    /// # Panics
    ///
    /// Panics if the file can't be read or the data can't be parsed.
    #[cfg(feature = "dump")]
    pub fn load<T: TransportRequest, P: AsRef<std::path::Path>>(
        &mut self,
        file: P,
    ) -> Result<(T, T::Response), std::io::Error>
    where
        T: Mockable,
        T::Response: Mockable,
    {
        use crate::DumpData;

        let buf = std::fs::read(file)?;
        let data = DumpData::<T>::mock_deserialize(&buf).ok_or({
            std::io::Error::new(std::io::ErrorKind::InvalidData, "unable to parse json")
        })?;

        let (request, response) = data.deserialize();
        self.expect(&request, &response);
        Ok((request, response))
    }
}

#[async_trait]
impl Dapi for MockDapiClient {
    async fn execute<R: TransportRequest>(
        &mut self,
        request: R,
        _settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>>
    where
        R: Mockable,
        R::Response: Mockable,
    {
        let (key, response) = self.expectations.get(&request);

        tracing::trace!(
            %key,
            request_type = std::any::type_name::<R>(),
            response_type = std::any::type_name::<R::Response>(),
            response = ?response,
            "mock execute"
        );

        return if let Some(response) = response {
            Ok(response)
        } else {
            Err(DapiClientError::MockExpectationNotFound(format!(
                "unexpected mock request with key {}, use MockDapiClient::expect(): {:?}",
                key, request
            )))
        };
    }
}

#[derive(Eq, Hash, PartialEq, PartialOrd, Ord, Clone)]
/// Unique identifier of some serializable object (e.g. request) that can be used as a key in a hashmap.
pub struct Key([u8; 32]);

impl Key {
    /// Create a new expectation key from a serializable object (e.g. request).
    ///
    /// # Panics
    ///
    /// Panics if the object can't be serialized.
    pub fn new<S: Mockable>(request: &S) -> Self {
        Self::try_new(request).expect("unable to create a key")
    }

    /// Generate unique identifier of some serializable object (e.g. request).
    pub fn try_new<S: Mockable>(request: &S) -> Result<Self, std::io::Error> {
        // we append type name to the encoded value to make sure that different types
        // will have different keys
        let typ = type_name::<S>().replace('&', ""); //remove & from type name

        let mut encoded = S::mock_serialize(request).ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("mocking not supported for object of type {}", typ),
        ))?;
        encoded.append(&mut typ.as_bytes().to_vec());

        let mut e = sha2::Sha256::new();
        e.update(encoded);
        let key = e.finalize().try_into().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid key generated: {}", e),
            )
        })?;

        Ok(Self(key))
    }
}

impl ToHex for Key {
    fn encode_hex<T: std::iter::FromIterator<char>>(&self) -> T {
        self.0.encode_hex()
    }

    fn encode_hex_upper<T: std::iter::FromIterator<char>>(&self) -> T {
        self.0.encode_hex_upper()
    }
}
impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.encode_hex::<String>(), f)
    }
}

struct ExpectedResponse(Vec<u8>);

impl ExpectedResponse {
    fn serialize<I: Mockable>(request: &I) -> Self {
        // We use json because bincode sometimes fail to deserialize
        Self(request.mock_serialize().expect("encode value"))
    }

    fn deserialize<O: Mockable>(&self) -> O {
        // We use json because bincode sometimes fail to deserialize
        O::mock_deserialize(&self.0).expect("deserialize value")
    }
}

#[derive(Default)]
/// Requests expected by a mock and their responses.
struct Expectations {
    expectations: HashMap<Key, ExpectedResponse>,
}

impl Expectations {
    /// Add expected request and provide given response.
    ///
    /// If the expectation already exists, it will be silently replaced.
    pub fn add<I: Mockable, O: Mockable>(&mut self, request: &I, response: &O) -> Key {
        let key = Key::new(request);
        let value = ExpectedResponse::serialize(response);

        self.expectations.insert(key.clone(), value);

        key
    }

    /// Get the response for a given request.
    ///
    /// Returns `None` if the request has not been expected.
    pub fn get<I: Mockable, O: Mockable>(&self, request: &I) -> (Key, Option<O>) {
        let key = Key::new(request);

        let response = self.expectations.get(&key).and_then(|v| v.deserialize());

        (key, response)
    }
}
