use crate::drive::Drive;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

/// Result type for fetched compacted address balance changes
/// Each entry is (start_block, end_block, address_balance_map)
pub type CompactedAddressBalanceChanges =
    Vec<(u64, u64, BTreeMap<PlatformAddress, CreditOperation>)>;

impl Drive {
    /// Version 0 implementation of fetching compacted address balance changes.
    ///
    /// Retrieves all compacted address balance change records from `start_block_height` onwards.
    /// Returns a vector of (start_block, end_block, address_balance_map) tuples.
    pub(super) fn fetch_compacted_address_balance_changes_v0(
        &self,
        start_block_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<CompactedAddressBalanceChanges, Error> {
        let path = Self::saved_compacted_block_transactions_address_balances_path_vec();

        // Create a range query starting from the specified height
        // Keys are 16 bytes: (start_block, end_block), both big-endian
        // We query from (start_block_height, 0) onwards
        let mut start_key = Vec::with_capacity(16);
        start_key.extend_from_slice(&start_block_height.to_be_bytes());
        start_key.extend_from_slice(&0u64.to_be_bytes());

        let mut query = Query::new();
        query.insert_range_from(start_key..);

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

        let mut compacted_changes = Vec::new();

        for (key, element) in results.to_key_elements() {
            // Parse start_block and end_block from key (16 bytes total, both big-endian)
            if key.len() != 16 {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "invalid compacted block key length, expected 16 bytes".to_string(),
                    ),
                )));
            }

            let start_block = u64::from_be_bytes(key[0..8].try_into().unwrap());
            let end_block = u64::from_be_bytes(key[8..16].try_into().unwrap());

            // Get the serialized data from the Item element
            let Element::Item(serialized_data, _) = element else {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "expected item element for compacted address balances".to_string(),
                    ),
                )));
            };

            // Deserialize the address balance map
            let (address_balances, _): (BTreeMap<PlatformAddress, CreditOperation>, usize) =
                bincode::decode_from_slice(&serialized_data, config).map_err(|e| {
                    Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                        "cannot decode compacted address balances: {}",
                        e
                    ))))
                })?;

            compacted_changes.push((start_block, end_block, address_balances));
        }

        Ok(compacted_changes)
    }

    /// Version 0 implementation for proving compacted address balance changes.
    pub(super) fn prove_compacted_address_balance_changes_v0(
        &self,
        start_block_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = Self::saved_compacted_block_transactions_address_balances_path_vec();

        // Create a range query starting from the specified height
        let mut start_key = Vec::with_capacity(16);
        start_key.extend_from_slice(&start_block_height.to_be_bytes());
        start_key.extend_from_slice(&0u64.to_be_bytes());

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
