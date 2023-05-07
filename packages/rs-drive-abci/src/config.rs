// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use dashcore_rpc::json::QuorumType;
use std::path::PathBuf;

use drive::drive::config::DriveConfig;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::abci::config::Keys;
use crate::{abci::config::AbciConfig, error::Error};

/// Configuration for Dash Core RPC client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoreRpcConfig {
    /// Core RPC client hostname or IP address
    #[serde(rename = "core_json_rpc_host")]
    pub host: String,

    // FIXME: fix error  Configuration(Custom("invalid type: string \"9998\", expected i16")) and change port to i16
    /// Core RPC client port number
    #[serde(rename = "core_json_rpc_port")]
    pub port: String,

    /// Core RPC client username
    #[serde(rename = "core_json_rpc_username")]
    pub username: String,

    /// Core RPC client password
    #[serde(rename = "core_json_rpc_password")]
    pub password: String,
}

impl CoreRpcConfig {
    /// Return core address in the `host:port` format.
    pub fn url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl Default for CoreRpcConfig {
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: String::from("1234"),
            username: String::from(""),
            password: String::from(""),
        }
    }
}

/// Configuration for Dash Core related things
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CoreConfig {
    /// Core RPC config
    #[serde(flatten)]
    pub rpc: CoreRpcConfig,

    /// DKG interval
    pub dkg_interval: String, // String due to https://github.com/softprops/envy/issues/26
    /// Minimum number of valid members to use the quorum
    pub min_quorum_valid_members: String, // String due to https://github.com/softprops/envy/issues/26
}

impl CoreConfig {
    /// return dkg_interval
    pub fn dkg_interval(&self) -> u32 {
        self.dkg_interval
            .parse::<u32>()
            .expect("DKG_INTERVAL is not an int")
    }
    /// Returns minimal number of quorum members
    pub fn min_quorum_valid_members(&self) -> u32 {
        self.min_quorum_valid_members
            .parse::<u32>()
            .expect("MIN_QUORUM_VALID_MEMBERS is not an int")
    }
}
impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            dkg_interval: String::from("24"),
            min_quorum_valid_members: String::from("3"),
            rpc: Default::default(),
        }
    }
}

/// Configurtion of Dash Platform.
///
/// All fields in this struct can be configured using environment variables.
/// These variables can also be defined in `.env` file in the current directory
/// or its parents. You can also provide path to the .env file as a command-line argument.
///
/// Environment variables should be renamed to `SCREAMING_SNAKE_CASE`.
/// For example, to define [`verify_sum_trees`], you should set VERIFY_SUM_TREES
/// environment variable:
///
/// ``
/// export VERIFY_SUM_TREES=true
/// ``
///
/// [`verify_sum_trees`]: PlatformConfig::verify_sum_trees
#[derive(Clone, Debug, Serialize, Deserialize)]
// NOTE: in renames, we use lower_snake_case, because uppercase does not work; see
// https://github.com/softprops/envy/issues/61 and https://github.com/softprops/envy/pull/69
pub struct PlatformConfig {
    /// Drive configuration
    #[serde(flatten)]
    pub drive: DriveConfig,

    /// Dash Core config
    #[serde(flatten)]
    pub core: CoreConfig,

    /// ABCI Application Server config
    #[serde(flatten)]
    pub abci: AbciConfig,

    /// Should we verify sum trees? Useful to set as `false` for tests
    #[serde(default = "PlatformConfig::default_verify_sum_trees")]
    pub verify_sum_trees: bool,

    /// The default quorum type
    pub quorum_type: String,

    /// The default quorum size
    pub quorum_size: u16,

    // todo: this should probably be coming from Tenderdash config
    /// Approximately how often are blocks produced
    pub block_spacing_ms: u64,

    /// How often should quorums change?
    pub validator_set_quorum_rotation_block_count: u32,

    /// Path to data storage
    pub db_path: PathBuf,

    // todo: put this in tests like #[cfg(test)]
    /// This should be None, except in the case of Testing platform
    #[serde(skip)]
    pub testing_configs: PlatformTestConfig,
}

impl PlatformConfig {
    // #[allow(unused)]
    fn default_verify_sum_trees() -> bool {
        true
    }

    /// Return type of quorum
    pub fn quorum_type(&self) -> QuorumType {
        let found = if let Ok(t) = self.quorum_type.trim().parse::<u32>() {
            QuorumType::from(t)
        } else {
            QuorumType::from(self.quorum_type.as_str())
        };

        if found == QuorumType::UNKNOWN {
            panic!("config: unsupported QUORUM_TYPE: {}", self.quorum_type);
        }

        found
    }
}
/// create new object using values from environment variables
pub trait FromEnv {
    /// create new object using values from environment variables
    fn from_env() -> Result<Self, Error>
    where
        Self: Sized + DeserializeOwned,
    {
        envy::from_env::<Self>().map_err(Error::from)
    }
}

impl FromEnv for PlatformConfig {}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            verify_sum_trees: true,
            quorum_type: "llmq_100_67".to_string(),
            quorum_size: 100,
            block_spacing_ms: 5000,
            validator_set_quorum_rotation_block_count: 15,
            drive: Default::default(),
            abci: AbciConfig {
                bind_address: "tcp://127.0.0.1:1234".to_string(),
                keys: Keys::new_random_keys_with_seed(18012014), //Dash genesis day
                genesis_height: AbciConfig::default_genesis_height(),
                genesis_core_height: AbciConfig::default_genesis_core_height(),
                chain_id: "chain_id".to_string(),
            },
            core: Default::default(),
            db_path: PathBuf::from("/var/lib/dash-platform/data"),
            testing_configs: PlatformTestConfig::default(),
        }
    }
}

/// Configs that should only happen during testing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlatformTestConfig {
    /// Block signing
    pub block_signing: bool,
    /// Block signature verification
    pub block_commit_signature_verification: bool,
}

impl PlatformTestConfig {
    /// Much faster config for tests
    pub fn default_with_no_block_signing() -> Self {
        Self {
            block_signing: false,
            block_commit_signature_verification: false,
        }
    }
}

impl Default for PlatformTestConfig {
    fn default() -> Self {
        Self {
            block_signing: true,
            block_commit_signature_verification: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FromEnv;
    use dashcore_rpc::dashcore_rpc_json::QuorumType;
    use std::env;

    #[test]
    fn test_config_from_env() {
        let envfile = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env.example");

        dotenvy::from_path(envfile.as_path()).expect("cannot load .env file");
        assert_eq!("5", env::var("QUORUM_SIZE").unwrap());

        let config = super::PlatformConfig::from_env().unwrap();
        assert!(config.verify_sum_trees);
        assert_ne!(config.quorum_type(), QuorumType::UNKNOWN);
    }
}
