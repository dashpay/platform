//! Configuration of ABCI Application server

use dpp::prelude::TimestampMillis;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// We allow changes in the ABCI configuration, but there should be a social process
// involved in making this change.
// @append_only
/// AbciAppConfig stores configuration of the ABCI Application.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbciConfig {
    /// Address to listen for ABCI connections
    ///
    /// Address should be an URL with scheme `tcp://` or `unix://`, for example:
    /// - `tcp://127.0.0.1:1234`
    /// - `unix:///var/run/abci.sock`
    #[serde(rename = "abci_consensus_bind_address")]
    pub consensus_bind_address: String,

    /// Height of genesis block; defaults to 1
    #[serde(default = "AbciConfig::default_genesis_height")]
    pub genesis_height: u64,

    /// Height of core at genesis
    #[serde(default = "AbciConfig::default_genesis_core_height")]
    pub genesis_core_height: u32,

    /// Chain ID of the network to use
    #[serde(default)]
    pub chain_id: String,

    /// Logging configuration
    // Note it is parsed directly in PlatformConfig::from_env() so here we just set defaults.
    #[serde(default)]
    pub log: crate::logging::LogConfigs,

    /// Maximum time limit (in ms) to process state transitions in block proposals
    #[serde(default = "AbciConfig::default_tx_processing_time_limit")]
    pub tx_processing_time_limit: TimestampMillis,

    /// Directory where files with consensus params updates for a given height, such as `H.json`, are
    /// stored, where `H` is the height of the block for which the consensus params are sent, for example `123456`.
    ///
    /// If empty or not set, consensus params updates are not supported.
    ///
    /// Note that consensus params sent at a height H will be applied at height H+1.
    #[serde(default)]
    pub consensus_params_path: Option<PathBuf>,
}

impl AbciConfig {
    pub(crate) fn default_genesis_height() -> u64 {
        1
    }

    pub(crate) fn default_genesis_core_height() -> u32 {
        1
    }

    pub(crate) fn default_tx_processing_time_limit() -> TimestampMillis {
        8000
    }
}

impl Default for AbciConfig {
    fn default() -> Self {
        Self {
            consensus_bind_address: "tcp://127.0.0.1:1234".to_string(),
            genesis_height: AbciConfig::default_genesis_height(),
            genesis_core_height: AbciConfig::default_genesis_core_height(),
            chain_id: "chain_id".to_string(),
            log: Default::default(),
            tx_processing_time_limit: AbciConfig::default_tx_processing_time_limit(),
            consensus_params_path: None,
        }
    }
}
