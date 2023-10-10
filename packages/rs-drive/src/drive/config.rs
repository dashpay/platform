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
//

//! Drive Configuration File
//!

use dpp::fee::epoch::DEFAULT_EPOCHS_PER_ERA;
use serde::{Deserialize, Serialize};

/// Boolean if GroveDB batching consistency verification is enabled by default
pub const DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED: bool = false;
/// Boolean if GroveDB has_raw in enabled by default
pub const DEFAULT_GROVE_HAS_RAW_ENABLED: bool = true;
/// The default default query limit
pub const DEFAULT_DEFAULT_QUERY_LIMIT: u16 = 100;
/// The default max query limit
pub const DEFAULT_MAX_QUERY_LIMIT: u16 = 100;
/// Default maximum number of contracts in cache
pub const DEFAULT_DATA_CONTRACTS_CACHE_SIZE: u64 = 500;

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Encoding for Drive
pub enum DriveEncoding {
    /// Drive CBOR
    DriveCbor,
    /// Drive protobuf
    DriveProtobuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Drive configuration struct
pub struct DriveConfig {
    /// Boolean if batching consistency verification is enabled
    #[serde(default = "default_batching_consistency_verification")]
    pub batching_consistency_verification: bool,

    /// Boolean if has_raw is enabled
    #[serde(default = "default_has_raw_enabled")]
    pub has_raw_enabled: bool,

    /// The default returned count if no limit is set
    #[serde(
        default = "default_default_query_limit",
        deserialize_with = "from_str_or_number"
    )]
    pub default_query_limit: u16,

    /// The default returned count if no limit is set
    #[serde(
        default = "default_epochs_per_era",
        deserialize_with = "from_str_or_number"
    )]
    pub epochs_per_era: u16,

    /// The limit for user defined queries
    #[serde(
        default = "default_max_query_limit",
        deserialize_with = "from_str_or_number"
    )]
    pub max_query_limit: u16,

    /// Default genesis time
    #[serde(default)]
    pub default_genesis_time: Option<u64>,

    /// Maximum number of contracts in global cache
    #[serde(
        default = "default_data_contracts_cache_size",
        deserialize_with = "from_str_or_number"
    )]
    pub data_contracts_global_cache_size: u64,

    /// Maximum number of contracts in block candidate cache
    #[serde(
        default = "default_data_contracts_cache_size",
        deserialize_with = "from_str_or_number"
    )]
    pub data_contracts_block_cache_size: u64,
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

// Define default functions for serde
fn default_batching_consistency_verification() -> bool {
    DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED
}

fn default_has_raw_enabled() -> bool {
    DEFAULT_GROVE_HAS_RAW_ENABLED
}

fn default_default_query_limit() -> u16 {
    DEFAULT_DEFAULT_QUERY_LIMIT
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

impl Default for DriveConfig {
    fn default() -> Self {
        DriveConfig {
            batching_consistency_verification:
                DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED,
            has_raw_enabled: DEFAULT_GROVE_HAS_RAW_ENABLED,
            default_query_limit: DEFAULT_DEFAULT_QUERY_LIMIT,
            epochs_per_era: DEFAULT_EPOCHS_PER_ERA,
            max_query_limit: DEFAULT_MAX_QUERY_LIMIT,
            default_genesis_time: None,
            data_contracts_global_cache_size: DEFAULT_DATA_CONTRACTS_CACHE_SIZE,
            data_contracts_block_cache_size: DEFAULT_DATA_CONTRACTS_CACHE_SIZE,
        }
    }
}
