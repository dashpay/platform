#[cfg(any(feature = "mocks", test))]
mod mock;

use crate::config::PlatformConfig;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::{CoreRPCLike, DefaultCoreRPC};
use drive::drive::Drive;
use std::fmt::{Debug, Formatter};

use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use arc_swap::ArcSwap;
use dpp::prelude::BlockHeight;
use dpp::serialization::PlatformDeserializableFromVersionedStructure;
use dpp::version::ProtocolVersion;
use dpp::version::INITIAL_PROTOCOL_VERSION;
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

// @append_only
/// Platform is not versioned as it holds the main logic, we could not switch from one structure
/// configuration of the Platform struct to another without a software upgrade
pub struct Platform<C> {
    /// Drive
    pub drive: Drive,
    /// State
    // We use ArcSwap that provide very fast and consistent reads
    // and atomic write (swap). This is important as we want read state
    // for query and check tx and we don't want to block affect the
    // state update on finalize block, and vise versa.
    pub state: ArcSwap<PlatformState>,
    /// Platform states corresponding to each checkpoint, keyed by block height.
    /// This allows queries against checkpoints to return the correct platform state.
    pub checkpoint_platform_states: ArcSwap<BTreeMap<BlockHeight, Arc<PlatformState>>>,
    /// block height guard
    pub committed_block_height_guard: AtomicU64,
    /// Configuration
    pub config: PlatformConfig,
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

impl Debug for PlatformStateRef<'_> {
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
            config.core.consensus_rpc.url().as_str(),
            config.core.consensus_rpc.username.clone(),
            config.core.consensus_rpc.password.clone(),
        )
        .map_err(|_e| {
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "Could not setup Dash Core RPC client",
            ))
        })?;
        Self::open_with_client(path, Some(config), core_rpc, None)
    }
}

impl<C> Platform<C> {
    /// Open Platform with Drive and block execution context.
    pub fn open_with_client<P: AsRef<Path>>(
        path: P,
        config: Option<PlatformConfig>,
        core_rpc: C,
        initial_protocol_version: Option<ProtocolVersion>,
    ) -> Result<Platform<C>, Error>
    where
        C: CoreRPCLike,
    {
        let config = match config {
            Some(config) => config,
            None => {
                // When using default config, set db_path to the provided path
                let mut config = PlatformConfig::default_testnet();
                config.db_path = path.as_ref().to_path_buf();
                config
            }
        };

        let (drive, current_platform_version) =
            Drive::open(&config.db_path, Some(config.drive.clone())).map_err(Error::Drive)?;

        if let Some(initial_protocol_version) = initial_protocol_version {
            if initial_protocol_version > 1 {
                drive
                    .cache
                    .system_data_contracts
                    .reload_system_contracts(PlatformVersion::get(initial_protocol_version)?)?;
            }
        }

        if let Some(platform_version) = current_platform_version {
            let Some(execution_state) =
                Platform::<C>::fetch_platform_state(&drive, None, platform_version)?
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCachedState(
                    "execution state should be stored as well as protocol version".to_string(),
                )));
            };
            if platform_version.protocol_version > 1 {
                drive
                    .cache
                    .system_data_contracts
                    .reload_system_contracts(platform_version)?;
            }

            // Load checkpoint platform states from disk
            let mut checkpoint_platform_states = BTreeMap::new();
            let checkpoints = drive.checkpoints.load();
            for (&block_height, _checkpoint_info) in checkpoints.iter() {
                let checkpoint_state_path = config
                    .db_path
                    .join("checkpoints")
                    .join(block_height.to_string())
                    .join("platform_state.bin");

                if checkpoint_state_path.exists() {
                    match std::fs::read(&checkpoint_state_path) {
                        Ok(state_bytes) => {
                            match PlatformState::versioned_deserialize(
                                &state_bytes,
                                platform_version,
                            ) {
                                Ok(state) => {
                                    checkpoint_platform_states
                                        .insert(block_height, Arc::new(state));
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to deserialize checkpoint platform state at height {}: {:?}",
                                        block_height,
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to read checkpoint platform state file at {:?}: {:?}",
                                checkpoint_state_path,
                                e
                            );
                        }
                    }
                }
            }

            return Platform::open_with_client_saved_state::<P>(
                drive,
                core_rpc,
                config,
                execution_state,
                checkpoint_platform_states,
            );
        }

        Platform::open_with_client_no_saved_state::<P>(
            drive,
            core_rpc,
            config,
            initial_protocol_version.unwrap_or(INITIAL_PROTOCOL_VERSION),
            initial_protocol_version.unwrap_or(INITIAL_PROTOCOL_VERSION),
        )
    }

    /// Open Platform with Drive and block execution context from saved state.
    pub fn open_with_client_saved_state<P: AsRef<Path>>(
        drive: Drive,
        core_rpc: C,
        config: PlatformConfig,
        platform_state: PlatformState,
        checkpoint_platform_states: BTreeMap<BlockHeight, Arc<PlatformState>>,
    ) -> Result<Platform<C>, Error>
    where
        C: CoreRPCLike,
    {
        let height = platform_state.last_committed_block_height();
        let platform_version =
            PlatformVersion::get(platform_state.current_protocol_version_in_consensus())
                .map_err(Error::from)?;

        PlatformVersion::set_current(platform_version);

        let platform: Platform<C> = Platform {
            drive,
            checkpoint_platform_states: ArcSwap::from_pointee(checkpoint_platform_states),
            state: ArcSwap::new(Arc::new(platform_state)),
            committed_block_height_guard: AtomicU64::from(height),
            config,
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
            &config,
        )?;

        let height = platform_state.last_committed_block_height();

        PlatformVersion::set_current(PlatformVersion::get(current_protocol_version_in_consensus)?);

        Ok(Platform {
            drive,
            checkpoint_platform_states: ArcSwap::from_pointee(BTreeMap::new()),
            state: ArcSwap::new(Arc::new(platform_state)),
            committed_block_height_guard: AtomicU64::from(height),
            config,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;

    #[test]
    fn debug_platform_state_ref_contains_state_and_config() {
        let platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let state = platform.state.load();
        let psr = PlatformStateRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
        };
        let s = format!("{:?}", psr);
        assert!(s.contains("state"));
        assert!(s.contains("config"));
    }

    #[test]
    fn debug_platform_is_minimal() {
        let platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let s = format!("{:?}", platform.platform);
        // The Debug impl intentionally outputs just "Platform"
        assert_eq!(s, "Platform");
    }

    #[test]
    fn platform_state_ref_from_platform_ref_forwards_fields() {
        let platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let state = platform.state.load();
        let pref = PlatformRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };
        let psr: PlatformStateRef = (&pref).into();
        // Both references should point at the same config and state
        assert!(std::ptr::eq(psr.config, pref.config));
        assert!(std::ptr::eq(psr.state, pref.state));
        assert!(std::ptr::eq(psr.drive, pref.drive));
    }

    #[test]
    fn open_with_latest_protocol_version_succeeds() {
        let tp = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        // Protocol version set correctly
        let state = tp.state.load();
        assert_eq!(
            state.current_protocol_version_in_consensus(),
            PlatformVersion::latest().protocol_version
        );
        // No checkpoint states on fresh DB
        assert!(tp.checkpoint_platform_states.load().is_empty());
    }

    #[test]
    fn open_with_initial_protocol_version_first_succeeds() {
        let first = PlatformVersion::first().protocol_version;
        let tp = TestPlatformBuilder::new()
            .with_initial_protocol_version(first)
            .build_with_mock_rpc();
        let state = tp.state.load();
        assert_eq!(state.current_protocol_version_in_consensus(), first);
    }

    #[test]
    fn committed_block_height_guard_initialized_to_zero_on_fresh_db() {
        let tp = TestPlatformBuilder::new().build_with_mock_rpc();
        use std::sync::atomic::Ordering;
        assert_eq!(tp.committed_block_height_guard.load(Ordering::Relaxed), 0);
    }
}
