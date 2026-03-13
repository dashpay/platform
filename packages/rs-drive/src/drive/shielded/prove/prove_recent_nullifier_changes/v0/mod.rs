use crate::drive::shielded::nullifiers::queries::shielded_recent_nullifiers_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation for proving nullifier changes from a start height.
    pub(super) fn prove_recent_nullifier_changes_v0(
        &self,
        start_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = shielded_recent_nullifiers_path_vec();

        // Create a range query starting from the specified height
        let mut query = Query::new();
        query.insert_range_from(start_height.to_be_bytes().to_vec()..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
