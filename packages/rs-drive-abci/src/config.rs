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

use dpp::util::deserializer::ProtocolVersion;
use drive::drive::config::DriveConfig;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::logging::LogConfigs;
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
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CoreConfig {
    /// Core RPC config
    #[serde(flatten)]
    pub rpc: CoreRpcConfig,
}

/// Configuration of the execution part of Dash Platform.
#[derive(Clone, Debug, Serialize, Deserialize)]
// NOTE: in renames, we use lower_snake_case, because uppercase does not work; see
// https://github.com/softprops/envy/issues/61 and https://github.com/softprops/envy/pull/69
pub struct ExecutionConfig {
    /// Should we use document triggers? Useful to set as `false` for tests
    #[serde(default = "ExecutionConfig::default_use_document_triggers")]
    pub use_document_triggers: bool,

    /// Should we verify sum trees? Useful to set as `false` for tests
    #[serde(default = "ExecutionConfig::default_verify_sum_trees")]
    pub verify_sum_trees: bool,

    /// How often should quorums change?
    #[serde(
        default = "ExecutionConfig::default_validator_set_rotation_block_count",
        deserialize_with = "from_str_or_number"
    )]
    pub validator_set_rotation_block_count: u32,

    /// How long in seconds should an epoch last
    /// It might last a lot longer if the chain is halted
    #[serde(
        default = "ExecutionConfig::default_epoch_time_length_s",
        deserialize_with = "from_str_or_number"
    )]
    pub epoch_time_length_s: u64,
}

fn from_str_or_number<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    let s = String::deserialize(deserializer)?;
    s.parse::<T>().map_err(Error::custom)
}

/// Configuration of Dash Platform.
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

    /// Address to listen for Prometheus connection.
    ///
    /// Optional.
    ///
    /// /// Address should be an URL with scheme `http://`, for example:
    /// - `http://127.0.0.1:29090`
    ///
    /// Port number defaults to [crate::metrics::DEFAULT_PROMETHEUS_PORT].
    pub prometheus_bind_address: Option<String>,

    /// Address to listen for gRPC connection.
    pub grpc_bind_address: String,

    /// Multiplier for num cpu to set max number of blocking threads
    /// of the main runtime
    #[serde(default = "default_max_blocking_thread_num_cpu_multiplier")]
    pub max_blocking_thread_num_cpu_multiplier: u8,

    /// Execution config
    #[serde(flatten)]
    pub execution: ExecutionConfig,

    /// The default quorum type
    pub validator_set_quorum_type: String,

    /// The quorum type used for verifying chain locks
    pub chain_lock_quorum_type: String,

    /// The validator set quorum size
    pub validator_set_quorum_size: u16,

    /// The chain lock quorum size
    pub chain_lock_quorum_size: u16,

    /// The window for chain locks
    /// On Mainnet Chain Locks are signed using 400_60: One quorum in every 288 blocks and activeQuorumCount is 4.
    /// On Testnet Chain Locks are signed using 50_60: One quorum in every 24 blocks and activeQuorumCount is 24.
    pub chain_lock_quorum_window: u32,

    // todo: this should probably be coming from Tenderdash config. It's a test only param
    /// Approximately how often are blocks produced
    pub block_spacing_ms: u64,

    /// Initial protocol version
    #[serde(default = "PlatformConfig::default_initial_protocol_version")]
    pub initial_protocol_version: ProtocolVersion,

    /// Path to data storage
    pub db_path: PathBuf,

    // todo: put this in tests like #[cfg(test)]
    /// This should be None, except in the case of Testing platform
    #[serde(skip)]
    pub testing_configs: PlatformTestConfig,
}

impl ExecutionConfig {
    fn default_verify_sum_trees() -> bool {
        true
    }

    fn default_use_document_triggers() -> bool {
        true
    }

    fn default_validator_set_rotation_block_count() -> u32 {
        15
    }

    fn default_epoch_time_length_s() -> u64 {
        788400
    }
}

impl PlatformConfig {
    fn default_initial_protocol_version() -> ProtocolVersion {
        //todo: versioning
        1
    }

    /// Return type of quorum
    pub fn validator_set_quorum_type(&self) -> QuorumType {
        let found = if let Ok(t) = self.validator_set_quorum_type.trim().parse::<u32>() {
            QuorumType::from(t)
        } else {
            QuorumType::from(self.validator_set_quorum_type.as_str())
        };

        if found == QuorumType::UNKNOWN {
            panic!(
                "config: unsupported QUORUM_TYPE: {}",
                self.validator_set_quorum_type
            );
        }

        found
    }

    /// Return type of quorum for validating chain locks
    pub fn chain_lock_quorum_type(&self) -> QuorumType {
        let found = if let Ok(t) = self.chain_lock_quorum_type.trim().parse::<u32>() {
            QuorumType::from(t)
        } else {
            QuorumType::from(self.chain_lock_quorum_type.as_str())
        };

        if found == QuorumType::UNKNOWN {
            panic!(
                "config: unsupported QUORUM_TYPE: {}",
                self.chain_lock_quorum_type
            );
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

impl FromEnv for PlatformConfig {
    fn from_env() -> Result<Self, Error>
    where
        Self: Sized + DeserializeOwned,
    {
        let mut me = envy::from_env::<Self>().map_err(Error::from)?;
        me.abci.log = LogConfigs::from_env()?;

        Ok(me)
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            use_document_triggers: ExecutionConfig::default_use_document_triggers(),
            verify_sum_trees: ExecutionConfig::default_verify_sum_trees(),
            validator_set_rotation_block_count:
                ExecutionConfig::default_validator_set_rotation_block_count(),
            epoch_time_length_s: ExecutionConfig::default_epoch_time_length_s(),
        }
    }
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self::default_mainnet()
    }
}

fn default_max_blocking_thread_num_cpu_multiplier() -> u8 {
    5
}

#[allow(missing_docs)]
impl PlatformConfig {
    pub fn default_local() -> Self {
        Self {
            validator_set_quorum_type: "llmq_test_platform".to_string(),
            chain_lock_quorum_type: "llmq_test".to_string(),
            validator_set_quorum_size: 3,
            chain_lock_quorum_size: 3,
            chain_lock_quorum_window: 24,
            block_spacing_ms: 5000,
            drive: Default::default(),
            abci: Default::default(),
            core: Default::default(),
            execution: Default::default(),
            db_path: PathBuf::from("/var/lib/dash-platform/data"),
            testing_configs: PlatformTestConfig::default(),
            initial_protocol_version: 1,
            prometheus_bind_address: None,
            grpc_bind_address: "0.0.0.0:26670".to_string(),
            max_blocking_thread_num_cpu_multiplier: 5,
        }
    }

    pub fn default_testnet() -> Self {
        Self {
            validator_set_quorum_type: "llmq_25_67".to_string(),
            chain_lock_quorum_type: "llmq_50_60".to_string(),
            validator_set_quorum_size: 25,
            chain_lock_quorum_size: 50,
            chain_lock_quorum_window: 24,
            block_spacing_ms: 5000,
            drive: Default::default(),
            abci: Default::default(),
            core: Default::default(),
            execution: Default::default(),
            db_path: PathBuf::from("/var/lib/dash-platform/data"),
            testing_configs: PlatformTestConfig::default(),
            initial_protocol_version: 1,
            prometheus_bind_address: None,
            grpc_bind_address: "0.0.0.0:26670".to_string(),
            max_blocking_thread_num_cpu_multiplier: 5,
        }
    }

    pub fn default_mainnet() -> Self {
        Self {
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_400_60".to_string(),
            validator_set_quorum_size: 100,
            chain_lock_quorum_size: 400,
            chain_lock_quorum_window: 288,
            block_spacing_ms: 5000,
            drive: Default::default(),
            abci: Default::default(),
            core: Default::default(),
            execution: Default::default(),
            db_path: PathBuf::from("/var/lib/dash-platform/data"),
            testing_configs: PlatformTestConfig::default(),
            initial_protocol_version: 1,
            prometheus_bind_address: None,
            grpc_bind_address: "0.0.0.0:26670".to_string(),
            max_blocking_thread_num_cpu_multiplier: 5,
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
    use crate::logging::LogDestination;
    use dashcore_rpc::dashcore_rpc_json::QuorumType;
    use std::env;

    #[test]
    fn test_config_from_env() {
        // ABCI log configs are parsed manually, so they deserve separate handling
        // Note that STDOUT is also defined in .env.example, but env var should overwrite it.
        let vectors = &[
            ("STDOUT", "pretty"),
            ("UPPERCASE", "json"),
            ("lowercase", "pretty"),
            ("miXedC4s3", "full"),
            ("123", "compact"),
        ];
        for vector in vectors {
            env::set_var(format!("ABCI_LOG_{}_DESTINATION", vector.0), "bytes");
            env::set_var(format!("ABCI_LOG_{}_FORMAT", vector.0), vector.1);
        }

        let envfile = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env.local");

        dotenvy::from_path(envfile.as_path()).expect("cannot load .env file");
        assert_eq!("/tmp/db", env::var("DB_PATH").unwrap());

        let config = super::PlatformConfig::from_env().expect("expected config from env");
        assert!(config.execution.verify_sum_trees);
        assert_ne!(config.validator_set_quorum_type(), QuorumType::UNKNOWN);
        for id in vectors {
            matches!(config.abci.log[id.0].destination, LogDestination::Bytes);
        }
    }
}
