use crate::drive::saved_block_transactions::ADDRESS_BALANCES_KEY_U8;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::ProtocolError;
use grovedb::Element;
use grovedb::TransactionArg;
use grovedb_path::SubtreePath;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Version 0 implementation of storing address balance changes for a block.
    ///
    /// Serializes the address balance map using bincode and stores it in the
    /// SavedBlockTransactions/AddressBalances count sum tree keyed by block height.
    /// Each entry is an ItemWithSumItem where:
    /// - The item contains the serialized address balance changes
    /// - The sum value is the number of address balance entries
    ///
    /// Before storing, checks if compaction thresholds are exceeded and triggers
    /// compaction if necessary. If compaction occurs, the current block's addresses
    /// are included in the compaction rather than stored separately.
    pub(super) fn store_address_balances_for_block_v0(
        &self,
        address_balances: &BTreeMap<PlatformAddress, CreditOperation>,
        block_height: u64,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Early return if there are no balance changes to store
        if address_balances.is_empty() {
            return Ok(());
        }

        // Check if compaction is needed - if so, include current addresses in compaction
        let compacted = self.check_and_compact_if_needed(
            address_balances,
            block_height,
            block_time_ms,
            transaction,
            platform_version,
        )?;

        // If we compacted, the current addresses are already included - don't store separately
        if compacted {
            return Ok(());
        }

        // Serialize the address balances map using bincode
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let serialized = bincode::encode_to_vec(address_balances, config).map_err(|e| {
            Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                "cannot encode address balances: {}",
                e
            ))))
        })?;

        // The sum value is the number of address balance entries
        let entry_count = address_balances.len() as i64;

        // Store in the SavedBlockTransactions/AddressBalances count sum tree with block height as key
        let path: [&[u8]; 2] = Drive::saved_block_transactions_address_balances_path();

        // Use block height as the key (big-endian for proper ordering)
        let key = block_height.to_be_bytes();

        // Insert as ItemWithSumItem where:
        // - item data = serialized address balance changes
        // - sum value = number of address balance entries
        let mut drive_operations = vec![];
        self.grove_insert(
            path.as_ref().into(),
            &key,
            Element::new_item_with_sum_item(serialized, entry_count),
            transaction,
            None,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // Apply any operations that were generated
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            drive_operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(())
    }

    /// Checks if compaction thresholds are exceeded and triggers compaction if needed.
    /// If compaction occurs, the provided address_balances are included in the compaction.
    ///
    /// Returns true if compaction was performed (meaning the addresses were included).
    fn check_and_compact_if_needed(
        &self,
        address_balances: &BTreeMap<PlatformAddress, CreditOperation>,
        block_height: u64,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let saved_block_tx_path = Self::saved_block_transactions_path();

        // Get the count sum tree element to check current count and sum
        let mut drive_operations = vec![];
        let tree_element = self.grove_get_raw(
            SubtreePath::from(saved_block_tx_path.as_slice()),
            &[ADDRESS_BALANCES_KEY_U8],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        if let Some(Element::CountSumTree(_, count, sum, _)) = tree_element {
            let max_blocks = platform_version
                .drive
                .methods
                .saved_block_transactions
                .max_blocks_before_compaction as u64;
            let max_addresses = platform_version
                .drive
                .methods
                .saved_block_transactions
                .max_addresses_before_compaction as i64;

            // Check if either threshold would be exceeded after adding the current block
            // count + 1 for the new block, sum + current addresses count
            let new_count = count + 1;
            let new_sum = sum + address_balances.len() as i64;

            if new_count >= max_blocks || new_sum >= max_addresses {
                // Trigger compaction, including the current block's addresses
                self.compact_address_balances_with_current_block(
                    address_balances,
                    block_height,
                    block_time_ms,
                    transaction,
                    platform_version,
                )?;
                return Ok(true);
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::mocks::v2_test::TEST_PLATFORM_V2;
    use dpp::version::PlatformVersion;

    // Test addresses
    const ADDR_1: PlatformAddress = PlatformAddress::P2pkh([1; 20]);
    const ADDR_2: PlatformAddress = PlatformAddress::P2pkh([2; 20]);
    const ADDR_3: PlatformAddress = PlatformAddress::P2pkh([3; 20]);
    const ADDR_4: PlatformAddress = PlatformAddress::P2pkh([4; 20]);

    fn create_test_platform_version_for_compaction(
        max_blocks: u16,
        max_addresses: u32,
    ) -> PlatformVersion {
        let mut version = TEST_PLATFORM_V2.clone();
        version
            .drive
            .methods
            .saved_block_transactions
            .max_blocks_before_compaction = max_blocks;
        version
            .drive
            .methods
            .saved_block_transactions
            .max_addresses_before_compaction = max_addresses;
        version
    }

    // Test block time constant (arbitrary value for testing)
    const TEST_BLOCK_TIME_MS: u64 = 1700000000000; // Some timestamp in ms

    #[test]
    fn should_compact_when_max_blocks_threshold_exceeded() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Low threshold: compact after 3 blocks
        let platform_version = create_test_platform_version_for_compaction(3, 1000);

        // Store balance changes for 3 blocks (should not compact yet, threshold is >=)
        let mut balances_block_1 = BTreeMap::new();
        balances_block_1.insert(ADDR_1, CreditOperation::AddToCredits(1000));

        let mut balances_block_2 = BTreeMap::new();
        balances_block_2.insert(ADDR_2, CreditOperation::AddToCredits(2000));

        drive
            .store_address_balances_for_block_v0(
                &balances_block_1,
                100,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 1");
        drive
            .store_address_balances_for_block_v0(
                &balances_block_2,
                101,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 2");

        // Verify blocks are stored (not compacted yet)
        let recent = drive
            .fetch_recent_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch recent changes");
        assert_eq!(recent.len(), 2, "should have 2 blocks before compaction");

        // Store block 3 - this should trigger compaction (count >= 3)
        let mut balances_block_3 = BTreeMap::new();
        balances_block_3.insert(ADDR_3, CreditOperation::AddToCredits(3000));

        drive
            .store_address_balances_for_block_v0(
                &balances_block_3,
                102,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store and compact block 3");

        // After compaction, recent address balances should be empty (all moved to compacted)
        let recent_after = drive
            .fetch_recent_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch recent changes");
        assert!(
            recent_after.is_empty(),
            "should have no recent blocks after compaction"
        );

        // Verify compacted data exists and contains all 3 addresses
        let compacted = drive
            .fetch_compacted_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch compacted changes");
        assert_eq!(compacted.len(), 1, "should have 1 compacted entry");

        let (start_block, end_block, merged) = &compacted[0];
        assert_eq!(*start_block, 100, "start block should be 100");
        assert_eq!(*end_block, 102, "end block should be 102");
        assert_eq!(merged.len(), 3, "should have 3 addresses in merged data");
        assert_eq!(
            merged.get(&ADDR_1),
            Some(&CreditOperation::AddToCredits(1000))
        );
        assert_eq!(
            merged.get(&ADDR_2),
            Some(&CreditOperation::AddToCredits(2000))
        );
        assert_eq!(
            merged.get(&ADDR_3),
            Some(&CreditOperation::AddToCredits(3000))
        );
    }

    #[test]
    fn should_compact_when_max_addresses_threshold_exceeded() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Low threshold: compact after 4 total address entries
        let platform_version = create_test_platform_version_for_compaction(100, 4);

        // Store block with 2 addresses
        let mut balances_block_1 = BTreeMap::new();
        balances_block_1.insert(ADDR_1, CreditOperation::AddToCredits(1000));
        balances_block_1.insert(ADDR_2, CreditOperation::AddToCredits(2000));

        drive
            .store_address_balances_for_block_v0(
                &balances_block_1,
                100,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 1");

        // Verify block is stored (not compacted yet - only 2 addresses)
        let recent = drive
            .fetch_recent_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch recent changes");
        assert_eq!(recent.len(), 1, "should have 1 block before compaction");

        // Store block with 2 more addresses - total will be 4, triggering compaction
        let mut balances_block_2 = BTreeMap::new();
        balances_block_2.insert(ADDR_3, CreditOperation::AddToCredits(3000));
        balances_block_2.insert(ADDR_4, CreditOperation::AddToCredits(4000));

        drive
            .store_address_balances_for_block_v0(
                &balances_block_2,
                101,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store and compact block 2");

        // After compaction, recent address balances should be empty
        let recent_after = drive
            .fetch_recent_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch recent changes");
        assert!(
            recent_after.is_empty(),
            "should have no recent blocks after compaction"
        );

        // Verify compacted data exists with all 4 addresses
        let compacted = drive
            .fetch_compacted_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch compacted changes");
        assert_eq!(compacted.len(), 1, "should have 1 compacted entry");

        let (start_block, end_block, merged) = &compacted[0];
        assert_eq!(*start_block, 100, "start block should be 100");
        assert_eq!(*end_block, 101, "end block should be 101");
        assert_eq!(merged.len(), 4, "should have 4 addresses in merged data");
    }

    #[test]
    fn should_merge_add_to_credits_operations_during_compaction() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Low threshold to trigger compaction after 2 blocks
        let platform_version = create_test_platform_version_for_compaction(2, 1000);

        // Block 1: Add credits to ADDR_1
        let mut balances_block_1 = BTreeMap::new();
        balances_block_1.insert(ADDR_1, CreditOperation::AddToCredits(1000));

        drive
            .store_address_balances_for_block_v0(
                &balances_block_1,
                100,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 1");

        // Block 2: Add more credits to same address - should trigger compaction and merge
        let mut balances_block_2 = BTreeMap::new();
        balances_block_2.insert(ADDR_1, CreditOperation::AddToCredits(500));

        drive
            .store_address_balances_for_block_v0(
                &balances_block_2,
                101,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store and compact block 2");

        // Verify compacted data has merged credits
        let compacted = drive
            .fetch_compacted_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch compacted changes");
        assert_eq!(compacted.len(), 1, "should have 1 compacted entry");

        let (_, _, merged) = &compacted[0];
        assert_eq!(merged.len(), 1, "should have 1 address");
        // 1000 + 500 = 1500
        assert_eq!(
            merged.get(&ADDR_1),
            Some(&CreditOperation::AddToCredits(1500)),
            "should merge add operations"
        );
    }

    #[test]
    fn should_merge_set_credits_with_add_to_credits_during_compaction() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Low threshold to trigger compaction after 2 blocks
        let platform_version = create_test_platform_version_for_compaction(2, 1000);

        // Block 1: Set credits to ADDR_1
        let mut balances_block_1 = BTreeMap::new();
        balances_block_1.insert(ADDR_1, CreditOperation::SetCredits(1000));

        drive
            .store_address_balances_for_block_v0(
                &balances_block_1,
                100,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 1");

        // Block 2: Add credits to same address - should trigger compaction and merge
        let mut balances_block_2 = BTreeMap::new();
        balances_block_2.insert(ADDR_1, CreditOperation::AddToCredits(500));

        drive
            .store_address_balances_for_block_v0(
                &balances_block_2,
                101,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store and compact block 2");

        // Verify compacted data has merged: SetCredits(1000) + AddToCredits(500) = SetCredits(1500)
        let compacted = drive
            .fetch_compacted_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch compacted changes");
        assert_eq!(compacted.len(), 1, "should have 1 compacted entry");

        let (_, _, merged) = &compacted[0];
        assert_eq!(merged.len(), 1, "should have 1 address");
        assert_eq!(
            merged.get(&ADDR_1),
            Some(&CreditOperation::SetCredits(1500)),
            "should merge set + add to set"
        );
    }

    #[test]
    fn should_override_with_later_set_credits_during_compaction() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Low threshold to trigger compaction after 2 blocks
        let platform_version = create_test_platform_version_for_compaction(2, 1000);

        // Block 1: Add credits to ADDR_1
        let mut balances_block_1 = BTreeMap::new();
        balances_block_1.insert(ADDR_1, CreditOperation::AddToCredits(1000));

        drive
            .store_address_balances_for_block_v0(
                &balances_block_1,
                100,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 1");

        // Block 2: Set credits to same address - should override
        let mut balances_block_2 = BTreeMap::new();
        balances_block_2.insert(ADDR_1, CreditOperation::SetCredits(500));

        drive
            .store_address_balances_for_block_v0(
                &balances_block_2,
                101,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store and compact block 2");

        // Verify compacted data has the set value (overrides previous add)
        let compacted = drive
            .fetch_compacted_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch compacted changes");
        assert_eq!(compacted.len(), 1, "should have 1 compacted entry");

        let (_, _, merged) = &compacted[0];
        assert_eq!(merged.len(), 1, "should have 1 address");
        assert_eq!(
            merged.get(&ADDR_1),
            Some(&CreditOperation::SetCredits(500)),
            "later SetCredits should override"
        );
    }

    #[test]
    fn should_not_store_empty_balances() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = create_test_platform_version_for_compaction(100, 1000);

        // Try to store empty balances
        let empty_balances: BTreeMap<PlatformAddress, CreditOperation> = BTreeMap::new();
        drive
            .store_address_balances_for_block_v0(
                &empty_balances,
                100,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should handle empty balances");

        // Verify nothing was stored
        let recent = drive
            .fetch_recent_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch recent changes");
        assert!(recent.is_empty(), "should have no blocks stored");
    }

    #[test]
    fn should_handle_multiple_compaction_cycles() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Compact after every 2 blocks
        let platform_version = create_test_platform_version_for_compaction(2, 1000);

        // First cycle: blocks 100, 101
        let mut balances_100 = BTreeMap::new();
        balances_100.insert(ADDR_1, CreditOperation::AddToCredits(100));
        let mut balances_101 = BTreeMap::new();
        balances_101.insert(ADDR_1, CreditOperation::AddToCredits(100));

        drive
            .store_address_balances_for_block_v0(
                &balances_100,
                100,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 100");
        drive
            .store_address_balances_for_block_v0(
                &balances_101,
                101,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store and compact block 101");

        // Second cycle: blocks 200, 201
        let mut balances_200 = BTreeMap::new();
        balances_200.insert(ADDR_2, CreditOperation::AddToCredits(200));
        let mut balances_201 = BTreeMap::new();
        balances_201.insert(ADDR_2, CreditOperation::AddToCredits(200));

        drive
            .store_address_balances_for_block_v0(
                &balances_200,
                200,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 200");
        drive
            .store_address_balances_for_block_v0(
                &balances_201,
                201,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store and compact block 201");

        // Verify we have 2 compacted entries
        let compacted = drive
            .fetch_compacted_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch compacted changes");
        assert_eq!(compacted.len(), 2, "should have 2 compacted entries");

        // First compaction: blocks 100-101 with ADDR_1 having 200 credits
        let (start1, end1, merged1) = &compacted[0];
        assert_eq!(*start1, 100);
        assert_eq!(*end1, 101);
        assert_eq!(
            merged1.get(&ADDR_1),
            Some(&CreditOperation::AddToCredits(200))
        );

        // Second compaction: blocks 200-201 with ADDR_2 having 400 credits
        let (start2, end2, merged2) = &compacted[1];
        assert_eq!(*start2, 200);
        assert_eq!(*end2, 201);
        assert_eq!(
            merged2.get(&ADDR_2),
            Some(&CreditOperation::AddToCredits(400))
        );
    }

    #[test]
    fn should_query_compacted_data_with_start_height_filter() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Compact after every 2 blocks
        let platform_version = create_test_platform_version_for_compaction(2, 1000);

        // Create 3 compaction cycles: 100-101, 200-201, 300-301
        for (base_block, addr) in [(100, ADDR_1), (200, ADDR_2), (300, ADDR_3)] {
            let mut balances_1 = BTreeMap::new();
            balances_1.insert(addr, CreditOperation::AddToCredits(100));
            let mut balances_2 = BTreeMap::new();
            balances_2.insert(addr, CreditOperation::AddToCredits(100));

            drive
                .store_address_balances_for_block_v0(
                    &balances_1,
                    base_block,
                    TEST_BLOCK_TIME_MS,
                    None,
                    &platform_version,
                )
                .expect("should store");
            drive
                .store_address_balances_for_block_v0(
                    &balances_2,
                    base_block + 1,
                    TEST_BLOCK_TIME_MS,
                    None,
                    &platform_version,
                )
                .expect("should store and compact");
        }

        // Query all compacted data from height 0
        let all_compacted = drive
            .fetch_compacted_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch");
        assert_eq!(all_compacted.len(), 3, "should have 3 compacted entries");

        // Query compacted data starting from height 150 (should skip first compaction)
        let from_150 = drive
            .fetch_compacted_address_balance_changes(150, None, None, &platform_version)
            .expect("should fetch");
        assert_eq!(
            from_150.len(),
            2,
            "should have 2 compacted entries when starting from 150"
        );
        assert_eq!(from_150[0].0, 200, "first result should start at 200");
        assert_eq!(from_150[1].0, 300, "second result should start at 300");

        // Query compacted data starting from height 250 (should skip first two)
        let from_250 = drive
            .fetch_compacted_address_balance_changes(250, None, None, &platform_version)
            .expect("should fetch");
        assert_eq!(
            from_250.len(),
            1,
            "should have 1 compacted entry when starting from 250"
        );
        assert_eq!(from_250[0].0, 300, "result should start at 300");

        // Query from height 400 (should return empty)
        let from_400 = drive
            .fetch_compacted_address_balance_changes(400, None, None, &platform_version)
            .expect("should fetch");
        assert!(
            from_400.is_empty(),
            "should have no results when starting after all compactions"
        );
    }

    #[test]
    fn should_query_compacted_data_with_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Compact after every 2 blocks
        let platform_version = create_test_platform_version_for_compaction(2, 1000);

        // Create 4 compaction cycles
        for (base_block, addr) in [(100, ADDR_1), (200, ADDR_2), (300, ADDR_3), (400, ADDR_4)] {
            let mut balances_1 = BTreeMap::new();
            balances_1.insert(addr, CreditOperation::AddToCredits(100));
            let mut balances_2 = BTreeMap::new();
            balances_2.insert(addr, CreditOperation::AddToCredits(100));

            drive
                .store_address_balances_for_block_v0(
                    &balances_1,
                    base_block,
                    TEST_BLOCK_TIME_MS,
                    None,
                    &platform_version,
                )
                .expect("should store");
            drive
                .store_address_balances_for_block_v0(
                    &balances_2,
                    base_block + 1,
                    TEST_BLOCK_TIME_MS,
                    None,
                    &platform_version,
                )
                .expect("should store and compact");
        }

        // Query with limit of 2
        let limited = drive
            .fetch_compacted_address_balance_changes(0, Some(2), None, &platform_version)
            .expect("should fetch");
        assert_eq!(limited.len(), 2, "should return only 2 entries with limit");
        assert_eq!(limited[0].0, 100, "first should be 100-101");
        assert_eq!(limited[1].0, 200, "second should be 200-201");

        // Query with limit of 1
        let limited_1 = drive
            .fetch_compacted_address_balance_changes(0, Some(1), None, &platform_version)
            .expect("should fetch");
        assert_eq!(limited_1.len(), 1, "should return only 1 entry");

        // Query from height 200 with limit of 2
        let from_200_limit_2 = drive
            .fetch_compacted_address_balance_changes(200, Some(2), None, &platform_version)
            .expect("should fetch");
        assert_eq!(from_200_limit_2.len(), 2, "should return 2 entries");
        assert_eq!(from_200_limit_2[0].0, 200, "first should be 200-201");
        assert_eq!(from_200_limit_2[1].0, 300, "second should be 300-301");
    }

    #[test]
    fn should_have_both_recent_and_compacted_data() {
        let drive = setup_drive_with_initial_state_structure(None);
        // Compact after every 3 blocks
        let platform_version = create_test_platform_version_for_compaction(3, 1000);

        // Store 3 blocks to trigger compaction (100, 101, 102)
        for block in 100..=102 {
            let mut balances = BTreeMap::new();
            balances.insert(ADDR_1, CreditOperation::AddToCredits(100));
            drive
                .store_address_balances_for_block_v0(
                    &balances,
                    block,
                    TEST_BLOCK_TIME_MS,
                    None,
                    &platform_version,
                )
                .expect("should store");
        }

        // Verify compaction happened
        let compacted = drive
            .fetch_compacted_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch compacted");
        assert_eq!(compacted.len(), 1, "should have 1 compacted entry");
        assert_eq!(compacted[0].0, 100, "start block should be 100");
        assert_eq!(compacted[0].1, 102, "end block should be 102");

        // Now store 1 more block (not enough to trigger another compaction)
        let mut balances_103 = BTreeMap::new();
        balances_103.insert(ADDR_2, CreditOperation::AddToCredits(500));
        drive
            .store_address_balances_for_block_v0(
                &balances_103,
                103,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 103");

        // Verify we have both compacted AND recent data
        let compacted_after = drive
            .fetch_compacted_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch compacted");
        assert_eq!(
            compacted_after.len(),
            1,
            "should still have 1 compacted entry"
        );

        let recent = drive
            .fetch_recent_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch recent");
        assert_eq!(recent.len(), 1, "should have 1 recent block");
        assert_eq!(recent[0].0, 103, "recent block should be 103");
        assert_eq!(
            recent[0].1.get(&ADDR_2),
            Some(&CreditOperation::AddToCredits(500))
        );

        // Store another block - still not enough for compaction
        let mut balances_104 = BTreeMap::new();
        balances_104.insert(ADDR_3, CreditOperation::AddToCredits(600));
        drive
            .store_address_balances_for_block_v0(
                &balances_104,
                104,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 104");

        let recent_2 = drive
            .fetch_recent_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch recent");
        assert_eq!(recent_2.len(), 2, "should have 2 recent blocks");

        // Store block 105 - should trigger second compaction
        let mut balances_105 = BTreeMap::new();
        balances_105.insert(ADDR_4, CreditOperation::AddToCredits(700));
        drive
            .store_address_balances_for_block_v0(
                &balances_105,
                105,
                TEST_BLOCK_TIME_MS,
                None,
                &platform_version,
            )
            .expect("should store block 105 and trigger compaction");

        // Verify we now have 2 compacted entries and no recent
        let final_compacted = drive
            .fetch_compacted_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch compacted");
        assert_eq!(final_compacted.len(), 2, "should have 2 compacted entries");
        assert_eq!(final_compacted[0].0, 100, "first compaction starts at 100");
        assert_eq!(final_compacted[0].1, 102, "first compaction ends at 102");
        assert_eq!(final_compacted[1].0, 103, "second compaction starts at 103");
        assert_eq!(final_compacted[1].1, 105, "second compaction ends at 105");

        let final_recent = drive
            .fetch_recent_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch recent");
        assert!(
            final_recent.is_empty(),
            "should have no recent blocks after second compaction"
        );
    }

    #[test]
    fn should_query_recent_data_with_start_height_filter() {
        let drive = setup_drive_with_initial_state_structure(None);
        // High threshold so no compaction happens
        let platform_version = create_test_platform_version_for_compaction(100, 10000);

        // Store blocks 100, 101, 102, 103
        for block in 100..=103 {
            let mut balances = BTreeMap::new();
            balances.insert(ADDR_1, CreditOperation::AddToCredits(block));
            drive
                .store_address_balances_for_block_v0(
                    &balances,
                    block,
                    TEST_BLOCK_TIME_MS,
                    None,
                    &platform_version,
                )
                .expect("should store");
        }

        // Query all
        let all = drive
            .fetch_recent_address_balance_changes(0, None, None, &platform_version)
            .expect("should fetch");
        assert_eq!(all.len(), 4, "should have 4 blocks");

        // Query from height 101
        let from_101 = drive
            .fetch_recent_address_balance_changes(101, None, None, &platform_version)
            .expect("should fetch");
        assert_eq!(from_101.len(), 3, "should have 3 blocks from 101");
        assert_eq!(from_101[0].0, 101, "first should be block 101");

        // Query from height 103
        let from_103 = drive
            .fetch_recent_address_balance_changes(103, None, None, &platform_version)
            .expect("should fetch");
        assert_eq!(from_103.len(), 1, "should have 1 block from 103");

        // Query from height 200 (after all blocks)
        let from_200 = drive
            .fetch_recent_address_balance_changes(200, None, None, &platform_version)
            .expect("should fetch");
        assert!(from_200.is_empty(), "should have no blocks from 200");
    }

    #[test]
    fn should_store_expiration_time_when_compacting() {
        use crate::drive::saved_block_transactions::compact_address_balances::ONE_WEEK_IN_MS;
        use crate::drive::saved_block_transactions::COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8;
        use grovedb::query_result_type::QueryResultType;
        use grovedb::{PathQuery, Query, SizedQuery};

        let drive = setup_drive_with_initial_state_structure(None);
        // Compact after 2 blocks
        let platform_version = create_test_platform_version_for_compaction(2, 1000);

        // Store 2 blocks to trigger compaction
        let mut balances_1 = BTreeMap::new();
        balances_1.insert(ADDR_1, CreditOperation::AddToCredits(1000));

        let mut balances_2 = BTreeMap::new();
        balances_2.insert(ADDR_2, CreditOperation::AddToCredits(2000));

        let block_time_ms: u64 = 1700000000000; // Some timestamp

        drive
            .store_address_balances_for_block_v0(
                &balances_1,
                100,
                block_time_ms,
                None,
                &platform_version,
            )
            .expect("should store block 1");
        drive
            .store_address_balances_for_block_v0(
                &balances_2,
                101,
                block_time_ms,
                None,
                &platform_version,
            )
            .expect("should store and compact block 2");

        // Query the expiration time tree directly to verify it was stored
        let path = vec![
            vec![crate::drive::RootTree::SavedBlockTransactions as u8],
            vec![COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8],
        ];

        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(path, SizedQuery::new(query, None, None));

        let (results, _) = drive
            .grove_get_path_query(
                &path_query,
                None,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("should query expiration tree");

        let key_elements = results.to_key_elements();
        assert_eq!(key_elements.len(), 1, "should have 1 expiration entry");

        // Verify the key is the expiration time (block_time + 1 week)
        let (key, element) = &key_elements[0];
        assert_eq!(key.len(), 8, "key should be 8 bytes (u64 expiration time)");

        let expiration_time = u64::from_be_bytes(key.as_slice().try_into().unwrap());
        let expected_expiration = block_time_ms + ONE_WEEK_IN_MS;
        assert_eq!(
            expiration_time, expected_expiration,
            "key should be expiration time (block_time + 1 week)"
        );

        // Verify the value is a vec of block ranges
        if let grovedb::Element::Item(serialized_ranges, _) = element {
            let config = bincode::config::standard()
                .with_big_endian()
                .with_no_limit();
            let (ranges, _): (Vec<(u64, u64)>, usize) =
                bincode::decode_from_slice(serialized_ranges, config)
                    .expect("should decode block ranges");

            assert_eq!(ranges.len(), 1, "should have 1 block range");
            assert_eq!(ranges[0], (100, 101), "block range should be (100, 101)");
        } else {
            panic!("expected Item element for block ranges");
        }
    }

    #[test]
    fn should_append_to_expiration_time_when_same_time() {
        use crate::drive::saved_block_transactions::compact_address_balances::ONE_WEEK_IN_MS;
        use crate::drive::saved_block_transactions::COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8;
        use grovedb::query_result_type::QueryResultType;
        use grovedb::{PathQuery, Query, SizedQuery};

        let drive = setup_drive_with_initial_state_structure(None);
        // Compact after 2 blocks
        let platform_version = create_test_platform_version_for_compaction(2, 1000);

        let block_time_ms: u64 = 1700000000000; // Same timestamp for both compactions

        // First compaction cycle: blocks 100-101
        let mut balances_1 = BTreeMap::new();
        balances_1.insert(ADDR_1, CreditOperation::AddToCredits(1000));
        let mut balances_2 = BTreeMap::new();
        balances_2.insert(ADDR_1, CreditOperation::AddToCredits(500));

        drive
            .store_address_balances_for_block_v0(
                &balances_1,
                100,
                block_time_ms,
                None,
                &platform_version,
            )
            .expect("should store block 100");
        drive
            .store_address_balances_for_block_v0(
                &balances_2,
                101,
                block_time_ms,
                None,
                &platform_version,
            )
            .expect("should store and compact block 101");

        // Second compaction cycle: blocks 200-201 (same block time - unlikely but possible)
        let mut balances_3 = BTreeMap::new();
        balances_3.insert(ADDR_2, CreditOperation::AddToCredits(2000));
        let mut balances_4 = BTreeMap::new();
        balances_4.insert(ADDR_2, CreditOperation::AddToCredits(500));

        drive
            .store_address_balances_for_block_v0(
                &balances_3,
                200,
                block_time_ms,
                None,
                &platform_version,
            )
            .expect("should store block 200");
        drive
            .store_address_balances_for_block_v0(
                &balances_4,
                201,
                block_time_ms,
                None,
                &platform_version,
            )
            .expect("should store and compact block 201");

        // Query the expiration time tree
        let path = vec![
            vec![crate::drive::RootTree::SavedBlockTransactions as u8],
            vec![COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8],
        ];

        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(path, SizedQuery::new(query, None, None));

        let (results, _) = drive
            .grove_get_path_query(
                &path_query,
                None,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("should query expiration tree");

        let key_elements = results.to_key_elements();
        // Should still have only 1 entry since both compactions have the same expiration time
        assert_eq!(
            key_elements.len(),
            1,
            "should have 1 expiration entry (same time)"
        );

        // Verify the key is the expiration time
        let (key, element) = &key_elements[0];
        let expiration_time = u64::from_be_bytes(key.as_slice().try_into().unwrap());
        let expected_expiration = block_time_ms + ONE_WEEK_IN_MS;
        assert_eq!(expiration_time, expected_expiration);

        // Verify the value contains BOTH block ranges
        if let grovedb::Element::Item(serialized_ranges, _) = element {
            let config = bincode::config::standard()
                .with_big_endian()
                .with_no_limit();
            let (ranges, _): (Vec<(u64, u64)>, usize) =
                bincode::decode_from_slice(serialized_ranges, config)
                    .expect("should decode block ranges");

            assert_eq!(ranges.len(), 2, "should have 2 block ranges");
            assert_eq!(
                ranges[0],
                (100, 101),
                "first block range should be (100, 101)"
            );
            assert_eq!(
                ranges[1],
                (200, 201),
                "second block range should be (200, 201)"
            );
        } else {
            panic!("expected Item element for block ranges");
        }
    }
}
