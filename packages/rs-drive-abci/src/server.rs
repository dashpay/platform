//! This module implements Drive ABCI server.
//!

use crate::abci::app::shadow::{open_shadow_platform, Shadow, ShadowError};
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
/// Should only return when server is stopped, or, before the gRPC and ABCI servers are
/// started, with what shadow mode does not start on.
pub fn start(
    runtime: &Runtime,
    platform: Arc<Platform<DefaultCoreRPC>>,
    config: PlatformConfig,
    cancel: CancellationToken,
) -> Result<(), ShadowError> {
    // In shadow mode every block the node commits is applied to a second database on the
    // other storage engine and compared. The shadow answers no request. Its Platform is
    // opened here, before anything is served: opening one sets the protocol version of
    // the process, which is set back to the node's own before a query can read it.
    let shadow_platform = open_shadow_platform(platform.as_ref())?;
    let ends = cancel.clone();
    let shadow = Shadow::for_application(
        platform.as_ref(),
        shadow_platform.as_ref(),
        Arc::new(move || ends.cancel()),
    )?;

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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A start that shadow mode refuses returns the refusal, with its reason and the
    /// path it is about, and does not tell the node to shut down, as a panic does.
    #[test]
    fn a_start_returns_what_shadow_mode_refuses() {
        let directory = tempfile::tempdir().expect("a directory");
        let mut config = PlatformConfig::default_testnet();
        config.db_path = directory.path().join("db");
        config.shadow.shadow_db_path = Some(config.db_path.clone());
        // An address for the client of Dash Core, which nothing here asks.
        config.core.consensus_rpc.host = "127.0.0.1".to_string();
        config.core.consensus_rpc.port = 1;
        let platform = Platform::<DefaultCoreRPC>::open(&config.db_path, Some(config.clone()))
            .expect("a Platform");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        let cancel = CancellationToken::new();

        let refused = start(&runtime, Arc::new(platform), config.clone(), cancel.clone())
            .expect_err("a shadow in the node's own directory");

        assert!(
            matches!(&refused, ShadowError::Refused(reason)
                if reason.contains("SHADOW_DB_PATH is the directory of the node's own database")),
            "{refused}"
        );
        assert!(refused
            .to_string()
            .contains(&config.db_path.display().to_string()));
        assert!(!cancel.is_cancelled());
    }
}
