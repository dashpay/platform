use crate::drive::Drive;
use crate::error::Error;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

/// Result type for fetched nullifier changes per block
pub type NullifierChangesPerBlock = Vec<(u64, Vec<[u8; 32]>)>;

impl Drive {
    /// Version 0 implementation of fetching nullifier changes from a start height.
    ///
    /// Retrieves all nullifier change records from `start_height` onwards.
    /// Returns a vector of (block_height, nullifiers) tuples.
    pub(super) fn fetch_recent_nullifier_changes_v0(
        &self,
        start_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<NullifierChangesPerBlock, Error> {
        let path = Self::saved_block_transactions_nullifiers_path_vec();

        // Create a range query starting from the specified height
        let mut query = Query::new();
        query.insert_range_from(start_height.to_be_bytes().to_vec()..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        let (results, _) = self.grove_get_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let mut nullifier_changes = Vec::new();

        for (key, element) in results.to_key_elements() {
            // Parse block height from key (8 bytes, big-endian)
            let height_bytes: [u8; 8] = key.try_into().map_err(|_| {
                Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(
                    "invalid block height key length".to_string(),
                )))
            })?;
            let block_height = u64::from_be_bytes(height_bytes);

            // Get the serialized data from the ItemWithSumItem element
            let Element::ItemWithSumItem(serialized_data, _, _) = element else {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "expected item with sum item element for nullifiers".to_string(),
                    ),
                )));
            };

            // Deserialize the nullifier list
            let (nullifiers, _): (Vec<[u8; 32]>, usize) =
                bincode::decode_from_slice(&serialized_data, config).map_err(|e| {
                    Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                        "cannot decode nullifiers: {}",
                        e
                    ))))
                })?;

            nullifier_changes.push((block_height, nullifiers));
        }

        Ok(nullifier_changes)
    }

    /// Version 0 implementation for proving nullifier changes from a start height.
    pub(super) fn prove_recent_nullifier_changes_v0(
        &self,
        start_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = Self::saved_block_transactions_nullifiers_path_vec();

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
