/// This module dedicated for a versioned add_update_next_withdrawal_transaction_index_operation
pub mod add_update_next_withdrawal_transaction_index_operation;
/// This module dedicated for a versioned fetch_next_withdrawal_transaction_index
pub mod fetch_next_withdrawal_transaction_index;

#[cfg(test)]
mod tests {
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;

    use dpp::version::PlatformVersion;

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    #[test]
    fn test_next_withdrawal_transaction_index() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let transaction = drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 1,
            epoch: Epoch::new(1).unwrap(),
        };

        let mut batch = vec![];

        let counter: u64 = 42;

        drive
            .add_update_next_withdrawal_transaction_index_operation(
                counter,
                &mut batch,
                platform_version,
            )
            .expect("to add update next withdrawal transaction index operation");

        drive
            .apply_drive_operations(
                batch,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("to apply drive ops");

        let stored_counter = drive
            .fetch_next_withdrawal_transaction_index(Some(&transaction), platform_version)
            .expect("to withdraw counter");

        assert_eq!(stored_counter, counter);
    }

    #[test]
    fn test_initial_withdrawal_transaction_index() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::latest();

        let transaction = drive.grove.start_transaction();

        let stored_counter = drive
            .fetch_next_withdrawal_transaction_index(Some(&transaction), platform_version)
            .expect("to withdraw counter");

        assert_eq!(stored_counter, 0);
    }
}
