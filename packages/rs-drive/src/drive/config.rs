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

use serde::{Deserialize, Serialize};

use crate::drive::config::DriveEncoding::DriveCbor;

/// Boolean if GroveDB batching consistency verification is enabled by default
pub const DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED: bool = false;
/// Boolean if GroveDB has_raw in enabled by default
pub const DEFAULT_GROVE_HAS_RAW_ENABLED: bool = true;
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
    pub batching_consistency_verification: bool,

    /// Boolean if has_raw is enabled
    pub has_raw_enabled: bool,

    /// Default genesis time
    pub default_genesis_time: Option<u64>,

    /// Encoding
    pub encoding: DriveEncoding,

    /// Maximum number of contracts in global cache
    pub data_contracts_global_cache_size: u64,

    /// Maximum number of contracts in block candidate cache
    pub data_contracts_block_cache_size: u64,
}

impl Default for DriveConfig {
    fn default() -> Self {
        DriveConfig {
            batching_consistency_verification:
                DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED,
            has_raw_enabled: DEFAULT_GROVE_HAS_RAW_ENABLED,
            default_genesis_time: None,
            encoding: DriveCbor,
            data_contracts_global_cache_size: DEFAULT_DATA_CONTRACTS_CACHE_SIZE,
            data_contracts_block_cache_size: DEFAULT_DATA_CONTRACTS_CACHE_SIZE,
        }
    }
}
