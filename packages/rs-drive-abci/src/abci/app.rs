//! This module implements ABCI application server.
//!
use super::messages::SystemIdentityPublicKeys;
use crate::{error::Error, platform::Platform};
use dpp::identity::TimestampMillis;
use drive::query::TransactionArg;
use std::sync::MutexGuard;
use tenderdash_abci::{proto::abci as proto, start_server, BindAddress, Server};

/// AbciAppConfig stores configuration of the ABCI Application.
#[allow(dead_code)]
pub struct AbciAppConfig {
    bind_address: BindAddress,
    system_identity_public_keys: SystemIdentityPublicKeys,
}

/// AbciApp is an implementation of ABCI Application, as defined by Tenderdash.
///
/// AbciApp implements logic that should be triggered when Tenderdash performs various operations, like
/// creating new proposal or finalizing new block.
pub struct AbciApp<'a> {
    platform: std::sync::Mutex<&'a Platform>,
    transaction: TransactionArg<'a, 'a>,
    config: &'a AbciAppConfig,
    // TODO: read from configuation?
}

impl<'a> AbciApp<'a> {
    /// Create new ABCI app
    pub fn new(config: &'a AbciAppConfig, platform: &'a Platform) -> Result<AbciApp<'a>, Error> {
        let app = AbciApp {
            platform: std::sync::Mutex::new(platform),
            transaction: None,
            config,
        };

        Ok(app)
    }

    /// Create ABCI Server for this application
    pub fn bind(self) -> Result<Box<dyn Server + 'a>, tenderdash_abci::Error> {
        start_server(&self.config.bind_address, self)
    }

    fn platform(&self) -> MutexGuard<&'a Platform> {
        self.platform.lock().unwrap()
    }

    fn transaction(&self) -> TransactionArg {
        self.transaction
    }
}

impl<'a> tenderdash_abci::Application for AbciApp<'a> {
    fn init_chain(&self, request: proto::RequestInitChain) -> proto::ResponseInitChain {
        let platform = self.platform();
        let transaction = self.transaction();
        let genesis_time = request.time.expect("init chain REQUIRES genesis time");
        let genesis_time = chrono::NaiveDateTime::from_timestamp_opt(
            genesis_time.seconds,
            genesis_time.nanos as u32,
        )
        .unwrap();
        let keys = self.config.system_identity_public_keys.to_owned();
        platform
            .create_genesis_state(
                genesis_time.timestamp_millis() as TimestampMillis,
                keys,
                transaction,
            )
            .expect("create genesis state");

        proto::ResponseInitChain {
            ..Default::default()
        }
    }
}
