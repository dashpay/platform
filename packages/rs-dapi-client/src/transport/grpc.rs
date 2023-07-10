//! gRPC transport declarations.

use dapi_grpc::platform::v0::{self as platform_proto, platform_client::PlatformClient};
use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use tonic::{transport::Channel, IntoRequest};

use crate::settings::AppliedSettings;

use super::TransportRequest;

pub type PlatformGrpcClient = PlatformClient<Channel>;

/// Shared transport implementation for all gRPC platform requests.
// TODO: decide if to share with core gRPC if needed.
impl<Req> TransportRequest for Req
where
    Req: GrpcTransportRequest,
{
    type Response = Req::GrpcTransportResponse;
    type Client = PlatformClient<Channel>;
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
type GrpcMethod<Req, Resp> = Box<
    dyn Fn(
        &mut PlatformClient<Channel>,
        tonic::Request<Req>,
    ) -> BoxFuture<Result<Resp, tonic::Status>>,
>;

/// Generic properties of all requests that use gRPC transport.
pub trait GrpcTransportRequest: Sized + Clone {
    /// Transport layer response specific for the gRPC request.
    type GrpcTransportResponse;

    /// Get a handle for a gRPC client method according to the gRPC request.
    fn get_grpc_method() -> GrpcMethod<Self, Self::GrpcTransportResponse>;
}

/// A shortcut to link between gRPC request type, response type and the mehod of
/// [PlatformClient] in order to represent it in a form of types and data.
macro_rules! link_grpc_method {
    ($request:ty, $response:ty, $($method:tt)+) => {
        impl GrpcTransportRequest for $request {
            type GrpcTransportResponse = $response;

            fn get_grpc_method() -> GrpcMethod<Self, Self::GrpcTransportResponse> {
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

link_grpc_method!(
    platform_proto::GetIdentityRequest,
    platform_proto::GetIdentityResponse,
    get_identity
);

link_grpc_method!(
    platform_proto::GetDocumentsRequest,
    platform_proto::GetDocumentsResponse,
    get_documents
);

link_grpc_method!(
    platform_proto::GetDataContractRequest,
    platform_proto::GetDataContractResponse,
    get_data_contract
);

link_grpc_method!(
    platform_proto::GetConsensusParamsRequest,
    platform_proto::GetConsensusParamsResponse,
    get_consensus_params
);

link_grpc_method!(
    platform_proto::BroadcastStateTransitionRequest,
    platform_proto::BroadcastStateTransitionResponse,
    broadcast_state_transition
);

link_grpc_method!(
    platform_proto::WaitForStateTransitionResultRequest,
    platform_proto::WaitForStateTransitionResultResponse,
    wait_for_state_transition_result
);

link_grpc_method!(
    platform_proto::GetIdentitiesByPublicKeyHashesRequest,
    platform_proto::GetIdentitiesByPublicKeyHashesResponse,
    get_identities_by_public_key_hashes
);
