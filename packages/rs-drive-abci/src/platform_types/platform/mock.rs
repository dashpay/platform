use crate::config::PlatformConfig;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::rpc::core::MockCoreRPCLike;
use dpp::dashcore::BlockHash;
use dpp::serialization::PlatformDeserializableFromVersionedStructure;
use dpp::version::PlatformVersionCurrentVersion;
use dpp::version::{PlatformVersion, ProtocolVersion};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

impl Platform<MockCoreRPCLike> {
    /// Open Platform with Drive and block execution context and mock core rpc.
    pub fn open<P: AsRef<Path>>(
        path: P,
        config: Option<PlatformConfig>,
        initial_protocol_version: Option<ProtocolVersion>,
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
        Self::open_with_client(path, config, core_rpc_mock, initial_protocol_version)
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

        // Reload checkpoint platform states from disk
        let mut checkpoint_platform_states = BTreeMap::new();
        let checkpoints = self.drive.checkpoints.load();
        for (&block_height, _checkpoint_info) in checkpoints.iter() {
            let checkpoint_state_path = self
                .config
                .db_path
                .join("checkpoints")
                .join(block_height.to_string())
                .join("platform_state.bin");

            if checkpoint_state_path.exists() {
                if let Ok(state_bytes) = std::fs::read(&checkpoint_state_path) {
                    if let Ok(state) =
                        PlatformState::versioned_deserialize(&state_bytes, platform_version)
                    {
                        checkpoint_platform_states.insert(block_height, Arc::new(state));
                    }
                }
            }
        }
        self.checkpoint_platform_states
            .store(Arc::new(checkpoint_platform_states));

        self.state.store(Arc::new(persisted_state));

        Ok(true)
    }
}
