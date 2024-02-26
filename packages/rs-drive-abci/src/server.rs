//! This module implements ABCI application server.
//!
mod query;

use crate::abci::app::CheckTxAbciApplication;
use crate::abci::app::ConsensusAbciApplication;
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
use thread_priority::{ThreadPriority, ThreadScopeExt};
use tokio::runtime::{Builder, Runtime};
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

    // TODO: Just limit on enovy side? But in this case we won't limit check_txs
    // TODO: Pass check tx through envoy to limit it as well?
    // TODO: We want check_tx and query should be limited separately
    // TODO: We should limit in both envoy/tenderdash and here
    /*
        clusters:
    - name: my_upstream_service
      connect_timeout: 0.25s
      type: STATIC
      lb_policy: ROUND_ROBIN
      circuit_breakers:
        thresholds:
          - priority: DEFAULT
            max_connections: 100
            max_pending_requests: 100
            max_requests: 100
            max_retries: 3

         */

    // TODO: Limit incoming requests with tower?
    // let layer = tower::ServiceBuilder::new()
    //     //.load_shed() // TODO: Do we want to drop requests or collect but not pass further to app?
    //     .concurrency_limit(num_cpus::get() * 5)
    //     .into_inner();

    let grpc_server = dapi_grpc::tonic::transport::Server::builder()
        // .layer(layer)
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

    // The Consensus ABCI application is handling sequential consensus requests
    // from Tenderdash. Since we don't need concurrency there we use blocking
    // socket-based ABCI server that processes messages sequentially.
    // To avoid blocking and isolate from other applications (queries and check tx)
    // execution we run it in a separate thread and own single-threaded runtime
    //   thread::scope(|s| {
    // The highest priority is set for this thread to prioritize the block producing over queries and check tx
    // Preferably to run Drive ABCI in inside docker in this priority will be set only within
    // other Drive ABCI threads and won't interfere with other running processes on the host machine
    //     s.spawn_with_priority(ThreadPriority::Max, |priority_result| {
    //       if let Err(error) = priority_result {
    //       tracing::warn!("Failed to set priority for consensus thread: {:?}", error)
    //     }

    let app = ConsensusAbciApplication::new(platform.as_ref());

    // TODO: 8 MB stack threads as some recursions in GroveDB can be pretty deep
    //  We could remove such a stack stack size once deletion of a node doesn't recurse in grovedb

    // let consensus_runtime = Builder::new_current_thread()
    //     .thread_stack_size(8 * 1024 * 1024) // TODO: To constant
    //     .thread_name("consensus".to_string())
    //     .enable_all()
    //     .build()
    //     .expect("cannot initialize tokio runtime");

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
