use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::prelude::BlockHeight;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// This function patches platform version and run migrations
    /// It modifies protocol version to function version mapping to apply hotfixes
    /// Also it performs migrations to fix corrupted state or prepare it for new features
    ///
    /// This function appends the patch to PlatformState, potentially alter Drive and Platform execution state
    /// and returns patched version
    pub fn apply_platform_version_patch_and_migrate_state_for_height(
        &self,
        height: BlockHeight,
        platform_state: &mut PlatformState,
        transaction: &Transaction,
    ) -> Result<Option<&'static PlatformVersion>, Error> {
        let patched_platform_version =
            platform_state.apply_platform_version_patch_for_height(height)?;

        self.migrate_state_for_height(height, platform_state, transaction)?;

        Ok(patched_platform_version)
    }
}
