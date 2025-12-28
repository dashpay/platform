use crate::drive::Drive;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

/// Result type for fetched address balance changes
pub type AddressBalanceChangesPerBlock = Vec<(u64, BTreeMap<PlatformAddress, CreditOperation>)>;

impl Drive {
    /// Version 0 implementation of fetching address balance changes from a start height.
    ///
    /// Retrieves all address balance change records from `start_height` onwards.
    /// Returns a vector of (block_height, address_balance_map) tuples.
    pub(super) fn fetch_recent_address_balance_changes_v0(
        &self,
        start_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<AddressBalanceChangesPerBlock, Error> {
        let path = Self::saved_block_transactions_address_balances_path_vec();

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

        let mut address_balance_changes = Vec::new();

        for (key, element) in results.to_key_elements() {
            // Parse block height from key (8 bytes, big-endian)
            let height_bytes: [u8; 8] = key.try_into().map_err(|_| {
                Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(
                    "invalid block height key length".to_string(),
                )))
            })?;
            let block_height = u64::from_be_bytes(height_bytes);

            // Get the serialized data from the element
            let serialized_data = element.into_item_bytes().map_err(|e| {
                Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                    "expected item element for address balances: {:?}",
                    e
                ))))
            })?;

            // Deserialize the address balance map
            let (address_balances, _): (BTreeMap<PlatformAddress, CreditOperation>, usize) =
                bincode::decode_from_slice(&serialized_data, config).map_err(|e| {
                    Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                        "cannot decode address balances: {}",
                        e
                    ))))
                })?;

            address_balance_changes.push((block_height, address_balances));
        }

        Ok(address_balance_changes)
    }

    /// Version 0 implementation for proving address balance changes from a start height.
    pub(super) fn prove_recent_address_balance_changes_v0(
        &self,
        start_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = Self::saved_block_transactions_address_balances_path_vec();

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
