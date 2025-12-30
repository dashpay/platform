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
    use platform_version::version::mocks::v2_test::TEST_PLATFORM_V2;
    use platform_version::version::PlatformVersion;

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
            .store_address_balances_for_block_v0(&balances_block_1, 100, None, &platform_version)
            .expect("should store block 1");
        drive
            .store_address_balances_for_block_v0(&balances_block_2, 101, None, &platform_version)
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
            .store_address_balances_for_block_v0(&balances_block_3, 102, None, &platform_version)
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
            .store_address_balances_for_block_v0(&balances_block_1, 100, None, &platform_version)
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
            .store_address_balances_for_block_v0(&balances_block_2, 101, None, &platform_version)
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
            .store_address_balances_for_block_v0(&balances_block_1, 100, None, &platform_version)
            .expect("should store block 1");

        // Block 2: Add more credits to same address - should trigger compaction and merge
        let mut balances_block_2 = BTreeMap::new();
        balances_block_2.insert(ADDR_1, CreditOperation::AddToCredits(500));

        drive
            .store_address_balances_for_block_v0(&balances_block_2, 101, None, &platform_version)
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
            .store_address_balances_for_block_v0(&balances_block_1, 100, None, &platform_version)
            .expect("should store block 1");

        // Block 2: Add credits to same address - should trigger compaction and merge
        let mut balances_block_2 = BTreeMap::new();
        balances_block_2.insert(ADDR_1, CreditOperation::AddToCredits(500));

        drive
            .store_address_balances_for_block_v0(&balances_block_2, 101, None, &platform_version)
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
            .store_address_balances_for_block_v0(&balances_block_1, 100, None, &platform_version)
            .expect("should store block 1");

        // Block 2: Set credits to same address - should override
        let mut balances_block_2 = BTreeMap::new();
        balances_block_2.insert(ADDR_1, CreditOperation::SetCredits(500));

        drive
            .store_address_balances_for_block_v0(&balances_block_2, 101, None, &platform_version)
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
            .store_address_balances_for_block_v0(&empty_balances, 100, None, &platform_version)
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
            .store_address_balances_for_block_v0(&balances_100, 100, None, &platform_version)
            .expect("should store block 100");
        drive
            .store_address_balances_for_block_v0(&balances_101, 101, None, &platform_version)
            .expect("should store and compact block 101");

        // Second cycle: blocks 200, 201
        let mut balances_200 = BTreeMap::new();
        balances_200.insert(ADDR_2, CreditOperation::AddToCredits(200));
        let mut balances_201 = BTreeMap::new();
        balances_201.insert(ADDR_2, CreditOperation::AddToCredits(200));

        drive
            .store_address_balances_for_block_v0(&balances_200, 200, None, &platform_version)
            .expect("should store block 200");
        drive
            .store_address_balances_for_block_v0(&balances_201, 201, None, &platform_version)
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
}
