use crate::drive::Drive;
use crate::error::Error;
use grovedb::operations::delete::ClearOptions;
use grovedb::TransactionArg;
use grovedb_path::SubtreePath;
use tracing::Level;

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

        #[cfg(feature = "grovedb_operations_logging")]
        let maybe_params_for_logs = if tracing::event_enabled!(target: "drive_grovedb_operations", Level::TRACE)
        {
            let root_hash = self
                .grove
                .root_hash(transaction)
                .unwrap()
                .map_err(Error::GroveDB)?;

            Some((path.clone(), root_hash))
        } else {
            None
        };

        // we will always return true if there is no error when we don't check for subtrees
        let result = self
            .grove
            .clear_subtree(path, Some(options), transaction)
            .map_err(Error::GroveDB)
            .map(|_| ());

        #[cfg(feature = "grovedb_operations_logging")]
        if tracing::event_enabled!(target: "drive_grovedb_operations", Level::TRACE)
            && result.is_ok()
        {
            let root_hash = self
                .grove
                .root_hash(transaction)
                .unwrap()
                .map_err(Error::GroveDB)?;

            let (path, previous_root_hash) =
                maybe_params_for_logs.expect("log params should be set above");

            tracing::trace!(
                target: "drive_grovedb_operations",
                path = ?path.to_vec(),
                ?root_hash,
                ?previous_root_hash,
                is_transactional = transaction.is_some(),
                "grovedb clear",
            );
        }

        result
    }
}
