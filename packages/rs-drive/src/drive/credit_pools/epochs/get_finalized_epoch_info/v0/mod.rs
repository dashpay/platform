use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::{EpochIndex, EPOCH_KEY_OFFSET};
use dpp::ProtocolError;
use grovedb::query_result_type::{QueryResultType};
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::serialization::PlatformDeserializable;
use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_FINISHED_EPOCH_INFO;
use crate::drive::credit_pools::pools_vec_path;
use crate::query::QueryItem;
use dpp::version::PlatformVersion;

impl Drive {
    /// Retrieves finalized epoch information for a given range of epochs.
    ///
    /// This method constructs a query over the stored finalized epoch information based on
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
    ///    of the keys are reversed and similar range variants are used, with the query’s
    ///    `left_to_right` flag set to `false`.
    ///
    /// Finally, the query is executed and the results are parsed into a vector of
    /// `FinalizedEpochInfo`.
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
    /// A `Result` containing a vector of `FinalizedEpochInfo` on success or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// - Returns a `ProtocolError::Overflow` if an epoch index plus the offset overflows.
    /// - Returns errors from the underlying storage query if the query fails.
    /// - Returns an empty vector if the range is empty due to exclusion of boundaries.
    ///
    pub(super) fn get_finalized_epoch_infos_v0<T: FromIterator<(EpochIndex, FinalizedEpochInfo)>>(
        &self,
        start_epoch_index: u16,
        start_epoch_index_included: bool,
        end_epoch_index: u16,
        end_epoch_index_included: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<T, Error> {
        // Compute the start and end keys with the offset.
        let start_index = start_epoch_index.checked_add(EPOCH_KEY_OFFSET)
            .ok_or(ProtocolError::Overflow("Stored epoch index too high"))?;
        let end_index = end_epoch_index.checked_add(EPOCH_KEY_OFFSET)
            .ok_or(ProtocolError::Overflow("Stored epoch index too high"))?;

        let start_key = start_index.to_be_bytes().to_vec();
        let end_key = end_index.to_be_bytes().to_vec();

        // Determine if the query should be ascending.
        let ascending = start_epoch_index <= end_epoch_index;

        // Build the query item based on the range and inclusivity parameters.
        let query_item = if start_epoch_index == end_epoch_index {
            // If the start and end are equal, only return a result if both boundaries are included.
            if start_epoch_index_included && end_epoch_index_included {
                QueryItem::Key(start_key)
            } else {
                // No epochs satisfy the range.
                return Ok(vec![]);
            }
        } else if ascending {
            // Ascending order: start_epoch_index < end_epoch_index.
            if start_epoch_index_included && end_epoch_index_included {
                QueryItem::RangeInclusive(start_key..=end_key)
            } else if start_epoch_index_included && !end_epoch_index_included {
                QueryItem::Range(start_key..end_key)
            } else if !start_epoch_index_included && end_epoch_index_included {
                QueryItem::RangeAfterToInclusive(start_key..=end_key)
            } else {
                QueryItem::RangeAfterTo(start_key..end_key)
            }
        } else {
            // Descending order: start_epoch_index > end_epoch_index.
            if start_epoch_index_included && end_epoch_index_included {
                QueryItem::RangeInclusive(end_key..=start_key)
            } else if start_epoch_index_included && !end_epoch_index_included {
                QueryItem::Range(end_key..start_key)
            } else if !start_epoch_index_included && end_epoch_index_included {
                QueryItem::RangeAfterToInclusive(end_key..=start_key)
            } else {
                QueryItem::RangeAfterTo(end_key..start_key)
            }
        };

        // Construct the query.
        let mut query = Query::new_single_query_item(query_item);
        query.left_to_right = ascending;
        query.set_subquery_key(KEY_FINISHED_EPOCH_INFO.to_vec());
        let path_query = PathQuery::new(
            pools_vec_path(),
            SizedQuery::new(query, None, None),
        );

        let results = self
            .grove_get_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryPathKeyElementTrioResultType,
                &mut vec![],
                &platform_version.drive,
            )?
            .0;
        
        results.to_path_key_elements().into_iter().map(|(mut path, _, element)| {
            let epoch_index_vec =
                path.pop()
                    .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                        "the path must have a last element".to_string(),
                    )))?;
            
            let epoch_index_bytes: [u8; 2] =
                epoch_index_vec.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "extended epoch info: item has an invalid length".to_string(),
                    ))
                })?;
            let epoch_index =
                EpochIndex::from_be_bytes(epoch_index_bytes)
                    .checked_sub(EPOCH_KEY_OFFSET)
                    .ok_or(Error::Drive(DriveError::CorruptedSerialization(
                        "epoch bytes on disk too small, should be over epoch key offset"
                            .to_string(),
                    )))?;
            
            let item_bytes = element.as_item_bytes()?;

            let epoch_info = FinalizedEpochInfo::deserialize_from_bytes(item_bytes)?;
            
            Ok((epoch_index, epoch_info))
        }).collect::<Result<T, Error>>()
    }
}
