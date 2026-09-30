use crate::drive::credit_pools::pending_epoch_refunds::pending_epoch_refunds_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::balances::credits::Creditable;
use dpp::fee::epoch::CreditsPerEpoch;
use grovedb::TransactionArg;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Fetches all pending epoch refunds
    pub(super) fn fetch_pending_epoch_refunds_v0(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<CreditsPerEpoch, Error> {
        // Edited in place in this shipped generation: the query and the reading of its sum
        // items moved, unchanged, into `fetch_sum_items`, which the lifetime storage fee pools
        // share. Only which of two corruption errors a corrupted tree reports first can differ.
        self.fetch_sum_items(
            pending_epoch_refunds_path_vec(),
            "pending refund credits must be sum items",
            transaction,
            drive_version,
        )?
        .into_iter()
        .map(|(epoch_index_key, credits)| {
            let epoch_index =
                u16::from_be_bytes(epoch_index_key.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(String::from(
                        "epoch index for pending pool updates must be i64",
                    )))
                })?);

            Ok((epoch_index, credits.to_unsigned()))
        })
        .collect::<Result<CreditsPerEpoch, Error>>()
    }
}
