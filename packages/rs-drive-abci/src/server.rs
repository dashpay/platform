//! This module implements Drive ABCI server.
//!

use crate::abci::app::CheckTxAbciApplication;
use crate::abci::app::ConsensusAbciApplication;
use crate::config::PlatformConfig;
use crate::platform_types::platform::Platform;
use crate::query::QueryService;
use crate::rpc::core::DefaultCoreRPC;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

/// The largest gRPC message the Drive server decodes and encodes on the DriveInternal and ABCI
/// CheckTx services, the two that carry whole state transitions.
///
/// The largest legitimate message is a contract-code capable state transition at
/// `SystemLimits::max_contract_code_state_transition_size` (32 MiB from protocol version 17),
/// carried whole inside a `getProofs` request from rs-dapi and inside the CheckTx request
/// Tenderdash routes over this endpoint, plus protobuf framing. Tonic's default of 4 MiB would
/// reject such a message before Drive saw it. The constant must stay above every family cap of
/// every registered protocol version plus framing; a test pins that.
pub const MAX_GRPC_MESSAGE_BYTES: usize = 34 * 1024 * 1024;

/// The largest gRPC request the Drive server decodes on the Platform query service.
///
/// Queries never carry a state transition (the proof request for one goes through
/// DriveInternal), so they keep tonic's default 4 MiB decode cap rather than the transaction
/// allowance; the largest query, `getPathElements` at 64 KiB of raw components, sits far below
/// it. Responses stay at the larger cap: a proof over a large contract is a response.
pub const MAX_PLATFORM_QUERY_REQUEST_BYTES: usize = 4 * 1024 * 1024;

/// Starts gRPC and ABCI servers to serve Query, CheckTx and Consensus applications
///
/// Should only return when server is stopped
pub fn start(
    runtime: &Runtime,
    platform: Arc<Platform<DefaultCoreRPC>>,
    config: PlatformConfig,
    cancel: CancellationToken,
) {
    let query_service = Arc::new(QueryService::new(Arc::clone(&platform)));

    let drive_internal = Arc::clone(&query_service);

    let check_tx_core_rpc = DefaultCoreRPC::open(
        &config.core.check_tx_rpc.url(),
        config.core.check_tx_rpc.username,
        config.core.check_tx_rpc.password,
    )
    .expect("failed to open check tx core rpc");

    let check_tx_service =
        CheckTxAbciApplication::new(Arc::clone(&platform), Arc::new(check_tx_core_rpc));

    let grpc_server = dapi_grpc::tonic::transport::Server::builder()
        .add_service(
            dapi_grpc::drive::v0::drive_internal_server::DriveInternalServer::from_arc(
                drive_internal,
            )
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES),
        )
        .add_service(
            dapi_grpc::platform::v0::platform_server::PlatformServer::from_arc(query_service)
                .max_decoding_message_size(MAX_PLATFORM_QUERY_REQUEST_BYTES)
                .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES),
        )
        .add_service(
            tenderdash_abci::proto::abci::abci_application_server::AbciApplicationServer::new(
                check_tx_service,
            )
            .max_decoding_message_size(MAX_GRPC_MESSAGE_BYTES)
            .max_encoding_message_size(MAX_GRPC_MESSAGE_BYTES),
        );

    let grpc_server_cancel = cancel.clone();

    runtime.spawn(async move {
        tracing::info!("gRPC server is listening on {}", &config.grpc_bind_address);

        grpc_server
            .serve_with_shutdown(
                config
                    .grpc_bind_address
                    .parse()
                    .expect("invalid grpc address"),
                grpc_server_cancel.cancelled(),
            )
            .await
            .expect("gRPC server failed");

        tracing::info!("gRPC server is stopped");
    });

    // Start blocking ABCI socket-server that process consensus requests sequentially

    let app = ConsensusAbciApplication::new(platform.as_ref());

    let server = tenderdash_abci::ServerBuilder::new(app, &config.abci.consensus_bind_address)
        .with_cancel_token(cancel.clone())
        .with_runtime(runtime.handle().clone())
        .build()
        .expect("failed to build ABCI server");

    while !cancel.is_cancelled() {
        tracing::info!(
            "ABCI app is waiting for new connection on {}",
            config.abci.consensus_bind_address
        );
        match server.next_client() {
            Err(e) => tracing::error!("ABCI connection terminated: {:?}", e),
            Ok(_) => tracing::info!("ABCI connection closed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_GRPC_MESSAGE_BYTES, MAX_PLATFORM_QUERY_REQUEST_BYTES};
    use dpp::version::PLATFORM_VERSIONS;

    /// Room for the protobuf framing around a maximal state transition (the field tag, the
    /// length varint and the request envelope) and for the proof response around it.
    const FRAMING_HEADROOM_BYTES: u64 = 1024 * 1024;

    #[test]
    fn should_exceed_every_state_transition_family_cap_of_every_protocol_version() {
        for platform_version in PLATFORM_VERSIONS {
            let limits = &platform_version.system_limits;
            let largest_family_cap = limits
                .max_contract_code_state_transition_size
                .unwrap_or(0)
                .max(limits.max_state_transition_size);
            assert!(
                (MAX_GRPC_MESSAGE_BYTES as u64) >= largest_family_cap + FRAMING_HEADROOM_BYTES,
                "protocol version {} admits a {largest_family_cap} byte state transition that the \
                 gRPC server could not receive",
                platform_version.protocol_version
            );
        }
    }

    /// The query service is not a transaction ingress and must not inherit the transaction
    /// allowance; the transaction cap is the larger of the two by construction.
    #[test]
    fn query_requests_keep_a_smaller_cap_than_transaction_carrying_services() {
        assert!(MAX_PLATFORM_QUERY_REQUEST_BYTES < MAX_GRPC_MESSAGE_BYTES);
    }
}
