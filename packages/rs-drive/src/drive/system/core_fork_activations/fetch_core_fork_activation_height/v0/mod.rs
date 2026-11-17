use crate::drive::system::core_fork_activations::CoreFork;
use crate::drive::system::misc_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::prelude::CoreBlockHeight;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_core_fork_activation_height_v0(
        &self,
        fork: CoreFork,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<CoreBlockHeight>, Error> {
        let Some(element) = self.grove_get_raw_optional(
            (&misc_path()).into(),
            fork.activation_key(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?
        else {
            return Ok(None);
        };
        let bytes = element.into_item_bytes()?;
        let core_height = u32::from_be_bytes(bytes.try_into().map_err(|_| {
            Error::Drive(DriveError::CorruptedSerialization(
                "a Core fork activation height must be a u32".to_string(),
            ))
        })?);
        Ok(Some(core_height))
    }
}
