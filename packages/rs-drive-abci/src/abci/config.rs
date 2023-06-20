//! Configuration of ABCI Application server

use rand::prelude::StdRng;
use rand::SeedableRng;

use dpp::identity::KeyType::ECDSA_SECP256K1;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

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
    #[serde(rename = "abci_bind_address")]
    pub bind_address: String,

    /// Address to listen for Prometheus connection.
    ///
    /// Optional.
    ///
    /// /// Address should be an URL with scheme `http://`, for example:
    /// - `http://127.0.0.1:29090`
    ///
    /// Port number defaults to [DEFAULT_PROMETHEUS_PORT].
    ///
    /// [DEFAULT_PROMETHEUS_PORT]: crate::metrics::DEFAULT_PROMETHEUS_PORT
    #[serde(default, rename = "abci_prometheus_bind_address")]
    pub prometheus_bind_address: Option<String>,

    /// Public keys used for system identity
    #[serde(flatten)]
    pub keys: Keys,

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
            bind_address: "tcp://127.0.0.1:1234".to_string(),
            prometheus_bind_address: None,
            keys: Keys::new_random_keys_with_seed(18012014), //Dash genesis day
            genesis_height: AbciConfig::default_genesis_height(),
            genesis_core_height: AbciConfig::default_genesis_core_height(),
            chain_id: "chain_id".to_string(),
            log: Default::default(),
        }
    }
}

// @append_only
/// Struct to easily load from environment keys used by the Platform.
///
/// Once loaded, Keys can be easily converted to [SystemIdentityPublicKeys]
///
/// [SystemIdentityPublicKeys]: super::messages::SystemIdentityPublicKeys
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keys {
    // dpns contract
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(crate) dpns_master_public_key: Vec<u8>,
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(crate) dpns_second_public_key: Vec<u8>,

    // dashpay contract
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(crate) dashpay_master_public_key: Vec<u8>,
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(crate) dashpay_second_public_key: Vec<u8>,

    // feature flags contract
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(crate) feature_flags_master_public_key: Vec<u8>,
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(crate) feature_flags_second_public_key: Vec<u8>,

    // masternode reward shares contract
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(crate) masternode_reward_shares_master_public_key: Vec<u8>,
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(crate) masternode_reward_shares_second_public_key: Vec<u8>,

    // withdrawals contract
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(crate) withdrawals_master_public_key: Vec<u8>,
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    pub(crate) withdrawals_second_public_key: Vec<u8>,
}

impl Keys {
    /// Create new random keys for a given seed
    pub fn new_random_keys_with_seed(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        Keys {
            dpns_master_public_key: ECDSA_SECP256K1.random_public_key_data(&mut rng),
            dpns_second_public_key: ECDSA_SECP256K1.random_public_key_data(&mut rng),
            dashpay_master_public_key: ECDSA_SECP256K1.random_public_key_data(&mut rng),
            dashpay_second_public_key: ECDSA_SECP256K1.random_public_key_data(&mut rng),
            feature_flags_master_public_key: ECDSA_SECP256K1.random_public_key_data(&mut rng),
            feature_flags_second_public_key: ECDSA_SECP256K1.random_public_key_data(&mut rng),
            masternode_reward_shares_master_public_key: ECDSA_SECP256K1
                .random_public_key_data(&mut rng),
            masternode_reward_shares_second_public_key: ECDSA_SECP256K1
                .random_public_key_data(&mut rng),
            withdrawals_master_public_key: ECDSA_SECP256K1.random_public_key_data(&mut rng),
            withdrawals_second_public_key: ECDSA_SECP256K1.random_public_key_data(&mut rng),
        }
    }
}
