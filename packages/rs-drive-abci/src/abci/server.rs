//! This module implements ABCI application server.
//!
use crate::abci::app::block_update::BlockUpdateChannel;
use crate::abci::app::QueryAbciApplication;
use crate::abci::app::{ConsensusAbciApplication, NamedApplication};
use crate::abci::AbciError;
use crate::rpc::core::DefaultCoreRPC;
use crate::{config::PlatformConfig, error::Error, platform_types::platform::Platform};
use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use std::{any, thread};
use tokio_util::sync::CancellationToken;

/// Start ABCI server and process incoming connections.
///
/// Should only return when server is stopped
pub fn start_abci_apps(config: PlatformConfig, cancel: CancellationToken) -> Result<(), Error> {
    let drive_semaphore = Arc::new((Mutex::new(false), Condvar::new()));

    let config = Arc::new(config);

    let block_update_channel = Arc::new(BlockUpdateChannel::default());

    let consensus_thread = spawn_consensus_thread(
        Arc::clone(&config),
        cancel.clone(),
        Arc::clone(&drive_semaphore),
        Arc::clone(&block_update_channel),
    );

    let query_thread = spawn_query_thread(
        Arc::clone(&config),
        cancel.clone(),
        Arc::clone(&drive_semaphore),
        Arc::clone(&block_update_channel),
    );

    consensus_thread.join().unwrap();
    query_thread.join().unwrap();

    Ok(())
}

fn spawn_consensus_thread(
    config: Arc<PlatformConfig>,
    cancel: CancellationToken,
    drive_semaphore: Arc<(Mutex<bool>, Condvar)>,
    block_update_channel: Arc<BlockUpdateChannel>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let core_rpc = new_core_rpc(&config).expect("Failed to create core RPC client");

        // Make sure primary Drive is open before the secondary Drive
        let (ref primary_drive_lock, ref drive_notifier) = &*drive_semaphore;
        let mut is_primary_drive_open = primary_drive_lock.lock();

        let platform: Platform<DefaultCoreRPC> = Platform::open_with_client(
            config.primary_db_path.clone(),
            Some((*config).clone()),
            core_rpc,
        )
        .expect("Failed to open platform");

        // Unlock other threads that use secondary Drive
        *is_primary_drive_open = true;
        drive_notifier.notify_all();
        drop(is_primary_drive_open);
        drop(drive_semaphore);

        let app = ConsensusAbciApplication::new(&platform, block_update_channel)
            .expect("Failed to create ABCI app");

        start_tenderdash_abci_server(app, &config.abci.consensus_bind_address, cancel)
    })
}

fn spawn_query_thread(
    config: Arc<PlatformConfig>,
    cancel: CancellationToken,
    drive_semaphore: Arc<(Mutex<bool>, Condvar)>,
    block_update_channel: Arc<BlockUpdateChannel>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let core_rpc = new_core_rpc(&config).expect("Failed to create core RPC client");

        // Wait until primary Drive is open
        let (ref primary_drive_lock, ref drive_notifier) = &*drive_semaphore;
        let mut is_primary_drive_open = primary_drive_lock.lock();
        if !*is_primary_drive_open {
            drive_notifier.wait(&mut is_primary_drive_open);
        }

        let platform: Platform<DefaultCoreRPC> = Platform::open_secondary_with_client(
            &config.primary_db_path,
            &config.secondary_db_path,
            Some((*config).clone()),
            core_rpc,
        )
        .expect("Failed to open platform");

        let app = QueryAbciApplication::new(&platform, block_update_channel)
            .expect("Failed to create ABCI app");

        start_tenderdash_abci_server(app, &config.abci.query_bind_address, cancel)
    })
}

fn start_tenderdash_abci_server<A: tenderdash_abci::Application + NamedApplication>(
    app: A,
    bind_address: &str,
    cancel: CancellationToken,
) {
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

fn new_core_rpc(config: &Arc<PlatformConfig>) -> Result<DefaultCoreRPC, Error> {
    DefaultCoreRPC::open(
        config.core.rpc.url().as_str(),
        config.core.rpc.username.clone(),
        config.core.rpc.password.clone(),
    )
    .map_err(Error::from)
}
