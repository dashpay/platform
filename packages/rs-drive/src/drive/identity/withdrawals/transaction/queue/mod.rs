/// This module is dedicated for a versioned add_enqueue_untied_withdrawal_transaction_operations
pub mod add_enqueue_untied_withdrawal_transaction_operations;
/// This module is dedicated for a versioned dequeue_untied_withdrawal_transactions
pub mod dequeue_untied_withdrawal_transactions;

/// This module is dedicated to removing broadcasted withdrawal transactions after core says they were completed
pub mod remove_broadcasted_withdrawal_transactions_after_completion;

/// This module is dedicated to moving broadcasted withdrawal transactions back to the main queue
pub mod move_broadcasted_withdrawal_transactions_back_to_queue_operations;

#[cfg(test)]
mod tests {
    use crate::util::batch::DriveOperation;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::withdrawal::{WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes};
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_enqueue_and_dequeue() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);

        let transaction = drive.grove.start_transaction();

        let withdrawals: Vec<WithdrawalTransactionIndexAndBytes> = (0..17)
            .map(|i: u8| (i as WithdrawalTransactionIndex, vec![i; 32]))
            .collect();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 1,
            epoch: Epoch::new(1).unwrap(),
        };

        let mut drive_operations: Vec<DriveOperation> = vec![];

        drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                withdrawals,
                100,
                &mut drive_operations,
                platform_version,
            )
            .expect("to add enqueue untied withdrawal transaction operations");

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("to apply batch");

        let mut drive_operations: Vec<DriveOperation> = vec![];

        let withdrawals = drive
            .dequeue_untied_withdrawal_transactions(
                16,
                Some(&transaction),
                &mut drive_operations,
                platform_version,
            )
            .expect("to dequeue withdrawals");

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("to apply batch");

        assert_eq!(withdrawals.len(), 16);

        let mut drive_operations: Vec<DriveOperation> = vec![];

        let withdrawals = drive
            .dequeue_untied_withdrawal_transactions(
                16,
                Some(&transaction),
                &mut drive_operations,
                platform_version,
            )
            .expect("to dequeue withdrawals");

        drive
            .apply_drive_operations(
                drive_operations,
                true,
                &block_info,
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("to apply batch");

        assert_eq!(withdrawals.len(), 1);

        let mut drive_operations: Vec<DriveOperation> = vec![];

        drive
            .dequeue_untied_withdrawal_transactions(
                16,
                Some(&transaction),
                &mut drive_operations,
                platform_version,
            )
            .expect("to dequeue withdrawals");

        assert_eq!(drive_operations.len(), 0);
    }
}
