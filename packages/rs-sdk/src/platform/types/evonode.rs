//! Evo Node represents a network node (server).

use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0;
use dapi_grpc::platform::v0::{self as proto, get_status_request, GetStatusRequest};
use dapi_grpc::tonic::IntoRequest;
use futures::future::BoxFuture;
use futures::{FutureExt, TryFutureExt};
use rs_dapi_client::transport::{
    AppliedRequestSettings, PlatformGrpcClient, TransportClient, TransportError, TransportRequest,
};
use rs_dapi_client::{Address, ConnectionPool, RequestSettings};
#[cfg(feature = "mocks")]
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// EvoNode allows querying the status of a single node using DAPI.
///
/// ## Example
///
/// ```rust,no_run
/// use dash_sdk::{platform::types::evonode::EvoNode,platform::FetchUnproved, Sdk};
/// use drive_proof_verifier::types::evonode_status::EvoNodeStatus;
/// use futures::executor::block_on;
///
/// let sdk = Sdk::new_mock();
/// let address = "http://127.0.0.1:1".parse().expect("valid address");
/// let node = EvoNode::new(address);
/// let status = block_on(EvoNodeStatus::fetch_unproved(&sdk, node)).unwrap();
/// ```

#[derive(Debug, Clone)]
#[cfg_attr(feature = "mocks", derive(Serialize, Deserialize))]
pub struct EvoNode(Address);

impl EvoNode {
    /// Creates a new `EvoNode` with the given address.
    pub fn new(address: Address) -> Self {
        Self(address)
    }
}

impl Mockable for EvoNode {
    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        serde_json::de::from_slice(data).ok()
    }

    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        serde_json::ser::to_vec(self).ok()
    }
}
impl TransportRequest for EvoNode {
    type Client = PlatformGrpcClient;
    type Response = proto::GetStatusResponse;

    const SETTINGS_OVERRIDES: rs_dapi_client::RequestSettings = RequestSettings::default();

    fn method_name(&self) -> &'static str {
        "get_status"
    }

    fn execute_transport<'c>(
        self,
        _client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        let uri = self.0.uri();
        // As this is single node connection case, we create a new connection pool with space for a single connection
        // and we drop it after use.
        //
        // We also create a new client to use with this request, so that the user does not need to
        // reconfigure SDK to use a single node.
        let pool = ConnectionPool::new(1);
        // We create a new client with the given URI and settings
        let client_result = Self::Client::with_uri_and_settings(uri.clone(), settings, &pool);

        // Handle the result manually to create a proper error response
        let mut client = match client_result {
            Ok(client) => client,
            Err(e) => {
                return async { Err(e) }.boxed();
            }
        };
        let mut grpc_request = GetStatusRequest {
            version: Some(get_status_request::Version::V0(GetStatusRequestV0 {})),
        }
        .into_request();

        // we need to establish connection only with provided node, so we override client

        if !settings.timeout.is_zero() {
            grpc_request.set_timeout(settings.timeout);
        }

        async move {
            let response = client
                .get_status(grpc_request)
                .map_err(TransportError::Grpc)
                .map_ok(|response| response.into_inner())
                .await;

            drop(client);
            drop(pool);
            response
        }
        .boxed()
    }
}

impl From<EvoNode> for GetStatusRequest {
    fn from(_node: EvoNode) -> Self {
        // we don't need to send any data to the node, and address is handled in impl TrasportRequest
        GetStatusRequestV0 {}.into()
    }
}
