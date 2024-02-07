use crate::config::PlatformConfig;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::{CoreRPCLike, DefaultCoreRPC};
use drive::drive::Drive;
use std::fmt::{Debug, Formatter};

#[cfg(any(feature = "mocks", test))]
use crate::rpc::core::MockCoreRPCLike;
use drive::drive::defaults::INITIAL_PROTOCOL_VERSION;
use std::path::Path;
use std::str::FromStr;
use std::sync::RwLock;

use dashcore_rpc::dashcore::BlockHash;

use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use serde_json::json;

/// Platform is not versioned as it holds the main logic, we could not switch from one structure
/// configuration of the Platform struct to another without a software upgrade

// @append_only
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

// @append_only
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
    /// Block info
    pub block_info: &'a BlockInfo,
}

// @append_only
/// Platform State Ref
pub struct PlatformStateRef<'a> {
    /// Drive
    pub drive: &'a Drive,
    /// State
    pub state: &'a PlatformState,
    /// Configuration
    pub config: &'a PlatformConfig,
}

impl<'a> Debug for PlatformStateRef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("platform_state_ref")
            .field("state", self.state)
            .field("config", self.config)
            .finish()
    }
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

impl<C> Debug for Platform<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Platform").finish()
    }
}

impl Platform<DefaultCoreRPC> {
    /// Open Platform with Drive and block execution context and default core rpc.
    pub fn open<P: AsRef<Path>>(
        path: P,
        config: Option<PlatformConfig>,
    ) -> Result<Platform<DefaultCoreRPC>, Error> {
        let config = config.unwrap_or(PlatformConfig::default_testnet());
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

#[cfg(any(feature = "mocks", test))]
impl Platform<MockCoreRPCLike> {
    /// Open Platform with Drive and block execution context and mock core rpc.
    pub fn open<P: AsRef<Path>>(
        path: P,
        config: Option<PlatformConfig>,
    ) -> Result<Platform<MockCoreRPCLike>, Error> {
        let mut core_rpc_mock = MockCoreRPCLike::new();

        core_rpc_mock.expect_get_block_hash().returning(|_| {
            Ok(BlockHash::from_str(
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

    /// Fetch and reload the state from the backing store
    pub fn reload_state_from_storage(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let Some(persisted_state) =
            Platform::<MockCoreRPCLike>::fetch_platform_state(&self.drive, None, platform_version)?
        else {
            return Ok(false);
        };

        PlatformVersion::set_current(PlatformVersion::get(
            persisted_state.current_protocol_version_in_consensus(),
        )?);

        let mut state_cache = self.state.write().unwrap();
        *state_cache = persisted_state;

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
        let config = config.unwrap_or(PlatformConfig::default_testnet());

        let (drive, current_protocol_version) =
            Drive::open(path, Some(config.drive.clone())).map_err(Error::Drive)?;

        if let Some(protocol_version) = current_protocol_version {
            let platform_version = PlatformVersion::get(protocol_version)?;

            let Some(execution_state) =
                Platform::<C>::fetch_platform_state(&drive, None, platform_version)?
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCachedState(
                    "execution state should be stored as well as protocol version",
                )));
            };

            return Platform::open_with_client_saved_state::<P>(
                drive,
                core_rpc,
                config,
                execution_state,
            );
        }

        Platform::open_with_client_no_saved_state::<P>(
            drive,
            core_rpc,
            config,
            INITIAL_PROTOCOL_VERSION,
            INITIAL_PROTOCOL_VERSION,
        )
    }

    /// Open Platform with Drive and block execution context from saved state.
    pub fn open_with_client_saved_state<P: AsRef<Path>>(
        drive: Drive,
        core_rpc: C,
        config: PlatformConfig,
        platform_state: PlatformState,
    ) -> Result<Platform<C>, Error>
    where
        C: CoreRPCLike,
    {
        PlatformVersion::set_current(PlatformVersion::get(
            platform_state.current_protocol_version_in_consensus(),
        )?);

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
        let platform_state = PlatformState::default_with_protocol_versions(
            current_protocol_version_in_consensus,
            next_epoch_protocol_version,
        );

        PlatformVersion::set_current(PlatformVersion::get(current_protocol_version_in_consensus)?);

        Ok(Platform {
            drive,
            state: RwLock::new(platform_state),
            config,
            block_execution_context: RwLock::new(None),
            core_rpc,
        })
    }
}

impl<C> Drop for Platform<C> {
    fn drop(&mut self) {
        tracing::trace!("platform is shutting down");

        if let Err(error) = self.drive.grove.flush() {
            tracing::error!(?error, "grovedb flush failed");
        }
        tracing::debug!("platform shutdown complete");
    }
}
