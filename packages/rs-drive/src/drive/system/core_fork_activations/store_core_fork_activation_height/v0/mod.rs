use crate::drive::system::core_fork_activations::CoreFork;
use crate::drive::system::misc_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::prelude::CoreBlockHeight;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    #[inline(always)]
    pub(super) fn store_core_fork_activation_height_v0(
        &self,
        fork: CoreFork,
        core_height: CoreBlockHeight,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let inserted = self.grove_insert_if_not_exists(
            (&misc_path()).into(),
            fork.activation_key(),
            Element::Item(core_height.to_be_bytes().to_vec(), None),
            transaction,
            None,
            &platform_version.drive,
        )?;
        if !inserted {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "a Core fork activation is recorded only once",
            )));
        }
        Ok(())
    }
}
