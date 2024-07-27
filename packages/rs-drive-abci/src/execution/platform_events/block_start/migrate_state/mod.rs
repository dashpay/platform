use crate::error::Error;

use dpp::prelude::BlockHeight;
use drive::grovedb::Transaction;

use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;

impl<C> Platform<C> {
    /// Perform state migration based on block height
    pub fn migrate_state_for_height(
        &self,
        height: BlockHeight,
        _block_platform_state: &mut PlatformState,
        _transaction: &Transaction,
    ) -> Result<(), Error> {
        #[allow(clippy::match_single_binding)]
        let is_migrated = match height {
            // 30 => self.migration_30_test(block_platform_state, transaction)?,
            _ => false,
        };

        if is_migrated {
            tracing::debug!("Successfully migrated state for height {}", height);
        }

        Ok(())
    }
}
