use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_FINISHED_EPOCH_INFO;
use crate::drive::credit_pools::pools_vec_path;
use crate::drive::Drive;
use crate::query::{Query, QueryItem};
use dpp::block::epoch::EPOCH_KEY_OFFSET;
use dpp::ProtocolError;
use grovedb::{PathQuery, SizedQuery};

impl Drive {
    /// Constructs a query for retrieving finalized epoch information within a specified range.
    ///
    /// This method builds a `PathQuery` that can be used to query finalized epoch information
    /// from the storage layer. The query supports flexible range specifications with optional
    /// inclusion/exclusion of boundaries and automatic handling of ascending/descending order.
    ///
    /// # Parameters
    ///
    /// - `start_epoch_index`: The starting epoch index for the query range.
    /// - `start_epoch_index_included`: If `true`, includes the start epoch in the results.
    /// - `end_epoch_index`: The ending epoch index for the query range.
    /// - `end_epoch_index_included`: If `true`, includes the end epoch in the results.
    ///
    /// # Returns
    ///
    /// Returns `Some(PathQuery)` if a valid query can be constructed, or `None` if:
    /// - The epoch indices would overflow when adding the internal offset
    /// - The range is empty (e.g., start equals end but boundaries are excluded)
    ///
    /// # Query Behavior
    ///
    /// The method automatically determines the query direction:
    /// - **Ascending**: When `start_epoch_index <= end_epoch_index`
    /// - **Descending**: When `start_epoch_index > end_epoch_index`
    ///
    /// For single epoch queries (start equals end), both boundaries must be included
    /// to return a result.
    ///
    pub fn finalized_epoch_infos_query(
        start_epoch_index: u16,
        start_epoch_index_included: bool,
        end_epoch_index: u16,
        end_epoch_index_included: bool,
    ) -> Result<Option<PathQuery>, ProtocolError> {
        // Compute the start and end keys with the offset.
        let start_index = start_epoch_index
            .checked_add(EPOCH_KEY_OFFSET)
            .ok_or(ProtocolError::Overflow("Stored epoch index too high"))?;
        let end_index = end_epoch_index
            .checked_add(EPOCH_KEY_OFFSET)
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
                return Ok(None);
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
        Ok(Some(PathQuery::new(
            pools_vec_path(),
            SizedQuery::new(query, None, None),
        )))
    }
}
