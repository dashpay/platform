use crate::drive::Drive;
use crate::error::Error;
use grovedb::operations::delete::ClearOptions;
use grovedb::TransactionArg;
use grovedb_path::SubtreePath;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes the `OperationCost` of deleting an element in groveDB to `drive_operations`.
    pub(crate) fn grove_clear_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let options = ClearOptions {
            check_for_subtrees: false,
            allow_deleting_subtrees: false,
            trying_to_clear_with_subtrees_returns_error: false,
        };

        #[cfg(feature = "grovedb_operations_logging")]
        let maybe_params_for_logs = if tracing::event_enabled!(target: "drive_grovedb_operations", tracing::Level::TRACE)
        {
            let root_hash = self
                .grove
                .root_hash(transaction, &drive_version.grove_version)
                .unwrap()
                .map_err(Error::GroveDB)?;

            Some((path.clone(), root_hash))
        } else {
            None
        };

        // we will always return true if there is no error when we don't check for subtrees
        #[allow(clippy::let_and_return)] // due to feature below; we must allow this lint here
        let result = self
            .grove
            .clear_subtree(
                path,
                Some(options),
                transaction,
                &drive_version.grove_version,
            )
            .map_err(Error::GroveDB)
            .map(|_| ());

        #[cfg(feature = "grovedb_operations_logging")]
        if tracing::event_enabled!(target: "drive_grovedb_operations", tracing::Level::TRACE)
            && result.is_ok()
        {
            let root_hash = self
                .grove
                .root_hash(transaction, &drive_version.grove_version)
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
