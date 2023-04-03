//! This module implements ABCI application server.
//!
use super::config::AbciConfig;
use crate::{abci::proposal::Proposal, config::PlatformConfig, error::Error, platform::Platform};
use dpp::identity::TimestampMillis;
use drive::query::TransactionArg;
use std::{fmt::Debug, sync::MutexGuard};
use tenderdash_abci::proto::{
    abci::{self as proto, ResponseException},
    serializers::timestamp::ToMilis,
};
/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct AbciApplication<'a> {
    platform: std::sync::Mutex<&'a Platform>,
    transaction: TransactionArg<'a, 'a>,
    config: AbciConfig,
}

/// Start ABCI server and process incoming connections.
///
/// Should never return.
pub fn start(config: &PlatformConfig) -> Result<(), Error> {
    let bind_address = config.abci.bind_address.clone();
    let abci_config = config.abci.clone();

    let platform: Platform = Platform::open(&config.db_path, Some(config.clone()))?;

    let abci = AbciApplication::new(abci_config, &platform)?;

    let server = tenderdash_abci::start_server(&bind_address, abci)
        .map_err(|e| super::AbciError::from(e))?;

    loop {
        tracing::info!("waiting for new connection");
        match server.handle_connection() {
            Err(e) => tracing::error!("tenderdash connection terminated: {:?}", e),
            Ok(_) => tracing::info!("tenderdash connection closed"),
        }
    }
}

impl<'a> AbciApplication<'a> {
    /// Create new ABCI app
    pub fn new(config: AbciConfig, platform: &'a Platform) -> Result<AbciApplication<'a>, Error> {
        let app = AbciApplication {
            platform: std::sync::Mutex::new(platform),
            transaction: None,
            config,
        };

        Ok(app)
    }

    /// Return locked Platform object
    fn platform(&self) -> MutexGuard<&'a Platform> {
        self.platform
            .lock()
            .expect("cannot acquire lock on platform")
    }

    /// Return current transaction.
    /// TODO: implement
    fn transaction(&self) -> TransactionArg {
        self.transaction
    }
}

impl<'a> Debug for AbciApplication<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<AbciApp>")
    }
}

impl<'a> tenderdash_abci::Application for AbciApplication<'a> {
    fn info(&self, request: proto::RequestInfo) -> Result<proto::ResponseInfo, ResponseException> {
        if !tenderdash_abci::check_version(&request.abci_version) {
            return Err(ResponseException::from(format!(
                "tenderdash requires ABCI version {}, our version is {}",
                request.version,
                tenderdash_abci::proto::ABCI_VERSION
            )));
        }

        let response = proto::ResponseInfo {
            app_version: 1,
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        };

        tracing::info!(method = "info", ?request, ?response, "info executed");
        Ok(response)
    }

    fn init_chain(
        &self,
        request: proto::RequestInitChain,
    ) -> Result<proto::ResponseInitChain, ResponseException> {
        let platform = self.platform();
        let transaction = self.transaction();
        let genesis_time = request
            .time
            .ok_or("genesis time is required in init chain")?
            .to_milis() as TimestampMillis;

        platform.create_genesis_state(
            genesis_time,
            self.config.keys.clone().into(),
            transaction,
        )?;

        let response = proto::ResponseInitChain {
            ..Default::default()
        };

        tracing::info!(method = "init_chain", "init chain executed");
        Ok(response)
    }

    fn prepare_proposal(
        &self,
        request: proto::RequestPrepareProposal,
    ) -> Result<proto::ResponsePrepareProposal, ResponseException> {
        let platform = self.platform();
        let transaction = self.transaction();
        let response = platform.prepare_proposal(&request, transaction)?;

        tracing::info!(
            method = "prepare_proposal",
            height = request.height,
            "prepare proposal executed",
        );
        Ok(response)
    }
}
