use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use drive::error::Error as DriveError;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    pub(super) fn migration_30_test(
        &self,
        _block_platform_state: &mut PlatformState,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        self.drive
            .grove
            .put_aux(
                "migration_42_example",
                b"migration_42_example",
                None,
                Some(transaction),
            )
            .unwrap()
            .map_err(DriveError::GroveDB)?;

        Ok(())
    }
}
