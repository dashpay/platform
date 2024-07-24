//! Configuration of ABCI Application server

use serde::{Deserialize, Serialize};

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
}

impl AbciConfig {
    pub(crate) fn default_genesis_height() -> u64 {
        1
    }

    pub(crate) fn default_genesis_core_height() -> u32 {
        1
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
        }
    }
}
