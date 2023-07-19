//! gRPC transport declarations.

use dapi_grpc::core::v0::{self as core_proto, core_client::CoreClient};
use dapi_grpc::platform::v0::{self as platform_proto, platform_client::PlatformClient};
use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use http::Uri;
use tonic::{transport::Channel, IntoRequest};

use crate::settings::AppliedSettings;

use super::{TransportClient, TransportRequest};

pub type PlatformGrpcClient = PlatformClient<Channel>;
pub type CoreGrpcClient = CoreClient<Channel>;

/// Shared transport implementation for all gRPC platform requests.
impl<Req> TransportRequest for Req
where
    Req: GrpcTransportRequest,
{
    type Response = Req::GrpcTransportResponse;
    type Client = Req::GrpcClient;
    type Error = tonic::Status;

    fn execute<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedSettings,
    ) -> BoxFuture<'c, Result<Self::Response, Self::Error>> {
        let mut grpc_request = self.into_request();
        grpc_request.set_timeout(settings.timeout);

        (Self::get_grpc_method())(client, grpc_request)
    }
}

/// Type alias to pull out a certain method of PlatformClient.
type GrpcMethod<Req, Resp, Client> =
    Box<dyn Fn(&mut Client, tonic::Request<Req>) -> BoxFuture<Result<Resp, tonic::Status>>>;

/// Generic properties of all requests that use gRPC transport.
pub trait GrpcTransportRequest: Sized + Clone {
    /// Transport layer response specific for the gRPC request.
    type GrpcTransportResponse;

    /// gRPC client to use.
    type GrpcClient: TransportClient;

    /// Get a handle for a gRPC client method according to the gRPC request.
    fn get_grpc_method() -> GrpcMethod<Self, Self::GrpcTransportResponse, Self::GrpcClient>;
}

impl TransportClient for PlatformGrpcClient {
    fn with_uri(uri: Uri) -> Self {
        let channel = Channel::builder(uri).connect_lazy();
        PlatformGrpcClient::new(channel)
    }
}

impl TransportClient for CoreGrpcClient {
    fn with_uri(uri: Uri) -> Self {
        let channel = Channel::builder(uri).connect_lazy();
        CoreGrpcClient::new(channel)
    }
}

/// A shortcut to link between gRPC request type, response type, client and its
/// method in order to represent it in a form of types and data.
macro_rules! link_grpc_method {
    ($request:ty, $response:ty, $client:ty, $($method:tt)+) => {
        impl GrpcTransportRequest for $request {
            type GrpcTransportResponse = $response;

            type GrpcClient = $client;

            fn get_grpc_method() -> GrpcMethod<Self, Self::GrpcTransportResponse, Self::GrpcClient> {
                Box::new(|client, request| {
                    client
                        .$($method)+(request)
                        .map_ok(tonic::Response::into_inner)
                        .boxed()
                })
            }
        }
    };
}

// Link to each platform gRPC request what client and method to use:

link_grpc_method!(
    platform_proto::GetIdentityRequest,
    platform_proto::GetIdentityResponse,
    PlatformGrpcClient,
    get_identity
);

link_grpc_method!(
    platform_proto::GetDocumentsRequest,
    platform_proto::GetDocumentsResponse,
    PlatformGrpcClient,
    get_documents
);

link_grpc_method!(
    platform_proto::GetDataContractRequest,
    platform_proto::GetDataContractResponse,
    PlatformGrpcClient,
    get_data_contract
);

link_grpc_method!(
    platform_proto::GetConsensusParamsRequest,
    platform_proto::GetConsensusParamsResponse,
    PlatformGrpcClient,
    get_consensus_params
);

link_grpc_method!(
    platform_proto::GetDataContractHistoryRequest,
    platform_proto::GetDataContractHistoryResponse,
    PlatformGrpcClient,
    get_data_contract_history
);

link_grpc_method!(
    platform_proto::BroadcastStateTransitionRequest,
    platform_proto::BroadcastStateTransitionResponse,
    PlatformGrpcClient,
    broadcast_state_transition
);

link_grpc_method!(
    platform_proto::WaitForStateTransitionResultRequest,
    platform_proto::WaitForStateTransitionResultResponse,
    PlatformGrpcClient,
    wait_for_state_transition_result
);

link_grpc_method!(
    platform_proto::GetIdentitiesByPublicKeyHashesRequest,
    platform_proto::GetIdentitiesByPublicKeyHashesResponse,
    PlatformGrpcClient,
    get_identities_by_public_key_hashes
);

// Link to each core gRPC request what client and method to use:

link_grpc_method!(
    core_proto::GetTransactionRequest,
    core_proto::GetTransactionResponse,
    CoreGrpcClient,
    get_transaction
);

link_grpc_method!(
    core_proto::GetStatusRequest,
    core_proto::GetStatusResponse,
    CoreGrpcClient,
    get_status
);

link_grpc_method!(
    core_proto::BroadcastTransactionRequest,
    core_proto::BroadcastTransactionResponse,
    CoreGrpcClient,
    broadcast_transaction
);
