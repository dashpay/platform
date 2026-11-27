//! This module implements Drive ABCI server.
//!

use crate::abci::app::shadow::{open_shadow_platform, Shadow};
use crate::abci::app::CheckTxAbciApplication;
use crate::abci::app::ConsensusAbciApplication;
use crate::config::PlatformConfig;
use crate::platform_types::platform::Platform;
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
    // In shadow mode every block the node commits is applied to a second database on the
    // other storage engine and compared. The shadow answers no request. Its Platform is
    // opened here, before anything is served: opening one sets the protocol version of
    // the process, which is set back to the node's own before a query can read it.
    let shadow_platform =
        open_shadow_platform(platform.as_ref()).expect("failed to open the database of the shadow");

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
    let ends = cancel.clone();
    let shadow = Shadow::for_application(
        platform.as_ref(),
        shadow_platform.as_ref(),
        Arc::new(move || ends.cancel()),
    )
    .expect("failed to start shadow mode");
    let app = match shadow {
        Some(shadow) => app.with_shadow(shadow),
        None => app,
    };

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
