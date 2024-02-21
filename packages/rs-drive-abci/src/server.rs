//! This module implements ABCI application server.
//!
mod query;

use crate::abci::app::CheckTxAbciApplication;
use crate::abci::app::{ConsensusAbciApplication, NamedApplication};
use crate::abci::AbciError;
use crate::server::query::QueryServer;
use drive_abci::config::PlatformConfig;
use drive_abci::error::Error;
use drive_abci::platform_types::platform::Platform;
use drive_abci::rpc::core::DefaultCoreRPC;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

/// Start ABCI server and process incoming connections.
///
/// Should only return when server is stopped
pub fn start(
    runtime: &Runtime,
    platform: Arc<Platform<DefaultCoreRPC>>,
    config: PlatformConfig,
    cancel: CancellationToken,
) {
    let query_server = QueryServer::new(Arc::clone(&platform));
    let check_tx_server = CheckTxAbciApplication::new(Arc::clone(&platform));

    let grpc_server = dapi_grpc::tonic::transport::Server::builder()
        .add_service(dapi_grpc::platform::v0::platform_server::PlatformServer::new(query_server))
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

    let cancel_ref = &cancel;

    let platform1 = Arc::clone(&platform);

    thread::scope(|scope| {
        scope.spawn(move || {
            let app = ConsensusAbciApplication::new(platform1.as_ref());
            start_tenderdash_abci_server(app, &config.abci.consensus_bind_address, cancel_ref)
        });
    });
}

fn start_tenderdash_abci_server<'a, A>(app: A, bind_address: &str, cancel: &CancellationToken)
where
    A: 'a + tenderdash_abci::Application + tenderdash_abci::RequestDispatcher + NamedApplication,
{
    let app_name = app.name().to_string();

    let server = tenderdash_abci::ServerBuilder::new(app, bind_address)
        .with_cancel_token(cancel.clone())
        .build()
        .map_err(AbciError::from)
        .expect("failed to build ABCI server");

    while !cancel.is_cancelled() {
        tracing::info!("{} ABCI app is waiting for new connection", app_name);
        match server.next_client() {
            Err(e) => tracing::error!("ABCI connection terminated: {:?}", e),
            Ok(_) => tracing::info!("ABCI connection closed"),
        }
    }
}

fn new_core_rpc(config: &PlatformConfig) -> Result<DefaultCoreRPC, Error> {
    DefaultCoreRPC::open(
        config.core.rpc.url().as_str(),
        config.core.rpc.username.clone(),
        config.core.rpc.password.clone(),
    )
    .map_err(Error::from)
}
