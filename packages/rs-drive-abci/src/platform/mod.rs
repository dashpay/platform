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

use crate::error::serialization::SerializationError;
use crate::error::Error::Serialization;
use crate::rpc::core::MockCoreRPCLike;
use dashcore::hashes::hex::FromHex;
use dashcore::hashes::Hash;
use dashcore::{BlockHash, QuorumHash};
use drive::drive::block_info::BlockInfo;
use drive::error::drive::DriveError;
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
        let drive = Drive::open(path, config.drive.clone()).map_err(Error::Drive)?;

        let current_protocol_version_in_consensus = drive
            .fetch_current_protocol_version(None)
            .map_err(Error::Drive)?
            .unwrap_or(PROTOCOL_VERSION);
        let next_epoch_protocol_version = drive
            .fetch_next_protocol_version(None)
            .map_err(Error::Drive)?
            .unwrap_or(PROTOCOL_VERSION);

        // TODO: factor out key so we don't duplicate
        let maybe_serialized_block_info = drive
            .grove
            .get_aux(b"saved_state", None)
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        if let Some(serialized_block_info) = maybe_serialized_block_info {
            Platform::open_with_client_saved_state::<P>(
                drive,
                core_rpc,
                config,
                serialized_block_info,
                current_protocol_version_in_consensus,
                next_epoch_protocol_version,
            )
        } else {
            Platform::open_with_client_no_saved_state::<P>(
                drive,
                core_rpc,
                config,
                current_protocol_version_in_consensus,
                next_epoch_protocol_version,
            )
        }
    }

    /// Open Platform with Drive and block execution context from saved state.
    pub fn open_with_client_saved_state<P: AsRef<Path>>(
        drive: Drive,
        core_rpc: C,
        config: PlatformConfig,
        serialized_block_info: Vec<u8>,
        current_protocol_version_in_consensus: u32,
        next_epoch_protocol_version: u32,
    ) -> Result<Platform<C>, Error>
    where
        C: CoreRPCLike,
    {
        let block_info: BlockInfo = bincode::deserialize(&serialized_block_info).map_err(|e| {
            Serialization(SerializationError::CorruptedDeserialization(
                "failed to deserialize saved state".to_string(),
            ))
        })?;

        let maybe_quorum_hash = drive
            .grove
            .get_aux(b"saved_quorum_hash", None)
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        // TODO: remove unwrap
        let current_validator_set_quorum_hash =
            QuorumHash::from_slice(&maybe_quorum_hash.unwrap()).unwrap();

        let state = PlatformState {
            last_committed_block_info: Some(block_info),
            current_protocol_version_in_consensus,
            next_epoch_protocol_version,
            quorums_extended_info: Default::default(),
            current_validator_set_quorum_hash,
            validator_sets: Default::default(),
            full_masternode_list: Default::default(),
            hpmn_masternode_list: Default::default(),
        };

        let core_height = state.core_height();
        let block_info = state
            .last_committed_block_info
            .clone()
            .unwrap_or(BlockInfo::genesis());

        let platform: Platform<C> = Platform {
            drive,
            state: RwLock::new(state),
            config,
            block_execution_context: RwLock::new(None),
            core_rpc,
        };

        let transaction = platform.drive.grove.start_transaction();
        let mut state_cache = platform.state.write().unwrap();
        platform.update_quorum_info(&mut state_cache, core_height)?;
        platform.update_masternode_list(
            &mut state_cache,
            core_height,
            &block_info,
            &transaction,
        )?;
        drop(state_cache);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        return Ok(platform);
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
        let state = PlatformState {
            last_committed_block_info: None,
            current_protocol_version_in_consensus,
            next_epoch_protocol_version,
            quorums_extended_info: Default::default(),
            current_validator_set_quorum_hash: Default::default(),
            validator_sets: Default::default(),
            full_masternode_list: Default::default(),
            hpmn_masternode_list: Default::default(),
        };

        Ok(Platform {
            drive,
            state: RwLock::new(state),
            config,
            block_execution_context: RwLock::new(None),
            core_rpc,
        })
    }
}
