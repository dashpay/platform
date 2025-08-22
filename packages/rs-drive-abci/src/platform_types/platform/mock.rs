use crate::config::PlatformConfig;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::MockCoreRPCLike;
use dpp::dashcore::BlockHash;
use dpp::version::PlatformVersionCurrentVersion;
use dpp::version::{PlatformVersion, ProtocolVersion};
use serde_json::json;
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

        self.state.store(Arc::new(persisted_state));

        Ok(true)
    }
}
