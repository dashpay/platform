use crate::drive::Drive;
use crate::error::Error;
use grovedb::operations::delete::ClearOptions;
use grovedb::TransactionArg;
use grovedb_path::SubtreePath;

impl Drive {
    /// Pushes the `OperationCost` of deleting an element in groveDB to `drive_operations`.
    pub(super) fn grove_clear_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let options = ClearOptions {
            check_for_subtrees: false,
            allow_deleting_subtrees: false,
            trying_to_clear_with_subtrees_returns_error: false,
        };
        // we will always return true if there is no error when we don't check for subtrees
        self.grove
            .clear_subtree(path, Some(options), transaction)
            .map_err(Error::GroveDB)
            .map(|_| ())
    }
}
