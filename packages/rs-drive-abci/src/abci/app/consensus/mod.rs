mod handlers;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use drive::grovedb::Transaction;
use std::fmt::Debug;
use std::sync::RwLock;

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct ConsensusAbciApplication<'a, C> {
    /// Platform
    pub platform: &'a Platform<C>,
    /// The current transaction
    pub transaction: RwLock<Option<Transaction<'a>>>,
}

impl<'a, C> ConsensusAbciApplication<'a, C> {
    /// Create new ABCI app
    pub fn new(platform: &'a Platform<C>) -> Result<ConsensusAbciApplication<'a, C>, Error> {
        let app = ConsensusAbciApplication {
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

impl<'a, C> Debug for ConsensusAbciApplication<'a, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<AbciApp>")
    }
}
