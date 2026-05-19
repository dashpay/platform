use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;

use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_voting_path,
    prefunded_specialized_balances_for_voting_path_vec,
};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations to add to the specialized balance
    #[inline(always)]
    pub(super) fn deduct_from_prefunded_specialized_balance_operations_v0(
        &self,
        specialized_balance_id: Identifier,
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_prefunded_specialized_balance_update(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        let path_holding_specialized_balances = prefunded_specialized_balances_for_voting_path();
        let previous_credits_in_specialized_balance = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path_holding_specialized_balances).into(),
                specialized_balance_id.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .ok_or(Error::Drive(
                DriveError::PrefundedSpecializedBalanceDoesNotExist(format!(
                    "trying to deduct from a prefunded specialized balance {} that does not exist",
                    specialized_balance_id
                )),
            ))?;
        let new_total = previous_credits_in_specialized_balance
            .checked_sub(amount)
            .ok_or(Error::Drive(
                DriveError::PrefundedSpecializedBalanceNotEnough(
                    previous_credits_in_specialized_balance,
                    amount,
                ),
            ))?;
        let path_holding_total_credits_vec = prefunded_specialized_balances_for_voting_path_vec();
        let replace_op = QualifiedGroveDbOp::replace_op(
            path_holding_total_credits_vec,
            specialized_balance_id.to_vec(),
            Element::new_sum_item(new_total as i64),
        );
        drive_operations.push(GroveOperation(replace_op));
        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;

    #[test]
    fn deduct_from_missing_balance_returns_does_not_exist_error() {
        // Error branch: previous balance is None -> PrefundedSpecializedBalanceDoesNotExist.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([1u8; 32]);

        let mut estimated = None;
        let err = drive
            .deduct_from_prefunded_specialized_balance_operations_v0(
                id,
                10,
                &mut estimated,
                None,
                platform_version,
            )
            .expect_err("expected missing balance error");
        assert!(
            matches!(
                err,
                Error::Drive(DriveError::PrefundedSpecializedBalanceDoesNotExist(_))
            ),
            "expected PrefundedSpecializedBalanceDoesNotExist, got: {:?}",
            err
        );
    }

    #[test]
    fn deduct_more_than_available_returns_not_enough_error() {
        // checked_sub branch: amount > previous -> PrefundedSpecializedBalanceNotEnough.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([2u8; 32]);

        drive
            .add_prefunded_specialized_balance(id, 100, None, platform_version)
            .expect("seed");

        let mut estimated = None;
        let err = drive
            .deduct_from_prefunded_specialized_balance_operations_v0(
                id,
                101,
                &mut estimated,
                None,
                platform_version,
            )
            .expect_err("expected not enough balance error");
        match err {
            Error::Drive(DriveError::PrefundedSpecializedBalanceNotEnough(avail, req)) => {
                assert_eq!(avail, 100);
                assert_eq!(req, 101);
            }
            other => panic!(
                "expected PrefundedSpecializedBalanceNotEnough, got: {:?}",
                other
            ),
        }
    }

    #[test]
    fn deduct_exact_amount_leaves_zero_balance() {
        // Boundary: deducting exactly the available amount should work, leaving 0.
        // We use the low-level _operations_v0 + manual apply because the public
        // dispatcher `deduct_from_prefunded_specialized_balance` currently only
        // knows version 0 while platform_version.deduct_from_prefunded_specialized_balance = 1.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([3u8; 32]);

        drive
            .add_prefunded_specialized_balance(id, 500, None, platform_version)
            .expect("seed");

        let mut estimated = None;
        let ops = drive
            .deduct_from_prefunded_specialized_balance_operations_v0(
                id,
                500,
                &mut estimated,
                None,
                platform_version,
            )
            .expect("build deduct ops");
        let grove_ops =
            crate::fees::op::LowLevelDriveOperation::grovedb_operations_batch_consume(ops);
        drive
            .grove_apply_batch_with_add_costs(
                grove_ops,
                false,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("apply deduct ops");

        let fetched = drive
            .fetch_prefunded_specialized_balance(id.to_buffer(), None, platform_version)
            .expect("fetch");
        assert_eq!(fetched, Some(0));
    }

    #[test]
    fn estimation_costs_populated_on_estimate_request() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([4u8; 32]);

        drive
            .add_prefunded_specialized_balance(id, 1_000, None, platform_version)
            .expect("seed");

        let mut estimated = Some(std::collections::HashMap::new());
        let _ops = drive
            .deduct_from_prefunded_specialized_balance_operations_v0(
                id,
                100,
                &mut estimated,
                None,
                platform_version,
            )
            .expect("deduct with estimation");
        assert!(!estimated.unwrap().is_empty());
    }
}
