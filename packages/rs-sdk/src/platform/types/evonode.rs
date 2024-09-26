//! Evo Node represents a network node (server).

use crate::mock::BINCODE_CONFIG;
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0;
use dapi_grpc::platform::v0::{self as proto, get_status_request, GetStatusRequest};
use dapi_grpc::tonic::IntoRequest;
use futures::future::BoxFuture;
use futures::{FutureExt, TryFutureExt};
use rs_dapi_client::transport::{
    AppliedRequestSettings, PlatformGrpcClient, TransportClient, TransportRequest,
};
use rs_dapi_client::{Address, ConnectionPool, RequestSettings};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// EvoNode allows querying the status of a single node using DAPI.
///
/// ## Example
///
/// ```rust
/// use dash_sdk::{platform::query::EvoNode, Sdk};
/// use futures::executor::block_on;
///
/// let sdk = Sdk::new_mock();
/// let node = EvoNode::new("https://44.232.196.6:443".parse().unwrap());
/// let status = block_on(node.get_status(&sdk)).unwrap();
/// ```

#[derive(Debug, Clone)]
#[cfg_attr(feature = "mocks", derive(Serialize, Deserialize))]
pub struct EvoNode(Address);

#[cfg(feature = "mocks")]
impl Mockable for EvoNode {
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        if let Ok((obj, _)) = bincode::serde::decode_from_slice(data, BINCODE_CONFIG) {
            Some(obj)
        } else {
            None
        }
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
    ) -> BoxFuture<'c, Result<Self::Response, <Self::Client as TransportClient>::Error>> {
        let uri = self.0.uri();
        // As this is single node connection case, we create a new connection pool with space for a single connection
        // and we drop it after use.
        //
        // We also create a new client to use with this request, so that the user does not need to
        // reconfigure SDK to use a single node.
        let pool = ConnectionPool::new(1);
        let mut client = Self::Client::with_uri_and_settings(uri.clone(), settings, &pool);
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
                .map_ok(|response| response.into_inner())
                .await;

            drop(client);
            drop(pool);
            response
        }
        .boxed()
    }
}
