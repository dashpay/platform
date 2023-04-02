//! This module implements ABCI application server.
//!
use super::config::AbciConfig;
use crate::error::execution::ExecutionError;
use crate::{config::PlatformConfig, error::Error, platform::Platform, rpc::core::CoreRPCLike};
use drive::grovedb::Transaction;
use drive::query::TransactionArg;
use std::panic::RefUnwindSafe;
use std::sync::{RwLock, RwLockReadGuard};
use std::{fmt::Debug, sync::MutexGuard};

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct AbciApplication<'a, C> {
    pub(crate) platform: Platform<C>,
    transaction: RwLock<Option<Transaction<'a>>>,
}

/// Start ABCI server and process incoming connections.
///
/// Should never return.
pub fn start<C: CoreRPCLike + RefUnwindSafe>(
    config: &PlatformConfig,
    core_rpc: C,
) -> Result<(), Error> {
    let bind_address = config.abci.bind_address.clone();

    let platform: Platform<C> =
        Platform::open_with_client(&config.db_path, Some(config.clone()), core_rpc)?;

    let abci = AbciApplication::new(platform)?;

    let server = tenderdash_abci::start_server(&bind_address, abci)
        .map_err(|e| super::AbciError::from(e))?;

    loop {
        tracing::info!("waiting for new connection");
        let result = std::panic::catch_unwind(|| match server.handle_connection() {
            Ok(_) => (),
            Err(e) => tracing::error!("tenderdash connection terminated: {:?}", e),
        });
        if let Err(_e) = result {
            tracing::error!("panic: connection terminated");
        }
    }
}

impl<'a, C> AbciApplication<'a, C> {
    /// Create new ABCI app
    pub fn new(platform: Platform<C>) -> Result<AbciApplication<'a, C>, Error> {
        let app = AbciApplication {
            platform,
            transaction: RwLock::new(None),
        };

        Ok(app)
    }

    /// create and store a new transaction
    pub(crate) fn start_transaction(&self) {
        let transaction = self.platform.drive.grove.start_transaction();
        self.transaction.write().unwrap().replace(transaction);
    }

    pub(crate) fn commit_transaction(&self) -> Result<(), Error> {
        let transaction = self
            .transaction
            .write()
            .unwrap()
            .take()
            .ok_or(Error::Execution(ExecutionError::NotInTransaction(
                "trying to commit a transaction, but we are not in one",
            )))?;
        self.platform
            .drive
            .commit_transaction(transaction)
            .map_err(Error::Drive)
    }

    pub(crate) fn transaction(&self) -> TransactionArg {
        self.transaction.read().unwrap().as_ref()
    }
}

impl<'a, C> Debug for AbciApplication<'a, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<AbciApp>")
    }
}
