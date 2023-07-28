//! Listing of gRPC requests used in DAPI.

mod self_signed;

use std::time::Duration;

use dapi_grpc::core::v0::{self as core_proto, core_client::CoreClient};
use dapi_grpc::platform::v0::{self as platform_proto, platform_client::PlatformClient};
use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use http::Uri;
use tonic::{transport::Channel, IntoRequest};

use super::{CanRetry, TransportClient, TransportRequest};
use crate::{request_settings::AppliedRequestSettings, RequestSettings};

type PlatformGrpcClient = PlatformClient<Channel>;
type CoreGrpcClient = CoreClient<Channel>;

impl TransportClient for PlatformGrpcClient {
    type Error = tonic::Status;

    fn with_uri(uri: Uri, allow_insecure: bool) -> Self {
        let channel = if allow_insecure {
            Channel::builder(uri)
                .connect_with_connector_lazy(self_signed::INSECURE_CONNECTOR.clone())
        } else {
            Channel::builder(uri).connect_lazy()
        };
        PlatformGrpcClient::new(channel)
    }
}

impl TransportClient for CoreGrpcClient {
    type Error = tonic::Status;

    fn with_uri(uri: Uri, allow_insecure: bool) -> Self {
        let channel = if allow_insecure {
            Channel::builder(uri)
                .connect_with_connector_lazy(self_signed::INSECURE_CONNECTOR.clone())
        } else {
            Channel::builder(uri).connect_lazy()
        };

        CoreGrpcClient::new(channel)
    }
}

impl CanRetry for tonic::Status {
    fn can_retry(&self) -> bool {
        let code = self.code();

        use tonic::Code::*;
        match code {
            Ok | DataLoss | Cancelled | Unknown | DeadlineExceeded | ResourceExhausted
            | Aborted | Internal => true,
            _ => false,
        }
    }
}

/// A shortcut to link between gRPC request type, response type, client and its
/// method in order to represent it in a form of types and data.
macro_rules! impl_transport_request_grpc {
    ($request:ty, $response:ty, $client:ty, $settings:expr, $($method:tt)+) => {
        impl TransportRequest for $request {
            type Client = $client;

            type Response = $response;

            const SETTINGS_OVERRIDES: RequestSettings = $settings;

            fn execute_transport<'c>(
                self,
                client: &'c mut Self::Client,
                settings: &AppliedRequestSettings,
            ) -> BoxFuture<'c, Result<Self::Response, <Self::Client as TransportClient>::Error>>
            {
                let mut grpc_request = self.into_request();
                grpc_request.set_timeout(settings.timeout);

                client
                    .$($method)+(grpc_request)
                    .map_ok(|response| response.into_inner())
                    .boxed()
            }
        }
    };
}

// Link to each platform gRPC request what client and method to use:

impl_transport_request_grpc!(
    platform_proto::GetIdentityRequest,
    platform_proto::GetIdentityResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identity
);

impl_transport_request_grpc!(
    platform_proto::GetDocumentsRequest,
    platform_proto::GetDocumentsResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_documents
);

impl_transport_request_grpc!(
    platform_proto::GetDataContractRequest,
    platform_proto::GetDataContractResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_data_contract
);

impl_transport_request_grpc!(
    platform_proto::GetConsensusParamsRequest,
    platform_proto::GetConsensusParamsResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_consensus_params
);

impl_transport_request_grpc!(
    platform_proto::GetDataContractHistoryRequest,
    platform_proto::GetDataContractHistoryResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_data_contract_history
);

impl_transport_request_grpc!(
    platform_proto::BroadcastStateTransitionRequest,
    platform_proto::BroadcastStateTransitionResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    broadcast_state_transition
);

impl_transport_request_grpc!(
    platform_proto::WaitForStateTransitionResultRequest,
    platform_proto::WaitForStateTransitionResultResponse,
    PlatformGrpcClient,
    RequestSettings {
        timeout: Some(Duration::from_secs(120)),
        ..RequestSettings::default()
    },
    wait_for_state_transition_result
);

impl_transport_request_grpc!(
    platform_proto::GetIdentitiesByPublicKeyHashesRequest,
    platform_proto::GetIdentitiesByPublicKeyHashesResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identities_by_public_key_hashes
);

// Link to each core gRPC request what client and method to use:

impl_transport_request_grpc!(
    core_proto::GetTransactionRequest,
    core_proto::GetTransactionResponse,
    CoreGrpcClient,
    RequestSettings::default(),
    get_transaction
);

impl_transport_request_grpc!(
    core_proto::GetStatusRequest,
    core_proto::GetStatusResponse,
    CoreGrpcClient,
    RequestSettings::default(),
    get_status
);

impl_transport_request_grpc!(
    core_proto::BroadcastTransactionRequest,
    core_proto::BroadcastTransactionResponse,
    CoreGrpcClient,
    RequestSettings::default(),
    broadcast_transaction
);
