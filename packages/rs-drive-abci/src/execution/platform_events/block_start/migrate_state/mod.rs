#[cfg(feature = "test-patch-platform")]
mod migration_30_test;

use crate::error::Error;

use dpp::prelude::BlockHeight;
use drive::grovedb::Transaction;

use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;

impl<C> Platform<C> {
    /// Perform state migration based on block height
    pub fn migrate_state(
        &self,
        height: BlockHeight,
        block_platform_state: &mut PlatformState,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        match height {
            #[cfg(feature = "test-patch-platform")]
            30 => self.migration_30_test(block_platform_state, transaction)?,
            _ => {}
        }

        Ok(())
    }
}
