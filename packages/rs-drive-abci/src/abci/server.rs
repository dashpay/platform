//! This module implements ABCI application server.
//!
use super::config::AbciConfig;
use crate::{abci::proposal::Proposal, config::PlatformConfig, error::Error, platform::Platform};
use dpp::identity::TimestampMillis;
use drive::query::TransactionArg;
use std::{fmt::Debug, sync::MutexGuard};
use tenderdash_abci::proto::{abci as proto, serializers::timestamp::ToMilis};
use tracing::debug;
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
        let result = std::panic::catch_unwind(|| match server.handle_connection() {
            Ok(_) => (),
            Err(e) => tracing::error!("tenderdash connection terminated: {:?}", e),
        });
        if let Err(_e) = result {
            tracing::error!("panic: connection terminated");
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
    fn info(&self, request: proto::RequestInfo) -> proto::ResponseInfo {
        if !check_version(&request.abci_version, tenderdash_abci::proto::ABCI_VERSION) {
            panic!(
                "SemVer mismatch: Tenderdash requires ABCI version {}, our version is {}",
                request.version,
                tenderdash_abci::proto::ABCI_VERSION
            );
        }

        let response = proto::ResponseInfo {
            app_version: 1,
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        };

        tracing::info!(method = "info", ?request, ?response, "info executed");
        response
    }

    fn init_chain(&self, request: proto::RequestInitChain) -> proto::ResponseInitChain {
        let platform = self.platform();
        let transaction = self.transaction();
        let genesis_time =
            request.time.expect("genesis time is required").to_milis() as TimestampMillis;

        platform
            .create_genesis_state(genesis_time, self.config.keys.clone().into(), transaction)
            .expect("create genesis state");

        let response = proto::ResponseInitChain {
            ..Default::default()
        };

        tracing::info!(method = "init_chain", "init chain executed");
        response
    }

    fn prepare_proposal(
        &self,
        request: proto::RequestPrepareProposal,
    ) -> proto::ResponsePrepareProposal {
        let platform = self.platform();
        let transaction = self.transaction();
        let response = platform
            .prepare_proposal(&request, transaction)
            .expect("failed to prepare proposal");

        tracing::info!(
            method = "prepare_proposal",
            height = request.height,
            "prepare proposal executed",
        );
        response
    }
}

/// Check if ABCI version required by Tenderdash matches our protobuf version.
///
/// Match is determined based on Semantic Versioning rules, as defined for '^' operator.
fn check_version(tenderdash_abci_requirement: &str, our_abci_version: &str) -> bool {
    let our_version =
        semver::Version::parse(our_abci_version).expect("cannot parse protobuf library version");

    let require = String::from("^") + tenderdash_abci_requirement;
    let td_version =
        semver::VersionReq::parse(require.as_str()).expect("cannot parse tenderdash version");

    debug!("ABCI version: required: {}, our: {}", require, our_version);

    td_version.matches(&our_version)
}

#[cfg(test)]
mod tests {
    use crate::abci::server::check_version;

    /// test_versions! {} (td_version, our_version, expected); }
    macro_rules! test_versions {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (td, our, expect) = $value;
                assert_eq!(check_version(td, our),expect);
            }
        )*
        }
    }

    test_versions! {
        test_versions_td_newer: ("0.1.2-dev.1", "0.1.0", false),
        test_versions_equal: ("0.1.0","0.1.0",true),
        test_versions_td_older: ("0.1.0","0.1.2",true),
        test_versions_equal_dev: ("0.1.0-dev.1","0.1.0-dev.1",true),
        test_versions_our_newer_dev: ("0.1.0-dev.1", "0.1.0-dev.2",true),
        test_versions_our_dev:("0.1.0","0.1.0-dev.1",false),
    }
}
