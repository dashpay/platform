//! This module implements Drive ABCI server.
//!

use crate::abci::app::CheckTxAbciApplication;
use crate::abci::app::ConsensusAbciApplication;
use drive_abci::config::PlatformConfig;
use drive_abci::platform_types::platform::Platform;
use drive_abci::query::QueryService;
use drive_abci::rpc::core::DefaultCoreRPC;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

/// Starts gRPC and ABCI servers to serve Query, CheckTx and Consensus applications
///
/// Should only return when server is stopped
pub fn start(
    runtime: &Runtime,
    platform: Arc<Platform<DefaultCoreRPC>>,
    config: PlatformConfig,
    cancel: CancellationToken,
) {
    let query_service = QueryService::new(Arc::clone(&platform));
    let check_tx_server = CheckTxAbciApplication::new(Arc::clone(&platform));

    let grpc_server = dapi_grpc::tonic::transport::Server::builder()
        .add_service(dapi_grpc::platform::v0::platform_server::PlatformServer::new(query_service))
        .add_service(
            tenderdash_abci::proto::abci::abci_application_server::AbciApplicationServer::new(
                check_tx_server,
            ),
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
