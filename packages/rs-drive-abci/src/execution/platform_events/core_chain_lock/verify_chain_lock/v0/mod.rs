use crate::config::PlatformConfig;
use crate::error::Error;
use dpp::dashcore::ChainLock;
use dpp::version::PlatformVersion;

use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;

use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn verify_chain_lock_v0(
        &self,
        platform_state: &PlatformState,
        chain_lock: &ChainLock,
        make_sure_core_is_synced: bool,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        // first we try to verify the chain lock locally
        if let Some(valid) =
            self.verify_chain_lock_locally(platform_state, chain_lock, platform_version)?
        {
            if valid && make_sure_core_is_synced {
                self.make_sure_core_is_synced_to_chain_lock(chain_lock, platform_version)?;
            }
            Ok(valid)
        } else {
            self.verify_chain_lock_through_core(
                chain_lock,
                make_sure_core_is_synced,
                platform_version,
            )
        }
    }
}
