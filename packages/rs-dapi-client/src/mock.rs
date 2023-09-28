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
    expectations: HashMap<Vec<u8>, Vec<u8>>,
}
impl MockDapiClient {
    /// Create a new mock client
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new expectation for a request
    pub fn expect<R>(&mut self, request: &R, response: &R::Response)
    where
        R: TransportRequest,
    {
        let key = bincode::serde::encode_to_vec(request, bincode::config::standard())
            .expect("encode request");
        let value = bincode::serde::encode_to_vec(response, bincode::config::standard())
            .expect("encode response");

        self.expectations.insert(key, value);
    }

    /// Read and deserialize expected response for provided request.
    ///
    /// Returns None if the request is not expected.
    ///
    /// # Panics
    ///
    /// Panics if the request can't be serialized or response can't be deserialized.
    fn get_expectation<R: TransportRequest>(&self, request: &R) -> Option<R::Response> {
        let config = bincode::config::standard();
        let key = bincode::serde::encode_to_vec(request, config).expect("encode request");

        self.expectations
            .get(&key)
            .map(|v| bincode::serde::decode_from_slice(v, config).expect("decode response"))
            .map(|(v, _)| v)
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

/*
/// Mock Platform transport, used for testing of the Platform without connecting to a remote node
type MockPlatformTransport = MockableClient<PlatformGrpcClient>;
impl MockPlatformTransport {
    /// Create a new mock client
    pub fn new() -> Self {
        Self::default()
    }
}

/// Wrapper around [TransportClient] to support mocking.
///
/// This is a generic enum that can be either a normal client or a mock client.
/// Usually, you don't need to use it directly, but instead use [MockDapiClient]
/// and [MockRequest] that uses it internally.
#[derive(Debug, Clone, Default)]
pub enum MockableClient<T: TransportClient> {
    /// Normal client
    Normal(T),
    #[cfg(feature = "mocks")]
    #[default]
    /// Mock client
    Mock,
}

impl<C: TransportClient> TransportClient for MockableClient<C> {
    type Inner = C;
    type Error = C::Error;

    fn as_mut_inner(&mut self) -> Option<&mut Self::Inner> {
        if let Self::Normal(inner) = self {
            return Some(inner);
        }
        None
    }

    fn mock() -> Self {
        Self::Mock
    }

    fn with_uri(uri: crate::Uri) -> Self {
        Self::Normal(C::with_uri(uri))
    }
}

/// Mock request that returns a predefined response.
///
/// This is a generic struct that can be used to mock any request implementing [DapiRequest].
///
/// ## Example
///
/// ```rust
/// use dapi_grpc::platform::v0::GetIdentityRequest;
/// use rs_dapi_client::mock::MockRequest;
///
/// let req = GetIdentityRequest::default();
/// let resp = dapi_grpc::platform::v0::GetIdentityResponse::default();
///
/// let mock_req = MockRequest::new(req, resp);
///
/// let mut client = MockableClient::<PlatformGrpcClient>::Mock;
/// ```
///
/// See `tests/mock_dapi_client.rs` for an example.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct MockRequest<Req>
where
    Req: TransportRequest + Send,
{
    request: Req,
    response: Req::Response,
    metadata: Metadata,
}

impl<Req: TransportRequest> MockRequest<Req>
where
    Req: TransportRequest,
{
    /// Create a new mock request with a predefined response
    pub fn new(request: Req, response: Req::Response) -> Self {
        Self {
            request,
            response,
            metadata: Default::default(),
        }
    }

    /// Create mock based on a file
    pub fn from_file(path: &str) -> Self
    where
        Req: serde::de::DeserializeOwned, // dapi_grpc::Message
        Req::Response: serde::de::DeserializeOwned,
    {
        let f = File::open(path).expect("open file with mock request");
        todo!("implment from_file");
        // serde_json::from_reader(f).expect("load mock request")
    }

    /// Return a [QuorumInfoProvider] that supports the quorum public key in the metadata.
    ///
    /// If no provider is given, a default one is created.
    pub fn quorum_provider(
        &self,
        provider: Option<MockQuorumInfoProvider>,
    ) -> MockQuorumInfoProvider {
        let mut provider = provider.unwrap_or_default();

        let pubkey: [u8; 48] = self
            .metadata
            .quorum_public_key
            .clone()
            .try_into()
            .expect("pubkey size");

        let quorum_hash: [u8; 32] = self
            .metadata
            .quorum_hash
            .clone()
            .try_into()
            .expect("quorum hash size");

        provider
            .expect_get_quorum_public_key()
            .withf(move |_ty, hash, _chainlock| hash == &quorum_hash)
            .returning(move |_, _, _| Ok(pubkey));

        provider
    }
}

impl<'a, Req: TransportRequest + Clone> TransportRequest for MockRequest<Req>
where
    Req::Response: 'static,
{
    const SETTINGS_OVERRIDES: crate::RequestSettings = crate::RequestSettings::default();
    type Client = MockableClient<PlatformGrpcClient>;
    type Response = Req::Response;
    fn execute_transport<'c>(
        self,
        _client: &'c mut Self::Client,
        _settings: &crate::request_settings::AppliedRequestSettings,
    ) -> futures::future::BoxFuture<
        'c,
        Result<Self::Response, <Self::Client as TransportClient>::Error>,
    > {
        // let me = &mut self;
        let response = self.response.clone();
        let response = response.to_owned();
        futures::future::lazy(move |_| Ok(response)).boxed()
        // futures::future::ready(Ok(response)).boxed()
    }
}

/// Metadata for a [MockRequest]
#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Metadata {
    #[serde(with = "dapi_grpc::deserialization::hexstring")]
    /// Quorum hash used to generate the proof
    pub quorum_hash: Vec<u8>,
    #[serde(with = "dapi_grpc::deserialization::hexstring")]
    /// Quorum public key used to sign proofs and verify it
    pub quorum_public_key: Vec<u8>,
    /// Data contract used in the request; only present when processing documents
    pub data_contract: Option<dpp::prelude::DataContract>,
}

// ========== Implementation of all requests for MockRequest ==========

// FIXME: can't we just use generic like:
// `impl<R:DapiRequest> From<MockRequest<R>> for R`?

macro_rules! impl_from {
    ($request: ty) => {
        impl From<MockRequest<$request>> for $request {
            fn from(mock_req: MockRequest<$request>) -> Self {
                mock_req.request.into()
            }
        }

        impl From<MockRequest<$request>> for <$request as TransportRequest>::Response {
            fn from(mock_req: MockRequest<$request>) -> Self {
                mock_req.response.into()
            }
        }
    };
}

impl_from!(platform_proto::GetIdentityRequest);
impl_from!(platform_proto::GetDocumentsRequest);
impl_from!(platform_proto::GetDataContractRequest);
impl_from!(platform_proto::GetConsensusParamsRequest);
impl_from!(platform_proto::GetDataContractHistoryRequest);
impl_from!(platform_proto::BroadcastStateTransitionRequest);
impl_from!(platform_proto::WaitForStateTransitionResultRequest);
impl_from!(platform_proto::GetIdentitiesByPublicKeyHashesRequest);

#[cfg(test)]
mod test {
    use dapi_grpc::platform::v0::GetIdentityRequest;

    use crate::transport::{grpc::PlatformGrpcClient, TransportRequest};

    #[tokio::test]
    async fn test_mock_platform_transport() {
        let req = GetIdentityRequest::default();
        let resp = dapi_grpc::platform::v0::GetIdentityResponse::default();

        let mock_req = super::MockRequest::new(req, resp);

        let mut transport = super::MockPlatformTransport::Mock;
        let settings = crate::RequestSettings::default().finalize();

        let _answer = mock_req
            .execute_transport(&mut transport, &settings)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_mock_get_identity_from_file() {
        let req = GetIdentityRequest::default();
        let resp = dapi_grpc::platform::v0::GetIdentityResponse::default();

        let mock_req = super::MockRequest::new(req, resp);

        let mut client = super::MockableClient::<PlatformGrpcClient>::Mock;
        let settings = crate::request_settings::RequestSettings::default().finalize();
        let _answer = mock_req
            .execute_transport(&mut client, &settings)
            .await
            .unwrap();
    }
}
*/
