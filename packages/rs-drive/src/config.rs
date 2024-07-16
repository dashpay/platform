//! Drive Configuration File
//!

use dpp::fee::epoch::DEFAULT_EPOCHS_PER_ERA;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Boolean if GroveDB batching consistency verification is enabled by default
pub const DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED: bool = false;
/// Boolean if GroveDB has_raw in enabled by default
pub const DEFAULT_GROVE_HAS_RAW_ENABLED: bool = true;
/// The default default query limit
pub const DEFAULT_QUERY_LIMIT: u16 = 100;
/// The default max query limit
pub const DEFAULT_MAX_QUERY_LIMIT: u16 = 100;
/// Default maximum number of contracts in cache
pub const DEFAULT_DATA_CONTRACTS_CACHE_SIZE: u64 = 500;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// Drive configuration struct
pub struct DriveConfig {
    /// Boolean if batching consistency verification is enabled
    #[cfg_attr(
        feature = "serde",
        serde(default = "default_batching_consistency_verification")
    )]
    pub batching_consistency_verification: bool,

    /// Boolean if has_raw is enabled
    #[cfg_attr(feature = "serde", serde(default = "default_has_raw_enabled"))]
    pub has_raw_enabled: bool,

    /// The default returned count if no limit is set
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_default_query_limit",
            deserialize_with = "from_str_or_number"
        )
    )]
    pub default_query_limit: u16,

    /// The default returned count if no limit is set
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_epochs_per_era",
            deserialize_with = "from_str_or_number"
        )
    )]
    pub epochs_per_era: u16,

    /// The limit for user defined queries
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_max_query_limit",
            deserialize_with = "from_str_or_number"
        )
    )]
    pub max_query_limit: u16,

    /// Default genesis time
    #[cfg_attr(feature = "serde", serde(default))]
    pub default_genesis_time: Option<u64>,

    /// Maximum number of contracts in global cache
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_data_contracts_cache_size",
            deserialize_with = "from_str_or_number"
        )
    )]
    pub data_contracts_global_cache_size: u64,

    /// Maximum number of contracts in block candidate cache
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_data_contracts_cache_size",
            deserialize_with = "from_str_or_number"
        )
    )]
    pub data_contracts_block_cache_size: u64,

    /// GroveDB visualizer address
    #[cfg(feature = "grovedbg")]
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_grovedb_visualizer_address",
            deserialize_with = "from_str_to_socket_address"
        )
    )]
    pub grovedb_visualizer_address: std::net::SocketAddr,

    /// Enable GroveDB visualizer
    #[cfg(feature = "grovedbg")]
    #[cfg_attr(
        feature = "serde",
        serde(default, deserialize_with = "from_str_to_bool")
    )]
    pub grovedb_visualizer_enabled: bool,
}

// TODO: some weird envy behavior requries this to exist
#[cfg(all(feature = "serde", feature = "grovedbg"))]
fn from_str_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

#[cfg(feature = "serde")]
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

#[cfg(all(feature = "serde", feature = "grovedbg"))]
fn from_str_to_socket_address<'de, D>(deserializer: D) -> Result<std::net::SocketAddr, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

// Define default functions for serde
fn default_batching_consistency_verification() -> bool {
    DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED
}

fn default_has_raw_enabled() -> bool {
    DEFAULT_GROVE_HAS_RAW_ENABLED
}

fn default_default_query_limit() -> u16 {
    DEFAULT_QUERY_LIMIT
}

fn default_epochs_per_era() -> u16 {
    DEFAULT_EPOCHS_PER_ERA
}

fn default_max_query_limit() -> u16 {
    DEFAULT_MAX_QUERY_LIMIT
}

fn default_data_contracts_cache_size() -> u64 {
    DEFAULT_DATA_CONTRACTS_CACHE_SIZE
}

fn default_grovedb_visualizer_address() -> std::net::SocketAddr {
    "127.0.0.1:8083".parse().unwrap()
}

impl Default for DriveConfig {
    fn default() -> Self {
        DriveConfig {
            batching_consistency_verification:
                DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED,
            has_raw_enabled: DEFAULT_GROVE_HAS_RAW_ENABLED,
            default_query_limit: DEFAULT_QUERY_LIMIT,
            epochs_per_era: DEFAULT_EPOCHS_PER_ERA,
            max_query_limit: DEFAULT_MAX_QUERY_LIMIT,
            default_genesis_time: None,
            data_contracts_global_cache_size: DEFAULT_DATA_CONTRACTS_CACHE_SIZE,
            data_contracts_block_cache_size: DEFAULT_DATA_CONTRACTS_CACHE_SIZE,
            #[cfg(feature = "grovedbg")]
            grovedb_visualizer_address: default_grovedb_visualizer_address(),
            #[cfg(feature = "grovedbg")]
            grovedb_visualizer_enabled: false,
        }
    }
}
