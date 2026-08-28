use crate::drive::platform_state::REDUCED_PLATFORM_STATE_KEY;
use crate::drive::system::misc_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn fetch_reduced_platform_state_bytes_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.grove_get_raw_optional_item(
            (&misc_path()).into(),
            REDUCED_PLATFORM_STATE_KEY,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
