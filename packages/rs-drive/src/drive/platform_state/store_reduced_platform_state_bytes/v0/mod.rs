use crate::drive::platform_state::REDUCED_PLATFORM_STATE_KEY;
use crate::drive::system::misc_path;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::operations::insert::InsertOptions;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn store_reduced_platform_state_bytes_v0(
        &self,
        reduced_state_bytes: &[u8],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.grove
            .insert(
                &misc_path(),
                REDUCED_PLATFORM_STATE_KEY,
                Element::Item(reduced_state_bytes.to_vec(), None),
                Some(InsertOptions::default()),
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;
        Ok(())
    }
}
