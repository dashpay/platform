//! Mock implementation of rs-dapi-client for testing

use std::{fmt::Debug, fs::File};

use futures::FutureExt;

use crate::{
    transport::{grpc::PlatformGrpcClient, TransportClient, TransportRequest},
    DapiRequest,
};

use dapi_grpc::platform::v0 as platform_proto;

/// Mock request that returns a predefined response
#[derive(Debug, Clone, PartialEq)]
pub struct MockRequest<Req: DapiRequest>
where
    Req: DapiRequest,
    Req::Response: 'static,
{
    request: Req,
    response: Req::Response,
}
impl<Req: DapiRequest> MockRequest<Req> {
    /// Create a new mock request with a predefined response
    pub fn new(request: Req, response: Req::Response) -> Self {
        Self { request, response }
    }

    #[allow(unused)]
    fn load(
        path: &str,
    ) -> (
        Req,
        Req::Response,
        Metadata,
        drive_proof_verifier::proof::from_proof::MockQuorumInfoProvider,
    )
    where
        Req: serde::de::DeserializeOwned, // dapi_grpc::Message
        Req::Response: serde::de::DeserializeOwned,
    {
        let f = File::open(path).unwrap();
        let (req, resp, metadata): (Req, Req::Response, Metadata) =
            serde_json::from_reader(f).unwrap();

        let pubkey = metadata
            .quorum_public_key
            .clone()
            .try_into()
            .expect("pubkey size");
        let mut provider = drive_proof_verifier::proof::from_proof::MockQuorumInfoProvider::new();
        provider
            .expect_get_quorum_public_key()
            .return_once(move |_, _, _| Ok(pubkey));

        (req, resp, metadata, provider)
    }
}

/// Metadata for a mock request
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Metadata {
    #[serde(with = "dapi_grpc::deserialization::hexstring")]
    pub quorum_public_key: Vec<u8>,
    pub data_contract: Option<dpp::prelude::DataContract>,
}

impl<'r, Req: DapiRequest> TransportRequest for MockRequest<Req>
where
    Req: Sync + Send + Debug + Clone + PartialEq,
    Req::Response: Sync + Send + Debug + Clone + PartialEq + 'r,
    <Req as DapiRequest>::Response: 'r,
    // NoopClient: 'r,
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

// FIXME: can't we just use generic like:
// `impl<R:DapiRequest> From<MockRequest<R>> for R`?
impl_from!(platform_proto::GetIdentityRequest);
impl_from!(platform_proto::GetDocumentsRequest);
impl_from!(platform_proto::GetDataContractRequest);
impl_from!(platform_proto::GetConsensusParamsRequest);
impl_from!(platform_proto::GetDataContractHistoryRequest);
impl_from!(platform_proto::BroadcastStateTransitionRequest);
impl_from!(platform_proto::WaitForStateTransitionResultRequest);
impl_from!(platform_proto::GetIdentitiesByPublicKeyHashesRequest);

/*
#[derive(Default)]
/// Mock [TransportClient] that does nothing
#[derive(Debug, Clone)]
pub struct NoopClient {}

impl TransportClient for NoopClient {
    type Inner = ();
    type Error = tonic::Status;

    fn with_uri(_uri: crate::Uri) -> Self {
        Self {}
    }

    fn as_mut_inner<'a>(&'a mut self) -> Option<&'a mut Self::Inner> {
        None
    }
}

impl GrpcService<tonic::body::BoxBody> for NoopClient {
    //         T: tonic::client::GrpcService<tonic::body::BoxBody>,
    // T::Error: Into<StdError>,
    // T::ResponseBody: Body<Data = Bytes> + Send + 'static,
    // <T::ResponseBody as Body>::Error: Into<StdError> + Send,

    /// Responses body given by the service.
    type ResponseBody = String;
    /// Errors produced by the service.
    type Error = tonic::Status;
    /// The future response value.
    type Future = futures::future::Ready<Result<http::Response<Self::ResponseBody>, Self::Error>>;

    /// Returns `Ready` when the service is able to process requests.
    ///
    /// Reference [`Service::poll_ready`].
    fn poll_ready(
        &mut self,
        _cx: &mut tonic::codegen::Context<'_>,
    ) -> tonic::codegen::Poll<Result<(), Self::Error>> {
        tonic::codegen::Poll::Ready(Ok(()))
    }

    /// Process the request and return the response asynchronously.
    ///
    /// Reference [`Service::call`].
    fn call(&mut self, _request: http::Request<tonic::body::BoxBody>) -> Self::Future {
        let response = http::Response::builder()
            .status(200)
            .body("".to_string())
            .unwrap();
        futures::future::ready(Ok(response))
    }
}
*/
/// Mockable client that can be implemented for any transport client.
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

    fn with_uri(uri: crate::Uri) -> Self {
        Self::Normal(C::with_uri(uri))
    }
}

#[cfg(test)]
mod test {
    use dapi_grpc::platform::v0::GetIdentityRequest;

    use crate::transport::{grpc::PlatformGrpcClient, TransportRequest};

    #[tokio::test]
    async fn test_mock_get_identity() {
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

// ========== Implementation of all requests for MockRequest ==========
