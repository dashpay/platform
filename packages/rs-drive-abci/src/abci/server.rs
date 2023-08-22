//! This module implements ABCI application server.
//!
use crate::error::execution::ExecutionError;
use crate::{
    config::PlatformConfig, error::Error, platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};
use drive::grovedb::Transaction;
use std::fmt::Debug;
use std::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct AbciApplication<'a, C> {
    /// Platform
    pub platform: &'a Platform<C>,
    /// The current transaction
    pub transaction: RwLock<Option<Transaction<'a>>>,
}

/// Start ABCI server and process incoming connections.
///
/// Should never return.
pub fn start<C: CoreRPCLike>(
    config: &PlatformConfig,
    core_rpc: C,
    cancel: CancellationToken,
) -> Result<(), Error> {
    let bind_address = config.abci.bind_address.clone();

    let platform: Platform<C> =
        Platform::open_with_client(&config.db_path, Some(config.clone()), core_rpc)?;

    let abci = AbciApplication::new(&platform)?;

    let server = tenderdash_abci::ServerBuilder::new(abci, &bind_address)
        .with_cancel_token(cancel.clone())
        .build()
        .map_err(super::AbciError::from)?;

    while !cancel.is_cancelled() {
        tracing::info!("waiting for new connection");
        match server.next_client() {
            Err(e) => tracing::error!("tenderdash connection terminated: {:?}", e),
            Ok(_) => tracing::info!("tenderdash connection closed"),
        }
    }

    Ok(())
}

impl<'a, C> AbciApplication<'a, C> {
    /// Create new ABCI app
    pub fn new(platform: &'a Platform<C>) -> Result<AbciApplication<'a, C>, Error> {
        let app = AbciApplication {
            platform,
            transaction: RwLock::new(None),
        };

        Ok(app)
    }

    /// create and store a new transaction
    pub fn start_transaction(&self) {
        let transaction = self.platform.drive.grove.start_transaction();
        self.transaction.write().unwrap().replace(transaction);
    }

    /// Commit a transaction
    pub fn commit_transaction(&self) -> Result<(), Error> {
        let transaction = self
            .transaction
            .write()
            .unwrap()
            .take()
            .ok_or(Error::Execution(ExecutionError::NotInTransaction(
                "trying to commit a transaction, but we are not in one",
            )))?;
        let platform_state = self.platform.state.read().unwrap();
        let platform_version = platform_state.current_platform_version()?;
        self.platform
            .drive
            .commit_transaction(transaction, &platform_version.drive)
            .map_err(Error::Drive)
    }
}

impl<'a, C> Debug for AbciApplication<'a, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<AbciApp>")
    }
}
