//! This module implements ABCI application server.
//!
use super::config::AbciConfig;
use crate::{config::PlatformConfig, error::Error, platform::Platform};
use dpp::identity::TimestampMillis;
use drive::query::TransactionArg;
use std::sync::MutexGuard;
use tenderdash_abci::proto::abci as proto;
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

    let platform: Platform =
        Platform::open(config.data_dir.to_owned(), Some(config.to_owned())).unwrap();

    let abci = AbciApplication::new(abci_config, &platform)?;

    let server =
        tenderdash_abci::start_server(&bind_address, abci).map_err(|e| super::Error::from(e))?;

    loop {
        match server.handle_connection() {
            Ok(_) => (),
            Err(e) => tracing::error!("tenderdash connection terminated: {:?}", e),
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

    fn platform(&self) -> MutexGuard<&'a Platform> {
        self.platform.lock().unwrap()
    }

    fn transaction(&self) -> TransactionArg {
        self.transaction
    }
}

impl<'a> tenderdash_abci::Application for AbciApplication<'a> {
    fn init_chain(&self, request: proto::RequestInitChain) -> proto::ResponseInitChain {
        let platform = self.platform();
        let transaction = self.transaction();
        let genesis_time = request.time.expect("init chain REQUIRES genesis time");
        let genesis_time = chrono::NaiveDateTime::from_timestamp_opt(
            genesis_time.seconds,
            genesis_time.nanos as u32,
        )
        .unwrap();

        platform
            .create_genesis_state(
                genesis_time.timestamp_millis() as TimestampMillis,
                self.config.keys.clone().into(),
                transaction,
            )
            .expect("create genesis state");

        proto::ResponseInitChain {
            ..Default::default()
        }
    }
}
