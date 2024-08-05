use crate::logging::LogConfigs;
use crate::{abci::config::AbciConfig, error::Error};
use bincode::{Decode, Encode};
use dashcore_rpc::json::QuorumType;
use dpp::dashcore::Network;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::INITIAL_PROTOCOL_VERSION;
use drive::config::DriveConfig;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

/// Configuration for Dash Core RPC client used in consensus logic
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ConsensusCoreRpcConfig {
    /// Core RPC client hostname or IP address
    #[serde(rename = "core_consensus_json_rpc_host")]
    pub host: String,

    /// Core RPC client port number
    #[serde(
        rename = "core_consensus_json_rpc_port",
        deserialize_with = "from_str_or_number"
    )]
    pub port: u16,

    /// Core RPC client username
    #[serde(rename = "core_consensus_json_rpc_username")]
    pub username: String,

    /// Core RPC client password
    #[serde(rename = "core_consensus_json_rpc_password")]
    pub password: String,
}

impl ConsensusCoreRpcConfig {
    /// Return core address in the `host:port` format.
    pub fn url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Configuration for Dash Core RPC client used in check tx
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CheckTxCoreRpcConfig {
    /// Core RPC client hostname or IP address
    #[serde(rename = "core_check_tx_json_rpc_host")]
    pub host: String,

    /// Core RPC client port number
    #[serde(
        rename = "core_check_tx_json_rpc_port",
        deserialize_with = "from_str_or_number"
    )]
    pub port: u16,

    /// Core RPC client username
    #[serde(rename = "core_check_tx_json_rpc_username")]
    pub username: String,

    /// Core RPC client password
    #[serde(rename = "core_check_tx_json_rpc_password")]
    pub password: String,
}

impl CheckTxCoreRpcConfig {
    /// Return core address in the `host:port` format.
    pub fn url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Configuration for Dash Core related things
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CoreConfig {
    /// Core RPC config for consensus
    #[serde(flatten)]
    pub consensus_rpc: ConsensusCoreRpcConfig,
    /// Core RPC config for check tx
    #[serde(flatten)]
    pub check_tx_rpc: CheckTxCoreRpcConfig,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            consensus_rpc: Default::default(),
            check_tx_rpc: Default::default(),
        }
    }
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
    /// The network type
    #[serde(
        default = "PlatformConfig::default_network",
        deserialize_with = "from_str_to_network_with_aliases"
    )]
    pub network: Network,
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

    /// Execution config
    #[serde(flatten)]
    pub execution: ExecutionConfig,

    /// The default quorum type
    #[serde(flatten)]
    pub validator_set: ValidatorSetConfig,

    /// Chain lock configuration
    #[serde(flatten)]
    pub chain_lock: ChainLockConfig,

    /// Instant lock configuration
    #[serde(flatten)]
    pub instant_lock: InstantLockConfig,

    // todo: this should probably be coming from Tenderdash config. It's a test only param
    /// Approximately how often are blocks produced
    pub block_spacing_ms: u64,

    /// Initial protocol version
    #[serde(default = "PlatformConfig::default_initial_protocol_version")]
    pub initial_protocol_version: ProtocolVersion,

    /// Path to data storage
    pub db_path: PathBuf,

    /// Path to store rejected / invalid items (like transactions).
    /// Used mainly for debuggig.
    ///
    /// If not set, rejected and invalid items will not be stored.
    #[serde(default)]
    pub rejections_path: Option<PathBuf>,

    #[cfg(feature = "testing-config")]
    /// This should be None, except in the case of Testing platform
    #[serde(skip)]
    pub testing_configs: PlatformTestConfig,

    /// Enable tokio console (console feature must be enabled)
    pub tokio_console_enabled: bool,

    // TODO: Use from_str_to_socket_address
    /// Tokio console address to connect to
    #[serde(default = "PlatformConfig::default_tokio_console_address")]
    pub tokio_console_address: String,

    /// Number of seconds to store task information if there is no clients connected
    #[serde(default = "PlatformConfig::default_tokio_console_retention_secs")]
    pub tokio_console_retention_secs: u64,
}

fn from_str_to_network_with_aliases<'de, D>(deserializer: D) -> Result<Network, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let network_name = String::deserialize(deserializer)?;

    match network_name.as_str() {
        "mainnet" => Ok(Network::Dash),
        "local" => Ok(Network::Regtest),
        _ => Network::from_str(network_name.as_str())
            .map_err(|e| serde::de::Error::custom(format!("can't parse network name: {e}"))),
    }
}

/// A config suitable for a quorum configuration
pub trait QuorumLikeConfig: Sized {
    /// Quorum type
    fn quorum_type(&self) -> QuorumType;

    /// Quorum size
    fn quorum_size(&self) -> u16;

    /// Quorum DKG interval
    fn quorum_window(&self) -> u32;

    /// Quorum active signers count
    fn quorum_active_signers(&self) -> u16;

    /// Quorum rotation (dip24) or classic
    fn quorum_rotation(&self) -> bool;
}

/// Chain Lock quorum configuration
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct ValidatorSetConfig {
    /// The quorum type used for verifying chain locks
    #[serde(
        rename = "validator_set_quorum_type",
        serialize_with = "serialize_quorum_type",
        deserialize_with = "deserialize_quorum_type"
    )]
    pub quorum_type: QuorumType,

    /// The quorum size
    #[serde(
        rename = "validator_set_quorum_size",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_size: u16,

    /// The quorum window (DKG interval)
    /// On Mainnet Chain Locks are signed using 400_60: One quorum in every 288 blocks and activeQuorumCount is 4.
    /// On Testnet Chain Locks are signed using 50_60: One quorum in every 24 blocks and activeQuorumCount is 24.
    #[serde(
        rename = "validator_set_quorum_window",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_window: u32,

    /// The number of active signers
    #[serde(
        rename = "validator_set_quorum_active_signers",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_active_signers: u16,

    /// Whether the quorum is rotated DIP24 or classic
    #[serde(
        rename = "validator_set_quorum_rotation",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_rotation: bool,
}

impl Default for ValidatorSetConfig {
    fn default() -> Self {
        // Mainnet
        Self::default_100_67()
    }
}

impl ValidatorSetConfig {
    /// Creates a default config for LLMQ 100 67
    pub fn default_100_67() -> Self {
        Self {
            quorum_type: QuorumType::Llmq100_67,
            quorum_size: 100,
            quorum_window: 24,
            quorum_active_signers: 24,
            quorum_rotation: false,
        }
    }
}

impl QuorumLikeConfig for ValidatorSetConfig {
    fn quorum_type(&self) -> QuorumType {
        self.quorum_type
    }

    fn quorum_size(&self) -> u16 {
        self.quorum_size
    }

    fn quorum_window(&self) -> u32 {
        self.quorum_window
    }

    fn quorum_active_signers(&self) -> u16 {
        self.quorum_active_signers
    }

    fn quorum_rotation(&self) -> bool {
        self.quorum_rotation
    }
}

/// Chain Lock quorum configuration
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct ChainLockConfig {
    /// The quorum type used for verifying chain locks
    #[serde(
        rename = "chain_lock_quorum_type",
        serialize_with = "serialize_quorum_type",
        deserialize_with = "deserialize_quorum_type"
    )]
    pub quorum_type: QuorumType,

    /// The quorum size
    #[serde(
        rename = "chain_lock_quorum_size",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_size: u16,

    /// The quorum window (DKG interval)
    /// On Mainnet Chain Locks are signed using 400_60: One quorum in every 288 blocks and activeQuorumCount is 4.
    /// On Testnet Chain Locks are signed using 50_60: One quorum in every 24 blocks and activeQuorumCount is 24.
    #[serde(
        rename = "chain_lock_quorum_window",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_window: u32,

    /// The number of active signers
    #[serde(
        rename = "chain_lock_quorum_active_signers",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_active_signers: u16,

    /// Whether the quorum is rotated DIP24 or classic
    #[serde(
        rename = "chain_lock_quorum_rotation",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_rotation: bool,
}

impl Default for ChainLockConfig {
    fn default() -> Self {
        // Mainnet
        Self {
            quorum_type: QuorumType::Llmq400_60,
            quorum_size: 400,
            quorum_window: 24 * 12,
            quorum_active_signers: 4,
            quorum_rotation: false,
        }
    }
}

impl QuorumLikeConfig for ChainLockConfig {
    fn quorum_type(&self) -> QuorumType {
        self.quorum_type
    }

    fn quorum_size(&self) -> u16 {
        self.quorum_size
    }

    fn quorum_window(&self) -> u32 {
        self.quorum_window
    }

    fn quorum_active_signers(&self) -> u16 {
        self.quorum_active_signers
    }

    fn quorum_rotation(&self) -> bool {
        self.quorum_rotation
    }
}

impl ChainLockConfig {
    /// Creates a default config for LLMQ 100 67
    pub fn default_100_67() -> Self {
        Self {
            quorum_type: QuorumType::Llmq100_67,
            quorum_size: 100,
            quorum_window: 24,
            quorum_active_signers: 24,
            quorum_rotation: false,
        }
    }
}

/// Chain Lock quorum configuration
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct InstantLockConfig {
    /// The quorum type used for verifying chain locks
    #[serde(
        rename = "instant_lock_quorum_type",
        serialize_with = "serialize_quorum_type",
        deserialize_with = "deserialize_quorum_type"
    )]
    pub quorum_type: QuorumType,

    /// The quorum size
    #[serde(
        rename = "instant_lock_quorum_size",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_size: u16,

    /// The quorum window (DKG interval)
    /// On Mainnet Chain Locks are signed using 400_60: One quorum in every 288 blocks and activeQuorumCount is 4.
    /// On Testnet Chain Locks are signed using 50_60: One quorum in every 24 blocks and activeQuorumCount is 24.
    #[serde(
        rename = "instant_lock_quorum_window",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_window: u32,

    /// The number of active signers
    #[serde(
        rename = "instant_lock_quorum_active_signers",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_active_signers: u16,

    /// Whether the quorum is rotated DIP24 or classic
    #[serde(
        rename = "instant_lock_quorum_rotation",
        deserialize_with = "from_str_or_number"
    )]
    pub quorum_rotation: bool,
}

impl Default for InstantLockConfig {
    fn default() -> Self {
        // Mainnet
        Self {
            quorum_type: QuorumType::Llmq60_75,
            quorum_active_signers: 32,
            quorum_size: 60,
            quorum_window: 24 * 12,
            quorum_rotation: true,
        }
    }
}

impl InstantLockConfig {
    /// Creates a default config for LLMQ 100 67
    pub fn default_100_67() -> Self {
        Self {
            quorum_type: QuorumType::Llmq100_67,
            quorum_size: 100,
            quorum_window: 24,
            quorum_active_signers: 24,
            quorum_rotation: false,
        }
    }
}

impl QuorumLikeConfig for InstantLockConfig {
    fn quorum_type(&self) -> QuorumType {
        self.quorum_type
    }

    fn quorum_size(&self) -> u16 {
        self.quorum_size
    }

    fn quorum_window(&self) -> u32 {
        self.quorum_window
    }

    fn quorum_active_signers(&self) -> u16 {
        self.quorum_active_signers
    }

    fn quorum_rotation(&self) -> bool {
        self.quorum_rotation
    }
}

fn serialize_quorum_type<S>(quorum_type: &QuorumType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(quorum_type.to_string().as_str())
}

fn deserialize_quorum_type<'de, D>(deserializer: D) -> Result<QuorumType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let quorum_type_name = String::deserialize(deserializer)?;

    let quorum_type = if let Ok(t) = quorum_type_name.trim().parse::<u32>() {
        QuorumType::from(t)
    } else {
        QuorumType::from(quorum_type_name.as_str())
    };

    if quorum_type == QuorumType::UNKNOWN {
        return Err(serde::de::Error::custom(format!(
            "unsupported QUORUM_TYPE: {}",
            quorum_type_name
        )));
    };

    Ok(quorum_type)
}

impl ExecutionConfig {
    fn default_verify_sum_trees() -> bool {
        true
    }

    fn default_use_document_triggers() -> bool {
        true
    }

    fn default_epoch_time_length_s() -> u64 {
        788400
    }
}

impl PlatformConfig {
    fn default_initial_protocol_version() -> ProtocolVersion {
        INITIAL_PROTOCOL_VERSION
    }

    fn default_network() -> Network {
        Network::Regtest //todo: Do not leave this as regtest
                         // TODO: Yes, must be mainnet
    }

    fn default_tokio_console_address() -> String {
        String::from("127.0.0.1:6669")
    }

    fn default_tokio_console_retention_secs() -> u64 {
        60 * 3
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
            epoch_time_length_s: ExecutionConfig::default_epoch_time_length_s(),
        }
    }
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self::default_mainnet()
    }
}

#[allow(missing_docs)]
impl PlatformConfig {
    pub fn default_local() -> Self {
        Self {
            network: Network::Regtest,
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::LlmqTestPlatform,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 2,
                quorum_rotation: false,
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::LlmqTest,
                quorum_active_signers: 2,
                quorum_size: 3,
                quorum_window: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::LlmqTest,
                quorum_active_signers: 2,
                quorum_size: 3,
                quorum_window: 24,
                quorum_rotation: false,
            },
            block_spacing_ms: 5000,
            drive: Default::default(),
            abci: Default::default(),
            core: Default::default(),
            execution: Default::default(),
            db_path: PathBuf::from("/var/lib/dash-platform/data"),
            rejections_path: Some(PathBuf::from("/var/log/dash/rejected")),
            #[cfg(feature = "testing-config")]
            testing_configs: PlatformTestConfig::default(),
            tokio_console_enabled: false,
            tokio_console_address: PlatformConfig::default_tokio_console_address(),
            tokio_console_retention_secs: PlatformConfig::default_tokio_console_retention_secs(),
            initial_protocol_version: Self::default_initial_protocol_version(),
            prometheus_bind_address: None,
            grpc_bind_address: "127.0.0.1:26670".to_string(),
        }
    }

    pub fn default_testnet() -> Self {
        Self {
            network: Network::Testnet,
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq25_67,
                quorum_size: 25,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq50_60,
                quorum_active_signers: 24,
                quorum_size: 50,
                quorum_window: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq60_75,
                quorum_active_signers: 32,
                quorum_size: 60,
                quorum_window: 24 * 12,
                quorum_rotation: true,
            },
            block_spacing_ms: 5000,
            drive: Default::default(),
            abci: Default::default(),
            core: Default::default(),
            execution: Default::default(),
            db_path: PathBuf::from("/var/lib/dash-platform/data"),
            rejections_path: Some(PathBuf::from("/var/log/dash/rejected")),
            #[cfg(feature = "testing-config")]
            testing_configs: PlatformTestConfig::default(),
            initial_protocol_version: Self::default_initial_protocol_version(),
            prometheus_bind_address: None,
            grpc_bind_address: "127.0.0.1:26670".to_string(),
            tokio_console_enabled: false,
            tokio_console_address: PlatformConfig::default_tokio_console_address(),
            tokio_console_retention_secs: PlatformConfig::default_tokio_console_retention_secs(),
        }
    }

    pub fn default_mainnet() -> Self {
        Self {
            network: Network::Dash,
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 100,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq400_60,
                quorum_active_signers: 4,
                quorum_size: 400,
                quorum_window: 24 * 12,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq60_75,
                quorum_active_signers: 32,
                quorum_size: 60,
                quorum_window: 24 * 12,
                quorum_rotation: true,
            },
            block_spacing_ms: 5000,
            drive: Default::default(),
            abci: Default::default(),
            core: Default::default(),
            execution: Default::default(),
            db_path: PathBuf::from("/var/lib/dash-platform/data"),
            rejections_path: Some(PathBuf::from("/var/log/dash/rejected")),
            #[cfg(feature = "testing-config")]
            testing_configs: PlatformTestConfig::default(),
            initial_protocol_version: Self::default_initial_protocol_version(),
            prometheus_bind_address: None,
            grpc_bind_address: "127.0.0.1:26670".to_string(),
            tokio_console_enabled: false,
            tokio_console_address: PlatformConfig::default_tokio_console_address(),
            tokio_console_retention_secs: PlatformConfig::default_tokio_console_retention_secs(),
        }
    }
}

#[cfg(feature = "testing-config")]
/// Configs that should only happen during testing
#[derive(Clone, Debug)]
pub struct PlatformTestConfig {
    /// Block signing
    pub block_signing: bool,
    /// Storing of platform state
    pub store_platform_state: bool,
    /// Block signature verification
    pub block_commit_signature_verification: bool,
    /// Disable instant lock signature verification
    pub disable_instant_lock_signature_verification: bool,
}

#[cfg(feature = "testing-config")]
impl PlatformTestConfig {
    /// Much faster config for tests
    pub fn default_minimal_verifications() -> Self {
        Self {
            block_signing: false,
            store_platform_state: false,
            block_commit_signature_verification: false,
            disable_instant_lock_signature_verification: true,
        }
    }
}

#[cfg(feature = "testing-config")]
impl Default for PlatformTestConfig {
    fn default() -> Self {
        Self {
            block_signing: true,
            store_platform_state: true,
            block_commit_signature_verification: true,
            disable_instant_lock_signature_verification: false,
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
        assert_eq!("/tmp/rejected", env::var("REJECTIONS_PATH").unwrap());

        let config = super::PlatformConfig::from_env().expect("expected config from env");
        assert!(config.execution.verify_sum_trees);
        assert_ne!(config.validator_set.quorum_type, QuorumType::UNKNOWN);
        for id in vectors {
            matches!(config.abci.log[id.0].destination, LogDestination::Bytes);
        }
    }
}
