//! This module implements ABCI application server.
//!
use crate::abci::app::ConsensusAbciApplication;
use crate::abci::app::QueryAbciApplication;
use crate::rpc::core::DefaultCoreRPC;
use crate::{config::PlatformConfig, error::Error, platform_types::platform::Platform};
use std::thread;
use tokio_util::sync::CancellationToken;

/// Start ABCI server and process incoming connections.
///
/// Should never return.
pub fn start(config: &PlatformConfig, cancel: CancellationToken) -> Result<(), Error> {
    // TODO: Do this on main
    let consensus_config = config.clone();
    let read_only_config = config.clone();
    let consensus_cancel = cancel.clone();
    let read_only_cancel = cancel.clone();

    let consensus_app = thread::spawn(move || {
        let core_rpc = DefaultCoreRPC::open(
            consensus_config.core.rpc.url().as_str(),
            consensus_config.core.rpc.username.clone(),
            consensus_config.core.rpc.password.clone(),
        )
        .unwrap();

        let platform: Platform<DefaultCoreRPC> = Platform::open_with_client(
            &consensus_config.primary_db_path,
            Some(consensus_config.clone()),
            core_rpc,
        )
        .expect("Failed to open platform");

        let abci = ConsensusAbciApplication::new(&platform).expect("Failed to create ABCI app");

        let server = tenderdash_abci::ServerBuilder::new(
            abci,
            &consensus_config.abci.consensus_bind_address,
        )
        .with_cancel_token(consensus_cancel.clone())
        .build()
        .map_err(super::AbciError::from)
        .expect("Failed to build ABCI server");

        while !consensus_cancel.is_cancelled() {
            tracing::info!("waiting for new ABCI connection");
            match server.next_client() {
                Err(e) => tracing::error!("ABCI connection terminated: {:?}", e),
                Ok(_) => tracing::info!("ABCI connection closed"),
            }
        }
    });

    thread::sleep(std::time::Duration::from_secs(5));

    let read_only_app = thread::spawn(move || {
        let core_rpc = DefaultCoreRPC::open(
            read_only_config.core.rpc.url().as_str(),
            read_only_config.core.rpc.username.clone(),
            read_only_config.core.rpc.password.clone(),
        )
        .unwrap();

        let platform: Platform<DefaultCoreRPC> = Platform::open_secondary_with_client(
            &read_only_config.primary_db_path,
            &read_only_config.secondary_db_path,
            Some(read_only_config.clone()),
            core_rpc,
        )
        .expect("Failed to open platform");

        let abci = QueryAbciApplication::new(&platform).expect("Failed to create ABCI app");

        let server = tenderdash_abci::ServerBuilder::new(
            abci,
            &read_only_config.abci.read_only_bind_address,
        )
        .with_cancel_token(read_only_cancel.clone())
        .build()
        .map_err(super::AbciError::from)
        .expect("Failed to build ABCI server");

        while !read_only_cancel.is_cancelled() {
            tracing::info!("waiting for new ABCI connection");
            match server.next_client() {
                Err(e) => tracing::error!("ABCI connection terminated: {:?}", e),
                Ok(_) => tracing::info!("ABCI connection closed"),
            }
        }
    });

    read_only_app.join().unwrap();
    consensus_app.join().unwrap();

    Ok(())
}
