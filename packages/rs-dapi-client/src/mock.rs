//! Mock implementation of rs-dapi-client for testing
//!
//! rs-dapi-client provides `mocks` feature that makes it possible to mock the transport layer.
//! Core concept of the mocks is a [MockDapiClient] that mimics [DapiClient] behavior and allows
//! to define expectations for requests and responses using [`MockDapiClient::expect`] function.
//!
//! In order to use the mocking feature, you need to:
//!
//! 1. Define your requests and responses.
//! 2. Create a [MockDapiClient] and use it instead of [DapiClient] in your tests.
//!
//! See `tests/mock_dapi_client.rs` for an example.

use std::collections::HashMap;
use tonic::async_trait;

use crate::{
    transport::{TransportClient, TransportRequest},
    Dapi, DapiClientError, RequestSettings,
};

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
        R: TransportRequest,
    {
        self.expectations.add(request, response);

        self
    }

    /// Load expectation from file.
    ///
    /// The file must contain JSON structure.
    /// See [DumpData](crate::DumpData) and [DapiClient::dump_dir()](crate::DapiClient::dump_dir()) more for details.
    #[cfg(feature = "mocks")]
    pub fn load<T: TransportRequest, P: AsRef<std::path::Path>>(
        &mut self,
        file: P,
    ) -> Result<(T, T::Response), std::io::Error>
    where
        T: for<'de> serde::Deserialize<'de>,
        T::Response: for<'de> serde::Deserialize<'de>,
    {
        let f = std::fs::File::open(file)?;

        #[derive(serde::Deserialize)]
        struct Data<R: TransportRequest> {
            request: R,
            response: R::Response,
        }

        let data: Data<T> = serde_json::from_reader(f).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unable to parse json: {}", e),
            )
        })?;

        self.expect(&data.request, &data.response);
        Ok((data.request, data.response))
    }

    /// Read and deserialize expected response for provided request.
    ///
    /// Returns None if the request is not expected.
    ///
    /// # Panics
    ///
    /// Panics if the request can't be serialized or response can't be deserialized.
    fn get_expectation<R: TransportRequest>(&self, request: &R) -> Option<R::Response> {
        self.expectations.get(request)
    }
}

#[async_trait]
impl Dapi for MockDapiClient {
    async fn execute<R: TransportRequest>(
        &mut self,
        request: R,
        _settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>> {
        let response = self.get_expectation(&request);

        if let Some(response) = response {
            return Ok(response);
        } else {
            return Err(DapiClientError::MockExpectationNotFound(format!(
                "unexpected mock request, use MockDapiClient::expect(): {:?}",
                request
            )));
        }
    }
}

type ExpectationKey = Vec<u8>;
type ExpectationValue = Vec<u8>;
#[derive(Default)]
/// Requests expected by a mock and their responses.
struct Expectations {
    expectations: HashMap<ExpectationKey, ExpectationValue>,
}

impl Expectations {
    /// Add expected request and provide given response.
    ///
    /// If the expectation already exists, it will be silently replaced.
    pub fn add<I: serde::Serialize, O: serde::Serialize + serde::de::DeserializeOwned>(
        &mut self,
        request: &I,
        response: &O,
    ) {
        let key = Self::key(&request);
        let value = Self::value(response);

        self.expectations.insert(key, value);
    }

    /// Get the response for a given request.
    ///
    /// Returns `None` if the request has not been expected.
    pub fn get<I: serde::Serialize, O: for<'de> serde::Deserialize<'de>>(
        &self,
        request: I,
    ) -> Option<O> {
        let key = Self::key(&request);

        self.expectations
            .get(&key)
            .and_then(Self::deserialize_value)
    }

    fn key<I: serde::Serialize>(request: &I) -> ExpectationKey {
        bincode::serde::encode_to_vec(request, bincode::config::standard()).expect("encode key")
    }

    fn value<I: serde::Serialize>(request: &I) -> ExpectationValue {
        // We use json because bincode sometimes fail to deserialize
        serde_json::to_vec(request).expect("encode value")
    }

    fn deserialize_value<O: for<'de> serde::Deserialize<'de>>(value: &ExpectationValue) -> O {
        // We use json because bincode sometimes fail to deserialize
        serde_json::from_slice(value).expect("deserialize value")
    }
}
