//! Listing of gRPC requests used in DAPI.

use std::time::Duration;

use dapi_grpc::core::v0::core_client::CoreClient;
use dapi_grpc::platform::v0::{self as platform_proto, platform_client::PlatformClient};
use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use http::Uri;
use tonic::{transport::Channel, IntoRequest};

use super::{CanRetry, TransportClient, TransportRequest, TransportResponse};
use crate::{request_settings::AppliedRequestSettings, RequestSettings};

/// Platform Client using gRPC transport.
pub type PlatformGrpcClient = PlatformClient<Channel>;
/// Core Client using gRPC transport.
pub type CoreGrpcClient = CoreClient<Channel>;

impl TransportClient for PlatformGrpcClient {
    type Error = tonic::Status;

    fn with_uri(uri: Uri) -> Self {
        Self::new(Channel::builder(uri).connect_lazy())
    }
}

impl TransportClient for CoreGrpcClient {
    type Error = tonic::Status;

    fn with_uri(uri: Uri) -> Self {
        Self::new(Channel::builder(uri).connect_lazy())
    }
}

impl CanRetry for tonic::Status {
    fn can_retry(&self) -> bool {
        let code = self.code();

        use tonic::Code::*;
        matches!(
            code,
            Ok | DataLoss
                | Cancelled
                | Unknown
                | DeadlineExceeded
                | ResourceExhausted
                | Aborted
                | Internal
        )
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
        impl TransportResponse for $response {}
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

impl_transport_request_grpc!(
    platform_proto::GetIdentityByPublicKeyHashRequest,
    platform_proto::GetIdentityByPublicKeyHashResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identity_by_public_key_hash
);

impl_transport_request_grpc!(
    platform_proto::GetIdentityBalanceRequest,
    platform_proto::GetIdentityBalanceResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identity_balance
);

impl_transport_request_grpc!(
    platform_proto::GetIdentityBalanceAndRevisionRequest,
    platform_proto::GetIdentityBalanceAndRevisionResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identity_balance_and_revision
);

impl_transport_request_grpc!(
    platform_proto::GetIdentityKeysRequest,
    platform_proto::GetIdentityKeysResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identity_keys
);

// Link to each core gRPC request what client and method to use:
/*
TODO: Implement serde on Core gRPC requests and responses

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
*/
