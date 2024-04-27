//! Listing of gRPC requests used in DAPI.

use std::time::Duration;

use super::{CanRetry, TransportClient, TransportRequest};
use crate::connection_pool::{ConnectionPool, PoolPrefix};
use crate::{request_settings::AppliedRequestSettings, RequestSettings};
use dapi_grpc::core::v0::core_client::CoreClient;
use dapi_grpc::core::v0::{self as core_proto};
use dapi_grpc::platform::v0::{self as platform_proto, platform_client::PlatformClient};
use dapi_grpc::tonic::Streaming;
use dapi_grpc::tonic::{transport::Channel, IntoRequest};
use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use http::Uri;

/// Platform Client using gRPC transport.
pub type PlatformGrpcClient = PlatformClient<Channel>;
/// Core Client using gRPC transport.
pub type CoreGrpcClient = CoreClient<Channel>;

fn create_channel(uri: Uri, settings: Option<&AppliedRequestSettings>) -> Channel {
    let mut builder = Channel::builder(uri);

    if let Some(settings) = settings {
        if let Some(timeout) = settings.connect_timeout {
            builder = builder.connect_timeout(timeout);
        }
    }

    builder.connect_lazy()
}

impl TransportClient for PlatformGrpcClient {
    type Error = dapi_grpc::tonic::Status;

    fn with_uri(uri: Uri, pool: &ConnectionPool) -> Self {
        pool.get_or_create(PoolPrefix::Platform, &uri, None, || {
            Self::new(create_channel(uri.clone(), None)).into()
        })
        .into()
    }

    fn with_uri_and_settings(
        uri: Uri,
        settings: &AppliedRequestSettings,
        pool: &ConnectionPool,
    ) -> Self {
        pool.get_or_create(PoolPrefix::Platform, &uri, Some(settings), || {
            Self::new(create_channel(uri.clone(), Some(settings))).into()
        })
        .into()
    }
}

impl TransportClient for CoreGrpcClient {
    type Error = dapi_grpc::tonic::Status;

    fn with_uri(uri: Uri, pool: &ConnectionPool) -> Self {
        pool.get_or_create(PoolPrefix::Core, &uri, None, || {
            Self::new(create_channel(uri.clone(), None)).into()
        })
        .into()
    }

    fn with_uri_and_settings(
        uri: Uri,
        settings: &AppliedRequestSettings,
        pool: &ConnectionPool,
    ) -> Self {
        pool.get_or_create(PoolPrefix::Core, &uri, Some(settings), || {
            Self::new(create_channel(uri.clone(), Some(settings))).into()
        })
        .into()
    }
}

impl CanRetry for dapi_grpc::tonic::Status {
    fn is_node_failure(&self) -> bool {
        let code = self.code();

        use dapi_grpc::tonic::Code::*;
        matches!(
            code,
            Ok | DataLoss
                | Cancelled
                | Unknown
                | DeadlineExceeded
                | ResourceExhausted
                | Aborted
                | Internal
                | Unavailable
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

            fn method_name(&self) -> &'static str {
                stringify!($($method)+)
            }

            fn execute_transport<'c>(
                self,
                client: &'c mut Self::Client,
                settings: &AppliedRequestSettings,
            ) -> BoxFuture<'c, Result<Self::Response, <Self::Client as TransportClient>::Error>>
            {
                let mut grpc_request = self.into_request();

                if !settings.timeout.is_zero() {
                    grpc_request.set_timeout(settings.timeout);
                }

                client
                    .$($method)+(grpc_request)
                    .map_ok(|response| response.into_inner())
                    .boxed()
            }
        }
    };
}

// Link to each platform gRPC request what client and method to use:

const STREAMING_TIMEOUT: Duration = Duration::from_secs(5 * 60);

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
        timeout: Some(Duration::from_secs(80)),
        retries: Some(0),
        ..RequestSettings::default()
    },
    wait_for_state_transition_result
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
    platform_proto::GetIdentityNonceRequest,
    platform_proto::GetIdentityNonceResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identity_nonce
);

impl_transport_request_grpc!(
    platform_proto::GetIdentityContractNonceRequest,
    platform_proto::GetIdentityContractNonceResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identity_contract_nonce
);

impl_transport_request_grpc!(
    platform_proto::GetIdentityBalanceAndRevisionRequest,
    platform_proto::GetIdentityBalanceAndRevisionResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identity_balance_and_revision
);

impl_transport_request_grpc!(
    platform_proto::GetIdentitiesContractKeysRequest,
    platform_proto::GetIdentitiesContractKeysResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identities_contract_keys
);

impl_transport_request_grpc!(
    platform_proto::GetIdentityKeysRequest,
    platform_proto::GetIdentityKeysResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_identity_keys
);

impl_transport_request_grpc!(
    platform_proto::GetEpochsInfoRequest,
    platform_proto::GetEpochsInfoResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_epochs_info
);

impl_transport_request_grpc!(
    platform_proto::GetProtocolVersionUpgradeStateRequest,
    platform_proto::GetProtocolVersionUpgradeStateResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_protocol_version_upgrade_state
);

impl_transport_request_grpc!(
    platform_proto::GetProtocolVersionUpgradeVoteStatusRequest,
    platform_proto::GetProtocolVersionUpgradeVoteStatusResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_protocol_version_upgrade_vote_status
);

impl_transport_request_grpc!(
    platform_proto::GetDataContractsRequest,
    platform_proto::GetDataContractsResponse,
    PlatformGrpcClient,
    RequestSettings::default(),
    get_data_contracts
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
    core_proto::GetBlockchainStatusRequest,
    core_proto::GetBlockchainStatusResponse,
    CoreGrpcClient,
    RequestSettings::default(),
    get_blockchain_status
);

impl_transport_request_grpc!(
    core_proto::BroadcastTransactionRequest,
    core_proto::BroadcastTransactionResponse,
    CoreGrpcClient,
    RequestSettings::default(),
    broadcast_transaction
);

impl_transport_request_grpc!(
    core_proto::TransactionsWithProofsRequest,
    Streaming<core_proto::TransactionsWithProofsResponse>,
    CoreGrpcClient,
    RequestSettings {
        timeout: Some(STREAMING_TIMEOUT),
        ..RequestSettings::default()
    },
    subscribe_to_transactions_with_proofs
);
