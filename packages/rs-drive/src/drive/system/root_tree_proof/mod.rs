use crate::drive::Drive;
use crate::error::Error;
use grovedb::{PathQuery, Query, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Returns a tree proof of the root of the tree.
    pub fn root_tree_proof(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = PathQuery::new_unsized(vec![], Query::new_range_full());
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
