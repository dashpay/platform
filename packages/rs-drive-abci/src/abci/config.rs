//! Configuration of ABCI Application server

use crate::config::FromEnv;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use super::messages::{RequiredIdentityPublicKeysSet, SystemIdentityPublicKeys};

/// AbciAppConfig stores configuration of the ABCI Application.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AbciConfig {
    /// Address to listen on
    ///
    /// Address should be an URL with scheme `tcp://` or `unix://`, for example:
    /// - `tcp://127.0.0.1:1234`
    /// - `unix:///var/run/abci.sock`
    #[serde(rename = "abci_bind_address")]
    pub bind_address: String,
    /// Public keys used for system identity
    ///
    #[serde(flatten)]
    pub keys: ContractKeys,

    /// Height of genesis block; defaults to 1
    #[serde(default = "AbciConfig::default_genesis_height")]
    pub genesis_height: i64,
}

impl AbciConfig {
    fn default_genesis_height() -> i64 {
        1
    }
}

impl FromEnv for AbciConfig {}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]

/// Various keys
pub struct ContractKeys {
    // dpns contract
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    dpns_master_public_key: Vec<u8>,
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    dpns_second_public_key: Vec<u8>,

    // dashpay contract
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    dashpay_master_public_key: Vec<u8>,
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    dashpay_second_public_key: Vec<u8>,

    // feature flags contract
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    feature_flags_master_public_key: Vec<u8>,
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    feature_flags_second_public_key: Vec<u8>,

    // masternode reward shares contract
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    masternode_reward_shares_master_public_key: Vec<u8>,
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    masternode_reward_shares_second_public_key: Vec<u8>,

    // withdrawals contract
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    withdrawals_master_public_key: Vec<u8>,
    /// hex-encoded
    #[serde_as(as = "serde_with::hex::Hex")]
    withdrawals_second_public_key: Vec<u8>,
}

impl From<ContractKeys> for SystemIdentityPublicKeys {
    fn from(keys: ContractKeys) -> Self {
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
        dbg!(&envfile);

        dotenvy::from_path(envfile.as_path()).expect("cannot load .env file");

        let config = super::AbciConfig::from_env().unwrap();
        dbg!(config.keys.dashpay_master_public_key);
    }
}
