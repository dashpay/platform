mod migration_42_example;

use crate::error::Error;

use dpp::block::block_info::BlockInfo;

use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;

impl<C> Platform<C> {
    /// Perform state migration based on block information
    pub fn migrate_state(
        &self,
        block_info: &BlockInfo,
        block_platform_state: &mut PlatformState,
    ) -> Result<(), Error> {
        // Implement functions in a separate modules with meaningful names and block height
        match block_info.height {
            42 => self.migration_42_example(block_info, block_platform_state),
            52 => self.migration_42_example(block_info, block_platform_state),
            _ => {}
        }

        Ok(())
    }
}
