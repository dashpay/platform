use crate::drive::shielded::nullifiers::queries::shielded_compacted_nullifiers_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 0 implementation for proving compacted nullifier changes.
    ///
    /// Uses a two-step approach:
    /// 1. First query (non-proving): descending to find any range containing start_block_height
    /// 2. Second query (proving): ascending from the found start_block or start_block_height
    ///
    /// This ensures the proof covers all relevant ranges efficiently.
    pub(super) fn prove_compacted_nullifier_changes_v0(
        &self,
        start_block_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = shielded_compacted_nullifiers_path_vec();

        // Step 1: Non-proving descending query to find any range containing start_block_height
        let mut desc_end_key = Vec::with_capacity(16);
        desc_end_key.extend_from_slice(&start_block_height.to_be_bytes());
        desc_end_key.extend_from_slice(&u64::MAX.to_be_bytes());

        let mut desc_query = Query::new_with_direction(false); // descending
        desc_query.insert_range_to_inclusive(..=desc_end_key);

        let desc_path_query =
            PathQuery::new(path.clone(), SizedQuery::new(desc_query, Some(1), None));

        let (desc_results, _) = self.grove_get_path_query(
            &desc_path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        // Determine the actual start key for the proved query
        // If we found a containing range, use its exact key
        // Otherwise use (start_block_height, start_block_height) since end_block >= start_block always
        let start_key = if let Some((key, _)) = desc_results.to_key_elements().into_iter().next() {
            if key.len() == 16 {
                let end_block = u64::from_be_bytes(key[8..16].try_into().map_err(|_| {
                    Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(
                        "invalid compacted key slice".to_string(),
                    )))
                })?);
                // If this range contains start_block_height, use its exact key
                if end_block >= start_block_height {
                    key
                } else {
                    // No containing range, use (start_block_height, start_block_height)
                    let mut key = Vec::with_capacity(16);
                    key.extend_from_slice(&start_block_height.to_be_bytes());
                    key.extend_from_slice(&start_block_height.to_be_bytes());
                    key
                }
            } else {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "invalid compacted block key length, expected 16 bytes".to_string(),
                    ),
                )));
            }
        } else {
            let mut key = Vec::with_capacity(16);
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key
        };

        // Step 2: Proved ascending query from start_key

        let mut query = Query::new();
        query.insert_range_from(start_key..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
