//! Mock implementation of rs-dapi-client for testing

use std::{fmt::Debug, fs::File};

use futures::FutureExt;
use tonic::async_trait;

use crate::{
    transport::{grpc::PlatformGrpcClient, TransportClient, TransportRequest},
    Dapi, DapiClientError, DapiRequest, RequestSettings,
};
use dapi_grpc::platform::v0 as platform_proto;
use drive_proof_verifier::proof::from_proof::MockQuorumInfoProvider;

/// Mock DAPI client
pub struct MockDapiClient {}
impl MockDapiClient {
    /// Create a new mock client
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for MockDapiClient {
    fn default() -> Self {
        Self {}
    }
}

#[async_trait]
impl Dapi for MockDapiClient {
    async fn execute<R>(
        &mut self,
        request: R,
        settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>>
    where
        R: TransportRequest,
    {
        let mut transport = R::Client::mock();
        let settings = settings.finalize();
        request
            .execute_transport(&mut transport, &settings)
            .await
            .map_err(|e| DapiClientError::Transport(e))
    }
}

/// Mock Platform transport, used for testing of the Platform without connecting to a remote node
pub type MockPlatformTransport = MockableClient<PlatformGrpcClient>;
impl MockPlatformTransport {
    /// Create a new mock client
    pub fn new() -> Self {
        Self::default()
    }
}
impl Default for MockPlatformTransport {
    fn default() -> Self {
        Self::Mock
    }
}

/// Wrapper around [TransportClient] to support mocking
#[derive(Debug, Clone)]
pub enum MockableClient<T: TransportClient> {
    /// Normal client
    Normal(T),
    #[cfg(feature = "mocks")]
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

/// Mock request that returns a predefined response
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MockRequest<Req: DapiRequest>
where
    Req: DapiRequest,
    Req::Response: 'static,
{
    request: Req,
    response: Req::Response,
    metadata: Metadata,
}

impl<Req: DapiRequest> MockRequest<Req> {
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
        serde_json::from_reader(f).expect("load mock request")
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

impl<'r, Req: DapiRequest> TransportRequest for MockRequest<Req>
where
    Req: Sync + Send + Debug + Clone,
    Req::Response: Sync + Send + Debug + Clone,
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
        futures::future::ready(Ok(self.response.clone())).boxed()
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

        impl From<MockRequest<$request>> for <$request as DapiRequest>::Response {
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
