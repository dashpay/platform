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
use std::path::Path;
use std::sync::RwLock;

use crate::rpc::core::MockCoreRPCLike;
use dashcore_rpc::dashcore::hashes::hex::FromHex;

use dashcore_rpc::dashcore::BlockHash;

use dpp::serialization_traits::PlatformDeserializable;

use drive::error::Error::GroveDB;
use serde_json::json;

mod state_repository;

/// Platform
pub struct Platform<C> {
    /// Drive
    pub drive: Drive,
    /// State
    pub state: RwLock<PlatformState>,
    /// Configuration
    pub config: PlatformConfig,
    /// Block execution context
    pub block_execution_context: RwLock<Option<BlockExecutionContext>>,
    /// Core RPC Client
    pub core_rpc: C,
}

/// Platform Ref
pub struct PlatformRef<'a, C> {
    /// Drive
    pub drive: &'a Drive,
    /// State
    pub state: &'a PlatformState,
    /// Configuration
    pub config: &'a PlatformConfig,
    /// Core RPC Client
    pub core_rpc: &'a C,
}

/// Platform State Ref
pub struct PlatformStateRef<'a> {
    /// Drive
    pub drive: &'a Drive,
    /// State
    pub state: &'a PlatformState,
    /// Configuration
    pub config: &'a PlatformConfig,
}

impl<'a, C> From<&PlatformRef<'a, C>> for PlatformStateRef<'a> {
    fn from(value: &PlatformRef<'a, C>) -> Self {
        let PlatformRef {
            drive,
            state,
            config,
            ..
        } = value;

        PlatformStateRef {
            drive,
            state,
            config,
        }
    }
}

impl<C> std::fmt::Debug for Platform<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Platform").finish()
    }
}

impl Platform<DefaultCoreRPC> {
    /// Open Platform with Drive and block execution context and default core rpc.
    pub fn open<P: AsRef<Path>>(
        path: P,
        config: Option<PlatformConfig>,
    ) -> Result<Platform<DefaultCoreRPC>, Error> {
        let config = config.unwrap_or_default();
        let core_rpc = DefaultCoreRPC::open(
            config.core.rpc.url().as_str(),
            config.core.rpc.username.clone(),
            config.core.rpc.password.clone(),
        )
        .map_err(|_e| {
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "Could not setup Dash Core RPC client",
            ))
        })?;
        Self::open_with_client(path, Some(config), core_rpc)
    }
}

impl Platform<MockCoreRPCLike> {
    /// Open Platform with Drive and block execution context and mock core rpc.
    pub fn open<P: AsRef<Path>>(
        path: P,
        config: Option<PlatformConfig>,
    ) -> Result<Platform<MockCoreRPCLike>, Error> {
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
        Self::open_with_client(path, config, core_rpc_mock)
    }

    /// Recreate the state from the backing store
    pub fn recreate_state(&self) -> Result<bool, Error> {
        let Some(serialized_platform_state) = self.drive
            .grove
            .get_aux(b"saved_state", None)
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))? else {
            return Ok(false);
        };

        let recreated_state = PlatformState::deserialize(&serialized_platform_state)?;

        let mut state_cache = self.state.write().unwrap();
        *state_cache = recreated_state;
        Ok(true)
    }
}

impl<C> Platform<C> {
    /// Open Platform with Drive and block execution context.
    pub fn open_with_client<P: AsRef<Path>>(
        path: P,
        config: Option<PlatformConfig>,
        core_rpc: C,
    ) -> Result<Platform<C>, Error>
    where
        C: CoreRPCLike,
    {
        let config = config.unwrap_or_default();
        let drive = Drive::open(path, Some(config.drive.clone())).map_err(Error::Drive)?;

        // TODO: factor out key so we don't duplicate
        let maybe_serialized_platform_state = drive
            .grove
            .get_aux(b"saved_state", None)
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        if let Some(serialized_platform_state) = maybe_serialized_platform_state {
            Platform::open_with_client_saved_state::<P>(
                drive,
                core_rpc,
                config,
                serialized_platform_state,
            )
        } else {
            Platform::open_with_client_no_saved_state::<P>(
                drive,
                core_rpc,
                config,
                PROTOCOL_VERSION,
                PROTOCOL_VERSION,
            )
        }
    }

    /// Open Platform with Drive and block execution context from saved state.
    pub fn open_with_client_saved_state<P: AsRef<Path>>(
        drive: Drive,
        core_rpc: C,
        config: PlatformConfig,
        serialized_platform_state: Vec<u8>,
    ) -> Result<Platform<C>, Error>
    where
        C: CoreRPCLike,
    {
        let platform_state = PlatformState::deserialize(&serialized_platform_state)?;

        let platform: Platform<C> = Platform {
            drive,
            state: RwLock::new(platform_state),
            config,
            block_execution_context: RwLock::new(None),
            core_rpc,
        };

        Ok(platform)
    }

    /// Open Platform with Drive and block execution context without saved state.
    pub fn open_with_client_no_saved_state<P: AsRef<Path>>(
        drive: Drive,
        core_rpc: C,
        config: PlatformConfig,
        current_protocol_version_in_consensus: u32,
        next_epoch_protocol_version: u32,
    ) -> Result<Platform<C>, Error>
    where
        C: CoreRPCLike,
    {
        let state = PlatformState::default_with_protocol_versions(
            current_protocol_version_in_consensus,
            next_epoch_protocol_version,
        );

        Ok(Platform {
            drive,
            state: RwLock::new(state),
            config,
            block_execution_context: RwLock::new(None),
            core_rpc,
        })
    }
}
