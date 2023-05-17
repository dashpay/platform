//! Configuration of ABCI Application server

use crate::config::FromEnv;
use rand::prelude::StdRng;
use rand::SeedableRng;

use dpp::identity::KeyType::ECDSA_SECP256K1;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use super::messages::{RequiredIdentityPublicKeysSet, SystemIdentityPublicKeys};

/// AbciAppConfig stores configuration of the ABCI Application.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbciConfig {
    /// Address to listen on
    ///
    /// Address should be an URL with scheme `tcp://` or `unix://`, for example:
    /// - `tcp://127.0.0.1:1234`
    /// - `unix:///var/run/abci.sock`
    #[serde(rename = "abci_bind_address")]
    pub bind_address: String,

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
}

impl AbciConfig {
    pub(crate) fn default_genesis_height() -> u64 {
        1
    }

    pub(crate) fn default_genesis_core_height() -> u32 {
        1
    }
}

impl FromEnv for AbciConfig {}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]

/// Struct to easily load from environment keys used by the Platform.
///
/// Once loaded, Keys can be easily converted to [SystemIdentityPublicKeys]
///
/// [SystemIdentityPublicKeys]: super::messages::SystemIdentityPublicKeys
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

impl From<Keys> for SystemIdentityPublicKeys {
    fn from(keys: Keys) -> Self {
        Self {
            masternode_reward_shares_contract_owner: RequiredIdentityPublicKeysSet {
                master: keys.masternode_reward_shares_master_public_key,
                high: keys.masternode_reward_shares_second_public_key,
            },
            feature_flags_contract_owner: RequiredIdentityPublicKeysSet {
                master: keys.feature_flags_master_public_key,
                high: keys.feature_flags_second_public_key,
            },
            dpns_contract_owner: RequiredIdentityPublicKeysSet {
                master: keys.dpns_master_public_key,
                high: keys.dpns_second_public_key,
            },
            withdrawals_contract_owner: RequiredIdentityPublicKeysSet {
                master: keys.withdrawals_master_public_key,
                high: keys.withdrawals_second_public_key,
            },
            dashpay_contract_owner: RequiredIdentityPublicKeysSet {
                master: keys.dashpay_master_public_key,
                high: keys.dashpay_second_public_key,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::FromEnv;

    #[test]
    fn test_config_from_env() {
        let envfile = format!("{}/.env.example", env!("CARGO_MANIFEST_DIR"));
        let envfile = std::path::PathBuf::from(envfile);

        dotenvy::from_path(envfile.as_path()).expect("cannot load .env file");

        let _config = super::AbciConfig::from_env().unwrap();
    }
}
