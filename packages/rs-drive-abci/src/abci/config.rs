//! Configuration of ABCI Application server

use crate::utils::from_opt_str_or_number;
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

    /// Maximum time limit (in ms) to process state transitions to prepare proposal
    #[serde(default, deserialize_with = "from_opt_str_or_number")]
    pub proposer_tx_processing_time_limit: Option<u16>,
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
            proposer_tx_processing_time_limit: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_genesis_height_returns_one() {
        assert_eq!(AbciConfig::default_genesis_height(), 1);
    }

    #[test]
    fn default_genesis_core_height_returns_one() {
        assert_eq!(AbciConfig::default_genesis_core_height(), 1);
    }

    #[test]
    fn default_config_has_expected_values() {
        let config = AbciConfig::default();
        assert_eq!(config.consensus_bind_address, "tcp://127.0.0.1:1234");
        assert_eq!(config.genesis_height, 1);
        assert_eq!(config.genesis_core_height, 1);
        assert_eq!(config.chain_id, "chain_id");
        assert!(config.log.is_empty());
        assert!(config.proposer_tx_processing_time_limit.is_none());
    }

    #[test]
    fn config_serializes_and_deserializes_roundtrip() {
        // proposer_tx_processing_time_limit uses a custom string-only deserializer,
        // so roundtrip only works when the field is None (serializes as null)
        let config = AbciConfig {
            consensus_bind_address: "tcp://0.0.0.0:5678".to_string(),
            genesis_height: 42,
            genesis_core_height: 10,
            chain_id: "test-chain".to_string(),
            log: Default::default(),
            proposer_tx_processing_time_limit: None,
        };

        let serialized = serde_json::to_string(&config).expect("should serialize");
        let deserialized: AbciConfig =
            serde_json::from_str(&serialized).expect("should deserialize");

        assert_eq!(deserialized.consensus_bind_address, "tcp://0.0.0.0:5678");
        assert_eq!(deserialized.genesis_height, 42);
        assert_eq!(deserialized.genesis_core_height, 10);
        assert_eq!(deserialized.chain_id, "test-chain");
        assert!(deserialized.proposer_tx_processing_time_limit.is_none());
    }

    #[test]
    fn config_deserialization_uses_defaults_for_missing_fields() {
        // Only the renamed field is mandatory; all others have defaults
        let json = r#"{"abci_consensus_bind_address": "tcp://1.2.3.4:9999"}"#;
        let config: AbciConfig = serde_json::from_str(json).expect("should deserialize");

        assert_eq!(config.consensus_bind_address, "tcp://1.2.3.4:9999");
        assert_eq!(config.genesis_height, 1);
        assert_eq!(config.genesis_core_height, 1);
        assert_eq!(config.chain_id, "");
        assert!(config.proposer_tx_processing_time_limit.is_none());
    }

    #[test]
    fn config_serialization_uses_renamed_field() {
        let config = AbciConfig::default();
        let serialized = serde_json::to_value(&config).expect("should serialize");

        // The field should be serialized as "abci_consensus_bind_address"
        assert!(serialized.get("abci_consensus_bind_address").is_some());
        assert!(serialized.get("consensus_bind_address").is_none());
    }

    #[test]
    fn config_clone_produces_equal_values() {
        let config = AbciConfig {
            consensus_bind_address: "unix:///tmp/test.sock".to_string(),
            genesis_height: 100,
            genesis_core_height: 50,
            chain_id: "clone-test".to_string(),
            log: Default::default(),
            proposer_tx_processing_time_limit: Some(1000),
        };

        let cloned = config.clone();
        assert_eq!(cloned.consensus_bind_address, config.consensus_bind_address);
        assert_eq!(cloned.genesis_height, config.genesis_height);
        assert_eq!(cloned.genesis_core_height, config.genesis_core_height);
        assert_eq!(cloned.chain_id, config.chain_id);
        assert_eq!(
            cloned.proposer_tx_processing_time_limit,
            config.proposer_tx_processing_time_limit
        );
    }

    #[test]
    fn config_debug_format_is_not_empty() {
        let config = AbciConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(!debug_str.is_empty());
        assert!(debug_str.contains("AbciConfig"));
    }

    #[test]
    fn config_deserializes_proposer_tx_processing_time_limit_from_string() {
        // The from_opt_str_or_number deserializer should accept string values
        let json = r#"{"abci_consensus_bind_address": "tcp://x:1", "proposer_tx_processing_time_limit": "250"}"#;
        let config: AbciConfig = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(config.proposer_tx_processing_time_limit, Some(250));
    }

    #[test]
    fn config_rejects_raw_number_for_proposer_tx_processing_time_limit() {
        // The custom from_opt_str_or_number deserializer only accepts strings,
        // not raw JSON numbers
        let json = r#"{"abci_consensus_bind_address": "tcp://x:1", "proposer_tx_processing_time_limit": 750}"#;
        let result = serde_json::from_str::<AbciConfig>(json);
        assert!(result.is_err());
    }

    #[test]
    fn config_deserializes_null_proposer_tx_processing_time_limit() {
        let json = r#"{"abci_consensus_bind_address": "tcp://x:1", "proposer_tx_processing_time_limit": null}"#;
        let config: AbciConfig = serde_json::from_str(json).expect("should deserialize");
        assert!(config.proposer_tx_processing_time_limit.is_none());
    }

    #[test]
    fn config_deserializes_empty_string_as_none_for_proposer_tx_processing_time_limit() {
        let json = r#"{"abci_consensus_bind_address": "tcp://x:1", "proposer_tx_processing_time_limit": ""}"#;
        let config: AbciConfig = serde_json::from_str(json).expect("should deserialize");
        assert!(config.proposer_tx_processing_time_limit.is_none());
    }
}
