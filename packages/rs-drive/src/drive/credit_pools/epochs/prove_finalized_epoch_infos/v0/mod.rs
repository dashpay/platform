use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves finalized epoch information for a given range of epochs.
    ///
    /// This method constructs a proof query over the stored finalized epoch information based on
    /// the epoch indices provided. The query range is determined using:
    ///
    /// - `start_epoch_index` and `end_epoch_index`: the epoch indices (of type `u16`).
    /// - `start_epoch_index_included` and `end_epoch_index_included`: booleans specifying
    ///   whether the start and end boundaries are included in the query.
    ///
    /// Before constructing the query, an internal offset (`EPOCH_KEY_OFFSET`) is added to
    /// the epoch indices, and the resulting values are converted to big‑endian byte arrays.
    ///
    /// The query is then built using one of several `QueryItem` variants, depending on the
    /// following cases:
    ///
    /// 1. **Single Key Query:** If `start_epoch_index == end_epoch_index` and both boundaries
    ///    are included, the query returns exactly that key (using `QueryItem::Key`).
    ///    If either boundary is excluded, the result is empty.
    ///
    /// 2. **Ascending Range Query:** If `start_epoch_index < end_epoch_index`, the query is
    ///    constructed in ascending order:
    ///    - **Both boundaries included:** `QueryItem::RangeInclusive(start_key, end_key)`.
    ///    - **Start included, end excluded:** `QueryItem::Range(start_key, end_key)`.
    ///    - **Start excluded, end included:** `QueryItem::RangeAfterToInclusive(start_key, end_key)`.
    ///    - **Both boundaries excluded:** `QueryItem::RangeAfterTo(start_key, end_key)`.
    ///
    /// 3. **Descending Range Query:** If `start_epoch_index > end_epoch_index`, the roles
    ///    of the keys are reversed and similar range variants are used, with the query's
    ///    `left_to_right` flag set to `false`.
    ///
    /// Finally, the query is executed to generate a proof.
    ///
    /// # Parameters
    ///
    /// - `start_epoch_index` (`u16`): The starting epoch index for the query.
    /// - `start_epoch_index_included` (`bool`): If `true`, the epoch at `start_epoch_index` is included.
    /// - `end_epoch_index` (`u16`): The ending epoch index for the query.
    /// - `end_epoch_index_included` (`bool`): If `true`, the epoch at `end_epoch_index` is included.
    /// - `transaction` (`TransactionArg`): The current GroveDB transaction.
    /// - `platform_version` (`&PlatformVersion`): The platform version to use for method dispatch.
    ///
    /// # Returns
    ///
    /// A `Result` containing a proof as a byte vector on success or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// - Returns a `ProtocolError::Overflow` if an epoch index plus the offset overflows.
    /// - Returns errors from the underlying storage query if the query fails.
    /// - Returns an empty proof if the range is empty due to exclusion of boundaries.
    ///
    pub(super) fn prove_finalized_epoch_infos_v0(
        &self,
        start_epoch_index: u16,
        start_epoch_index_included: bool,
        end_epoch_index: u16,
        end_epoch_index_included: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let Some(path_query) = Drive::finalized_epoch_infos_query(
            start_epoch_index,
            start_epoch_index_included,
            end_epoch_index,
            end_epoch_index_included,
        )?
        else {
            return Err(Error::Query(QuerySyntaxError::NoQueryItems(
                "the end epoch index is the start epoch index and they are not included",
            )));
        };

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
