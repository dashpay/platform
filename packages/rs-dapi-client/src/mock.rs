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
    transport::TransportRequest, DapiClientError, DapiRequestExecutor, ExecutionError,
    ExecutionResponse, ExecutionResult, RequestSettings,
};
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::async_trait;
use hex::ToHex;
use sha2::Digest;
use std::{
    any::type_name,
    collections::HashMap,
    fmt::{Debug, Display},
};

/// Mock DAPI client.
///
/// This is a mock implmeneation of [DapiRequestExecutor] that can be used for testing.
///
/// See `tests/mock_dapi_client.rs` for an example.
#[derive(Default, Debug)]
pub struct MockDapiClient {
    expectations: Expectations,
}
/// Result of executing a mock request
pub type MockResult<T> = ExecutionResult<<T as TransportRequest>::Response, DapiClientError>;

impl MockDapiClient {
    /// Create a new mock client
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new expectation for a request
    pub fn expect<R>(&mut self, request: &R, result: &MockResult<R>) -> Result<&mut Self, MockError>
    where
        R: TransportRequest + Mockable,
        R::Response: Mockable,
    {
        let key = self.expectations.add(request, result)?;

        tracing::trace!(
            %key,
            request_type = std::any::type_name::<R>(),
            response_type = std::any::type_name::<R::Response>(),
            "mock added expectation"
        );

        Ok(self)
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
    pub fn load<T, P: AsRef<std::path::Path>>(
        &mut self,
        file: P,
    ) -> Result<(T, MockResult<T>), std::io::Error>
    where
        T: TransportRequest + Mockable,
        T::Response: Mockable,
    {
        use crate::DumpData;

        let buf = std::fs::read(file)?;
        let data = DumpData::<T>::mock_deserialize(&buf).ok_or({
            std::io::Error::new(std::io::ErrorKind::InvalidData, "unable to parse json")
        })?;

        let (request, response) = data.deserialize();
        self.expect(&request, &response).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unable to add expectation: {}", e),
            )
        })?;
        Ok((request, response))
    }
}

#[async_trait]
impl DapiRequestExecutor for MockDapiClient {
    async fn execute<R: TransportRequest>(
        &self,
        request: R,
        _settings: RequestSettings,
    ) -> MockResult<R>
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

        if let Some(response) = response {
            response
        } else {
            let error = MockError::MockExpectationNotFound(format!(
                "unexpected mock request with key {}, use MockDapiClient::expect(): {:?}",
                key, request
            ));

            Err(ExecutionError {
                inner: DapiClientError::Mock(error),
                retries: 0,
                address: None,
            })
        }
    }
}

#[derive(Eq, Hash, PartialEq, PartialOrd, Ord, Clone, Debug)]
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
        let key = e.finalize().into();

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

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
/// Mock errors
pub enum MockError {
    #[error("mock expectation not found for request: {0}")]
    /// Expectation not found
    MockExpectationNotFound(String),

    #[error("expectation already defined for request: {0}")]
    /// Expectation already defined for request
    MockExpectationConflict(String),
}

#[derive(Debug)]
/// Wrapper that encapsulated serialized form of expected response for a request
struct ExpectedResult(Vec<u8>);

impl ExpectedResult {
    fn serialize<I: Mockable>(request: &I) -> Self {
        // We use json because bincode sometimes fail to deserialize
        Self(request.mock_serialize().expect("encode value"))
    }

    fn deserialize<O: Mockable>(&self) -> O {
        // We use json because bincode sometimes fail to deserialize
        O::mock_deserialize(&self.0).expect("deserialize value")
    }
}

#[derive(Default, Debug)]
/// Requests expected by a mock and their responses.
struct Expectations {
    expectations: HashMap<Key, ExpectedResult>,
}

impl Expectations {
    /// Add expected request and provide given response.
    ///
    /// If the expectation already exists, error is returned.
    pub fn add<I: Mockable + Debug, O: Mockable>(
        &mut self,
        request: &I,
        result: &O,
    ) -> Result<Key, MockError> {
        let key = Key::new(request);
        let value = ExpectedResult::serialize(result);

        if self.expectations.contains_key(&key) {
            return Err(MockError::MockExpectationConflict(format!(
                "expectation with key {} already defined for {} request",
                key,
                std::any::type_name::<I>(),
            )));
        }

        self.expectations.insert(key.clone(), value);

        Ok(key)
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

impl<R: Mockable> Mockable for ExecutionResponse<R> {
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        R::mock_serialize(&self.inner)
    }

    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        // TODO: We need serialize retries and address too
        R::mock_deserialize(data).map(|inner| ExecutionResponse {
            inner,
            retries: 0,
            address: "http://127.0.0.1:9000"
                .parse()
                .expect("failed to parse address"),
        })
    }
}

impl<E: Mockable> Mockable for ExecutionError<E> {
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        E::mock_serialize(&self.inner)
    }

    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        // TODO: We need serialize retries and address too
        E::mock_deserialize(data).map(|inner| ExecutionError {
            inner,
            retries: 0,
            address: None,
        })
    }
}

/// Create full wrapping object from inner type, using defaults for
/// fields that cannot be derived from the inner type.
pub trait FromInner<R>
where
    Self: Default,
{
    /// Create full wrapping object from inner type, using defaults for
    /// fields that cannot be derived from the inner type.
    ///
    /// Note this is imprecise conversion and should be avoided outside of tests.
    fn from_inner(inner: R) -> Self;
}

impl<R> FromInner<R> for ExecutionResponse<R>
where
    Self: Default,
{
    fn from_inner(inner: R) -> Self {
        Self {
            inner,
            ..Default::default()
        }
    }
}
