//! This module implements ABCI application server.
//!
use crate::error::execution::ExecutionError;
use crate::platform::state_repository::DPPStateRepository;
use crate::platform::PlatformWithBlockContextRef;
use crate::{config::PlatformConfig, error::Error, platform::Platform, rpc::core::CoreRPCLike};
use dpp::state_repository::StateRepositoryLike;
use dpp::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
use dpp::{BlsModule, DPPOptions, DashPlatformProtocol, NativeBlsModule};
use drive::grovedb::Transaction;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct AbciApplication<'a, C, SR, BLS>
where
    SR: StateRepositoryLike + Clone,
    BLS: BlsModule + Clone,
{
    /// Platform
    pub platform: &'a Platform<C>,
    /// The current transaction
    pub transaction: Arc<RwLock<Option<Transaction<'a>>>>,

    /// Dash Platform Protocol instance
    pub dpp: DashPlatformProtocol<SR, BLS>,

    /// Dash Platform Protocol instance for block execution
    pub dpp_transactional: DashPlatformProtocol<SR, BLS>,
}

/// Start ABCI server and process incoming connections.
///
/// Should never return.
pub fn start<C, SR, BLS>(config: &PlatformConfig, core_rpc: C) -> Result<(), Error>
where
    C: CoreRPCLike,
    SR: StateRepositoryLike + Clone,
    BLS: BlsModule + Clone,
{
    let bind_address = config.abci.bind_address.clone();

    let platform = Platform::open_with_client(&config.db_path, Some(config.clone()), core_rpc)?;

    // let abci: AbciApplication<'_, C, SR, NativeBlsModule> =
    //     AbciApplication::<'_, C, SR, BLS>::new(&platform)?;

    let abci = AbciApplication::<'_, C, SR, BLS>::new(&platform)?;

    let server =
        tenderdash_abci::start_server(&bind_address, abci).map_err(super::AbciError::from)?;

    loop {
        tracing::info!("waiting for new connection");
        match server.handle_connection() {
            Err(e) => tracing::error!("tenderdash connection terminated: {:?}", e),
            Ok(_) => tracing::info!("tenderdash connection closed"),
        }
    }
}

impl<'a, C, SR, BLS> AbciApplication<'a, C, SR, BLS>
where
    SR: StateRepositoryLike + Clone,
    BLS: BlsModule + Clone,
{
    /// Create new ABCI app
    pub fn new(
        platform: &'a Platform<C>,
    ) -> Result<AbciApplication<'a, C, DPPStateRepository<C>, NativeBlsModule>, Error>
    where
        C: CoreRPCLike,
    {
        let transaction = Arc::new(RwLock::new(None));

        // Initialize two DPP instances, one is transactional for block execution
        // and the other is not, for check_tx

        let options = DPPOptions::default();

        let adapter = NativeBlsModule::default();

        let platform_ref = Arc::new(PlatformWithBlockContextRef::from(platform));

        let dpp_state_repository = DPPStateRepository::new(platform_ref.clone());
        let dpp_state_repository_transactional =
            DPPStateRepository::with_transaction(platform_ref, transaction.clone());

        let dpp = DashPlatformProtocol::new(
            options.clone(),
            dpp_state_repository.clone(),
            adapter.clone(),
        )?;
        let dpp_transactional =
            DashPlatformProtocol::new(options, dpp_state_repository_transactional, adapter)?;

        let app = AbciApplication {
            platform,
            transaction,
            dpp,
            dpp_transactional,
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
        self.platform
            .drive
            .commit_transaction(transaction)
            .map_err(Error::Drive)
    }
}

impl<'a, C, SR, BLS> Debug for AbciApplication<'a, C, SR, BLS>
where
    SR: StateRepositoryLike + Clone,
    BLS: BlsModule + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<AbciApp>")
    }
}
