use crate::drive::Drive;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::ProtocolError;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
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

            // Get the serialized data from the ItemWithSumItem element
            let Element::ItemWithSumItem(serialized_data, _, _) = element else {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "expected item with sum item element for address balances".to_string(),
                    ),
                )));
            };

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

    /// Version 0 implementation of fetching address balance changes after a height
    /// (exclusive start).
    ///
    /// Uses `RangeAfter` so that `after_height` becomes a boundary node in proofs
    /// rather than a result element. This enables `key_exists_as_boundary` to
    /// detect whether the cursor height still exists in the tree.
    pub(super) fn fetch_recent_address_balance_changes_after_v0(
        &self,
        after_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<AddressBalanceChangesPerBlock, Error> {
        let path = Self::saved_block_transactions_address_balances_path_vec();

        // Create an exclusive range query starting after the specified height
        let mut query = Query::new();
        query.insert_range_after(after_height.to_be_bytes().to_vec()..);

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

            // Get the serialized data from the ItemWithSumItem element
            let Element::ItemWithSumItem(serialized_data, _, _) = element else {
                return Err(Error::Protocol(Box::new(
                    ProtocolError::CorruptedSerialization(
                        "expected item with sum item element for address balances".to_string(),
                    ),
                )));
            };

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

    /// Version 0 implementation for proving address balance changes after a height
    /// (exclusive start).
    ///
    /// Uses `RangeAfter` so that `after_height` becomes a boundary node in proofs
    /// rather than a result element.
    pub(super) fn prove_recent_address_balance_changes_after_v0(
        &self,
        after_height: u64,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = Self::saved_block_transactions_address_balances_path_vec();

        // Create an exclusive range query starting after the specified height
        let mut query = Query::new();
        query.insert_range_after(after_height.to_be_bytes().to_vec()..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::address_funds::PlatformAddress;
    use dpp::balances::credits::CreditOperation;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_fetch_recent_address_balance_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address_1 = PlatformAddress::P2pkh([10; 20]);
        let address_2 = PlatformAddress::P2sh([11; 20]);

        // Store balance changes for block 100
        let mut block_100_changes = BTreeMap::new();
        block_100_changes.insert(address_1, CreditOperation::SetCredits(500_000));
        block_100_changes.insert(address_2, CreditOperation::AddToCredits(200_000));

        drive
            .store_address_balances_for_block(
                &block_100_changes,
                100,
                100_000,
                None,
                platform_version,
            )
            .expect("should store block 100 balances");

        // Store balance changes for block 200
        let mut block_200_changes = BTreeMap::new();
        block_200_changes.insert(address_1, CreditOperation::AddToCredits(100_000));

        drive
            .store_address_balances_for_block(
                &block_200_changes,
                200,
                200_000,
                None,
                platform_version,
            )
            .expect("should store block 200 balances");

        // Fetch from height 0 to get all blocks
        let result = drive
            .fetch_recent_address_balance_changes(0, None, None, platform_version)
            .expect("should fetch recent address balance changes");

        assert_eq!(result.len(), 2, "should have 2 blocks of changes");

        // Verify block 100
        let (height, balances) = &result[0];
        assert_eq!(*height, 100);
        assert_eq!(balances.len(), 2);
        assert_eq!(
            balances.get(&address_1),
            Some(&CreditOperation::SetCredits(500_000))
        );
        assert_eq!(
            balances.get(&address_2),
            Some(&CreditOperation::AddToCredits(200_000))
        );

        // Verify block 200
        let (height, balances) = &result[1];
        assert_eq!(*height, 200);
        assert_eq!(balances.len(), 1);
        assert_eq!(
            balances.get(&address_1),
            Some(&CreditOperation::AddToCredits(100_000))
        );
    }

    #[test]
    fn should_fetch_empty_address_balance_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Fetch from height 500 with no data stored at all
        let result = drive
            .fetch_recent_address_balance_changes(500, None, None, platform_version)
            .expect("should fetch empty address balance changes");

        assert!(result.is_empty(), "should have no changes");
    }

    #[test]
    fn should_fetch_address_balance_changes_with_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address = PlatformAddress::P2pkh([10; 20]);

        // Store 3 blocks of changes
        for block_height in [100u64, 200, 300] {
            let mut changes = BTreeMap::new();
            changes.insert(address, CreditOperation::AddToCredits(block_height * 1000));
            drive
                .store_address_balances_for_block(
                    &changes,
                    block_height,
                    block_height * 1000,
                    None,
                    platform_version,
                )
                .expect("should store balances");
        }

        // Fetch from height 0 with limit 2
        let result = drive
            .fetch_recent_address_balance_changes(0, Some(2), None, platform_version)
            .expect("should fetch with limit");

        assert_eq!(result.len(), 2, "should have 2 blocks of changes (limited)");
        assert_eq!(result[0].0, 100);
        assert_eq!(result[1].0, 200);
    }

    #[test]
    fn should_prove_recent_address_balance_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address_1 = PlatformAddress::P2pkh([10; 20]);
        let address_2 = PlatformAddress::P2sh([11; 20]);

        // Store balance changes for block 100
        let mut block_100_changes = BTreeMap::new();
        block_100_changes.insert(address_1, CreditOperation::SetCredits(500_000));
        block_100_changes.insert(address_2, CreditOperation::AddToCredits(200_000));

        drive
            .store_address_balances_for_block(
                &block_100_changes,
                100,
                100_000,
                None,
                platform_version,
            )
            .expect("should store block 100 balances");

        // Store balance changes for block 200
        let mut block_200_changes = BTreeMap::new();
        block_200_changes.insert(address_1, CreditOperation::AddToCredits(100_000));

        drive
            .store_address_balances_for_block(
                &block_200_changes,
                200,
                200_000,
                None,
                platform_version,
            )
            .expect("should store block 200 balances");

        // Prove from block 100
        let proof = drive
            .prove_recent_address_balance_changes(100, None, None, platform_version)
            .expect("should prove recent address balance changes");

        assert!(!proof.is_empty(), "proof should be non-empty bytes");
    }

    #[test]
    fn should_fetch_recent_address_balance_changes_after() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address = PlatformAddress::P2pkh([10; 20]);

        // Store 3 blocks of changes at heights 100, 200, 300
        for block_height in [100u64, 200, 300] {
            let mut changes = BTreeMap::new();
            changes.insert(address, CreditOperation::AddToCredits(block_height * 1000));
            drive
                .store_address_balances_for_block(
                    &changes,
                    block_height,
                    block_height * 1000,
                    None,
                    platform_version,
                )
                .expect("should store balances");
        }

        // Fetch after height 100 (exclusive), should return heights 200 and 300
        let result = drive
            .fetch_recent_address_balance_changes_after(100, None, None, platform_version)
            .expect("should fetch address balance changes after height 100");

        assert_eq!(result.len(), 2, "should have 2 blocks (200 and 300)");
        assert_eq!(result[0].0, 200);
        assert_eq!(result[1].0, 300);

        // Verify data at height 200
        assert_eq!(
            result[0].1.get(&address),
            Some(&CreditOperation::AddToCredits(200_000))
        );

        // Verify data at height 300
        assert_eq!(
            result[1].1.get(&address),
            Some(&CreditOperation::AddToCredits(300_000))
        );
    }

    #[test]
    fn should_prove_recent_address_balance_changes_after() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address = PlatformAddress::P2pkh([10; 20]);

        // Store 3 blocks of changes at heights 100, 200, 300
        for block_height in [100u64, 200, 300] {
            let mut changes = BTreeMap::new();
            changes.insert(address, CreditOperation::AddToCredits(block_height * 1000));
            drive
                .store_address_balances_for_block(
                    &changes,
                    block_height,
                    block_height * 1000,
                    None,
                    platform_version,
                )
                .expect("should store balances");
        }

        // Prove after height 100 (exclusive)
        let proof = drive
            .prove_recent_address_balance_changes_after(100, None, None, platform_version)
            .expect("should prove address balance changes after height 100");

        assert!(!proof.is_empty(), "proof should be non-empty bytes");
    }

    #[test]
    fn should_fetch_after_returns_empty_when_no_later_blocks() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address = PlatformAddress::P2pkh([10; 20]);

        // Store a single block at height 300
        let mut changes = BTreeMap::new();
        changes.insert(address, CreditOperation::SetCredits(1000));
        drive
            .store_address_balances_for_block(&changes, 300, 300_000, None, platform_version)
            .expect("should store balances");

        // Fetch after height 300 (exclusive) - nothing should come back
        let result = drive
            .fetch_recent_address_balance_changes_after(300, None, None, platform_version)
            .expect("should fetch with no later blocks");

        assert!(
            result.is_empty(),
            "should have no changes after the max stored height"
        );
    }
}
