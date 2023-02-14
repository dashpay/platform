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

use drive::drive::config::DriveConfig;

/// Configuration for Dash Core RPC client
#[derive(Clone, Debug)]
pub struct CoreRpcConfig {
    /// Core RPC client url
    pub url: String,

    /// Core RPC client username
    pub username: String,

    /// Core RPC client password
    pub password: String,
}

/// Configuration for Dash Core related things
#[derive(Clone, Debug)]
pub struct CoreConfig {
    /// Core RPC config
    pub rpc: CoreRpcConfig,
}

/// Platform configuration
#[derive(Clone, Debug)]
pub struct PlatformConfig {
    /// Drive configuration
    pub drive: Option<DriveConfig>,

    /// Dash Core config
    pub core: CoreConfig,

    /// Should we verify sum trees? Useful to set as no for tests
    pub verify_sum_trees: bool,

    /// The default quorum size
    pub quorum_size: u16,

    /// How often should quorums change?
    pub validator_set_quorum_rotation_block_count: u64,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 15,
            drive: Default::default(),
            core: CoreConfig {
                rpc: CoreRpcConfig {
                    url: "127.0.0.1".to_owned(),
                    username: "".to_owned(),
                    password: "".to_owned(),
                },
            },
        }
    }
}
