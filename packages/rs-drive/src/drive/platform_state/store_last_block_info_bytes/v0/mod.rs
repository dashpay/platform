use crate::drive::platform_state::LAST_BLOCK_INFO_KEY;
use crate::drive::system::misc_path;
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::operations::insert::InsertOptions;
use grovedb::{Element, TransactionArg};

impl Drive {
    pub(super) fn store_last_block_info_bytes_v0(
        &self,
        last_block_info_bytes: &[u8],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.grove
            .insert(
                &misc_path(),
                LAST_BLOCK_INFO_KEY,
                Element::Item(last_block_info_bytes.to_vec(), None),
                Some(InsertOptions::default()),
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;
        Ok(())
    }
}
