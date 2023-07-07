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

impl GrpcTransportRequest for platform_proto::GetIdentityRequest {
    type GrpcTransportResponse = platform_proto::GetIdentityResponse;

    fn get_grpc_method() -> GrpcMethod<Self, Self::GrpcTransportResponse> {
        Box::new(|client, request| {
            client
                .get_identity(request)
                .map_ok(tonic::Response::into_inner)
                .boxed()
        })
    }
}
