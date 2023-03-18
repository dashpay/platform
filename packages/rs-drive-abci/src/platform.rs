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

//! Platform Init
//!

use crate::block::BlockExecutionContext;
use crate::config::PlatformConfig;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::{CoreRPCLike, DefaultCoreRPC};
use crate::state::PlatformState;
use drive::drive::Drive;

use drive::drive::defaults::PROTOCOL_VERSION;
use std::cell::RefCell;
use std::path::Path;

#[cfg(feature = "fixtures-and-mocks")]
use crate::rpc::core::MockCoreRPCLike;
#[cfg(feature = "fixtures-and-mocks")]
use dashcore::hashes::hex::FromHex;
#[cfg(feature = "fixtures-and-mocks")]
use dashcore::BlockHash;
#[cfg(feature = "fixtures-and-mocks")]
use serde_json::json;

/// Platform
pub struct Platform {
    /// Drive
    pub drive: Drive,
    /// State
    pub state: RefCell<PlatformState>,
    /// Configuration
    pub config: PlatformConfig,
    /// Block execution context
    pub block_execution_context: RefCell<Option<BlockExecutionContext>>,
    /// Core RPC Client
    pub core_rpc: Box<dyn CoreRPCLike>,
}

impl std::fmt::Debug for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Platform").finish()
    }
}

impl Platform {
    /// Open Platform with Drive and block execution context.
    pub fn open<P: AsRef<Path>>(path: P, config: Option<PlatformConfig>) -> Result<Self, Error> {
        let config = config.unwrap_or_default();
        let drive = Drive::open(path, config.drive.clone()).map_err(Error::Drive)?;

        let core_rpc: Box<dyn CoreRPCLike> = Box::new(
            DefaultCoreRPC::open(
                config.core.rpc.url.as_str(),
                config.core.rpc.username.clone(),
                config.core.rpc.password.clone(),
            )
            .map_err(|_e| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "Could not setup Dash Core RPC client",
                ))
            })?,
        );

        let current_protocol_version_in_consensus = drive
            .fetch_current_protocol_version(None)
            .map_err(Error::Drive)?
            .unwrap_or(PROTOCOL_VERSION);
        let next_epoch_protocol_version = drive
            .fetch_next_protocol_version(None)
            .map_err(Error::Drive)?
            .unwrap_or(PROTOCOL_VERSION);

        let state = PlatformState {
            last_block_info: None,
            current_protocol_version_in_consensus,
            next_epoch_protocol_version,
        };

        Ok(Platform {
            drive,
            state: RefCell::new(state),
            config,
            block_execution_context: RefCell::new(None),
            core_rpc,
        })
    }

    /// Helper function to be able
    /// to quickly mock core rpc for tests
    #[cfg(feature = "fixtures-and-mocks")]
    pub fn mock_core_rpc_client(&mut self) {
        let mut core_rpc_mock = MockCoreRPCLike::new();

        core_rpc_mock.expect_get_block_hash().returning(|_| {
            Ok(BlockHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap())
        });

        core_rpc_mock.expect_get_block_json().returning(|_| {
            Ok(json!({
                "tx": [],
            }))
        });

        self.core_rpc = Box::new(core_rpc_mock);
    }
}
