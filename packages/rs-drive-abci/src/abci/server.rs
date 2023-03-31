//! This module implements ABCI application server.
//!
use super::config::AbciConfig;
use crate::{
    config::PlatformConfig, error::Error, platform::Platform,
    rpc::core::CoreRPCLike,
};
use dpp::identity::TimestampMillis;
use dpp::state_transition::StateTransition;
use drive::query::TransactionArg;
use std::{fmt::Debug, sync::MutexGuard};
use tenderdash_abci::proto::abci::{
    RequestCheckTx, RequestProcessProposal, ResponseCheckTx, ResponseProcessProposal,
};
use tenderdash_abci::proto::{
    abci::{self as proto, ResponseException},
    serializers::timestamp::ToMilis,
};

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct AbciApplication<'a, C> {
    platform: std::sync::Mutex<&'a Platform<'a, C>>,
    transaction: TransactionArg<'a, 'a>,
    pub(crate) config: AbciConfig,
}

/// Start ABCI server and process incoming connections.
///
/// Should never return.
pub fn start<C: CoreRPCLike>(config: &PlatformConfig, core_rpc: C) -> Result<(), Error> {
    let bind_address = config.abci.bind_address.clone();
    let abci_config = config.abci.clone();

    let platform: Platform<C> =
        Platform::open_with_client(&config.db_path, Some(config.clone()), core_rpc)?;

    let abci = AbciApplication::new(abci_config, &platform)?;

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
    pub fn new(
        config: AbciConfig,
        platform: &'a Platform<'a, C>,
    ) -> Result<AbciApplication<'a, C>, Error> {
        let app = AbciApplication {
            platform: std::sync::Mutex::new(platform),
            transaction: None,
            config,
        };

        Ok(app)
    }

    /// Return locked Platform object
    pub(crate) fn platform(&self) -> MutexGuard<&'a Platform<'a, C>> {
        self.platform
            .lock()
            .expect("cannot acquire lock on platform")
    }

    /// Return current transaction.
    /// TODO: implement
    pub(crate) fn transaction(&self) -> TransactionArg {
        self.transaction
    }
}

impl<'a, C> Debug for AbciApplication<'a, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<AbciApp>")
    }
}
