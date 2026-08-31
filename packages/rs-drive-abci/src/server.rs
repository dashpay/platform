//! This module implements Drive ABCI server.
//!

use crate::abci::app::CheckTxAbciApplication;
use crate::abci::app::ConsensusAbciApplication;
use crate::config::PlatformConfig;
use crate::platform_types::platform::Platform;
use crate::platform_types::snapshot::{SnapshotManager, SERVING_PIN_SWEEP_INTERVAL};
use crate::query::QueryService;
use crate::rpc::core::DefaultCoreRPC;
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
    let query_service = Arc::new(QueryService::new(Arc::clone(&platform)));

    let drive_internal = Arc::clone(&query_service);

    let check_tx_core_rpc = DefaultCoreRPC::open(
        &config.core.check_tx_rpc.url(),
        config.core.check_tx_rpc.username,
        config.core.check_tx_rpc.password,
    )
    .expect("failed to open check tx core rpc");

    // The snapshot manager pins the checkpoints being served to state-syncing peers.
    // It lives with the gRPC application below (which answers ListSnapshots and
    // LoadSnapshotChunk), but that application never sees blocks, so nothing on its
    // request path would ever release the pin of an ABANDONED transfer — the peer
    // simply stops asking. A shared handle and a timer task make expiry autonomous.
    let snapshot_manager = Arc::new(SnapshotManager::new());

    let serving_pin_sweep_cancel = cancel.clone();
    let serving_pin_sweep_manager = Arc::clone(&snapshot_manager);
    runtime.spawn(async move {
        let mut interval = tokio::time::interval(SERVING_PIN_SWEEP_INTERVAL);
        loop {
            tokio::select! {
                _ = serving_pin_sweep_cancel.cancelled() => break,
                _ = interval.tick() => serving_pin_sweep_manager.release_expired_pins(),
            }
        }
    });

    let check_tx_service = CheckTxAbciApplication::new(
        Arc::clone(&platform),
        Arc::new(check_tx_core_rpc),
        snapshot_manager,
    );

    let grpc_server = dapi_grpc::tonic::transport::Server::builder()
        .add_service(
            dapi_grpc::drive::v0::drive_internal_server::DriveInternalServer::from_arc(
                drive_internal,
            ),
        )
        .add_service(
            dapi_grpc::platform::v0::platform_server::PlatformServer::from_arc(query_service),
        )
        .add_service(
            tenderdash_abci::proto::abci::abci_application_server::AbciApplicationServer::new(
                check_tx_service,
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
