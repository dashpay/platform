//! This module implements ABCI application server.
//!
use crate::abci::app::QueryAbciApplication;
use crate::abci::app::{ConsensusAbciApplication, NamedApplication};
use crate::abci::AbciError;
use crate::rpc::core::DefaultCoreRPC;
use crate::{config::PlatformConfig, error::Error, platform_types::platform::Platform};
use std::thread;
use tokio_util::sync::CancellationToken;

/// Start ABCI server and process incoming connections.
///
/// Should only return when server is stopped
pub fn start_abci_apps(config: PlatformConfig, cancel: CancellationToken) -> Result<(), Error> {
    let core_rpc = new_core_rpc(&config).expect("Failed to create core RPC client");

    let platform: Platform<DefaultCoreRPC> =
        Platform::open_with_client(config.db_path.clone(), Some(config.clone()), core_rpc)
            .expect("Failed to open platform");

    let platform_ref = &platform;
    let cancel_ref = &cancel;

    thread::scope(|scope| {
        scope.spawn(move || {
            let app =
                ConsensusAbciApplication::new(platform_ref).expect("Failed to create ABCI app");

            start_tenderdash_abci_server(app, &config.abci.consensus_bind_address, cancel_ref)
        });

        scope.spawn(move || {
            let app = QueryAbciApplication::new(platform_ref).expect("Failed to create ABCI app");

            start_tenderdash_abci_server(app, &config.abci.query_bind_address, cancel_ref)
        });
    });

    Ok(())
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
