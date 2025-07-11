use crate::drive::platform_state::REDUCED_PLATFORM_STATE_KEY;
use crate::drive::system::misc_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::{PathQuery, Query, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn fetch_reduced_platform_state_bytes_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, Error> {
        let mut query = Query::new();
        query.insert_key(Vec::from(REDUCED_PLATFORM_STATE_KEY));

        let path_query = PathQuery::new_unsized(misc_path_vec(), query.clone());

        let (res, _) = self
            .grove
            .query_item_value(
                &path_query,
                true,
                false,
                true,
                transaction,
                &platform_version.drive.grove_version,
            )
            .value?;

        if res.len() != 1 {
            return Err(Error::GroveDB(grovedb::Error::InvalidQuery(
                "Invalid number of reduced platform state elements",
            )));
        }
        Ok(res.into_iter().next())
    }
}
