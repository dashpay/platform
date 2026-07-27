use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_voting_path;
use dpp::version::PlatformVersion;
use grovedb::Element::SumItem;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    /// Fetches the Prefunded specialized balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_prefunded_specialized_balance_v0(
        &self,
        balance_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_prefunded_specialized_balance_operations_v0(
            balance_id,
            true,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Fetches the Prefunded specialized balance from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_prefunded_specialized_balance_with_costs_v0(
        &self,
        balance_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<Credits>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_prefunded_specialized_balance_operations_v0(
            balance_id,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok((value, fees))
    }

    /// Creates the operations to get Prefunded specialized balance from the backing store
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(super) fn fetch_prefunded_specialized_balance_operations_v0(
        &self,
        balance_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            // 8 is the size of a i64 used in sum trees
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(8),
            }
        };

        let balance_path = prefunded_specialized_balances_for_voting_path();

        match self.grove_get_raw_optional(
            (&balance_path).into(),
            balance_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(SumItem(balance, _))) if balance >= 0 => Ok(Some(balance as Credits)),

            Ok(None) => {
                if apply {
                    Ok(None)
                } else {
                    Ok(Some(0))
                }
            }
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_)) => {
                if apply {
                    Ok(None)
                } else {
                    Ok(Some(0))
                }
            }

            Ok(Some(SumItem(..))) => Err(Error::Drive(DriveError::CorruptedElementType(
                "specialized balance was present but was negative",
            ))),

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "specialized balance was present but was not identified as a sum item",
            ))),

            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_voting_path_vec;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;
    use grovedb::batch::QualifiedGroveDbOp;
    use grovedb::Element;

    #[test]
    fn fetch_missing_apply_true_returns_none() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let balance = drive
            .fetch_prefunded_specialized_balance_v0([0u8; 32], None, platform_version)
            .expect("fetch should succeed");
        assert_eq!(balance, None);
    }

    #[test]
    fn fetch_missing_with_apply_false_returns_some_zero_for_estimation() {
        // When apply = false (stateless), a missing key should return Some(0),
        // not None, because estimation paths assume the entry exists.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut drive_ops = vec![];
        let balance = drive
            .fetch_prefunded_specialized_balance_operations_v0(
                [7u8; 32],
                false,
                None,
                &mut drive_ops,
                platform_version,
            )
            .expect("stateless fetch should succeed");
        assert_eq!(balance, Some(0));
    }

    #[test]
    fn fetch_existing_balance_roundtrips() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([99u8; 32]);

        drive
            .add_prefunded_specialized_balance(id, 1_234_567, None, platform_version)
            .expect("seed");

        let balance = drive
            .fetch_prefunded_specialized_balance_v0(id.to_buffer(), None, platform_version)
            .expect("fetch");
        assert_eq!(balance, Some(1_234_567));
    }

    #[test]
    fn fetch_negative_sum_item_returns_corrupted_element_type_error() {
        // Corrupted-state branch: a SumItem with a negative balance is corrupted.
        // We write one directly via a batch op with a negative value and expect
        // a CorruptedElementType error from the fetch path.
        use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([50u8; 32]);

        // Directly insert a negative SumItem bypassing the checked add path.
        let op = QualifiedGroveDbOp::insert_or_replace_op(
            prefunded_specialized_balances_for_voting_path_vec(),
            id.to_vec(),
            Element::new_sum_item(-42),
        );
        drive
            .grove_apply_batch(
                crate::util::batch::GroveDbOpBatch::from_operations(vec![op]),
                false,
                None,
                &platform_version.drive,
            )
            .expect("apply negative sum item");

        let err = drive
            .fetch_prefunded_specialized_balance_v0(id.to_buffer(), None, platform_version)
            .expect_err("expected CorruptedElementType");
        use crate::error::drive::DriveError;
        use crate::error::Error;
        assert!(
            matches!(err, Error::Drive(DriveError::CorruptedElementType(_))),
            "expected CorruptedElementType, got: {:?}",
            err
        );
    }

    #[test]
    fn fetch_with_costs_returns_fees_for_existing_balance() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let id = Identifier::from([55u8; 32]);

        drive
            .add_prefunded_specialized_balance(id, 42, None, platform_version)
            .expect("seed");

        let (balance, fees) = drive
            .fetch_prefunded_specialized_balance_with_costs_v0(
                id.to_buffer(),
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("fetch with costs");
        assert_eq!(balance, Some(42));
        assert!(
            fees.processing_fee > 0 || fees.storage_fee > 0,
            "expected non-zero fees"
        );
    }
}
