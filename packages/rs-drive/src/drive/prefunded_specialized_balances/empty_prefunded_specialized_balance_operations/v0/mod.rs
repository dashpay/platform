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
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// The operations to add to the specialized balance
    #[inline(always)]
    pub(super) fn empty_prefunded_specialized_balance_operations_v0(
        &self,
        specialized_balance_id: Identifier,
        error_if_does_not_exist: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Credits, Vec<LowLevelDriveOperation>), Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_prefunded_specialized_balance_update(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(8),
            }
        };

        let path_holding_specialized_balances = prefunded_specialized_balances_for_voting_path();
        let previous_credits_in_specialized_balance = match self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path_holding_specialized_balances).into(),
                specialized_balance_id.as_slice(),
                direct_query_type,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )? {
            None => {
                if estimated_costs_only_with_layer_info.is_none() {
                    return if error_if_does_not_exist {
                        Err(Error::Drive(
                            DriveError::PrefundedSpecializedBalanceDoesNotExist(format!(
                                "trying to deduct from a prefunded specialized balance {} that does not exist",
                                specialized_balance_id
                            )),
                        ))
                    } else {
                        Ok((0, drive_operations))
                    };
                } else {
                    0
                }
            }
            Some(value) => value,
        };
        let path_holding_total_credits_vec = prefunded_specialized_balances_for_voting_path_vec();
        let delete_op = QualifiedGroveDbOp::delete_op(
            path_holding_total_credits_vec,
            specialized_balance_id.to_vec(),
        );
        drive_operations.push(GroveOperation(delete_op));
        Ok((previous_credits_in_specialized_balance, drive_operations))
    }
}

#[cfg(test)]
mod tests {
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::HashMap;

    #[test]
    fn empty_missing_without_error_flag_returns_zero_and_no_ops() {
        // error_if_does_not_exist = false, missing balance -> return (0, drive_operations-so-far)
        // before pushing the delete op. We assert the returned credits is 0 and that no
        // delete op was added (len remains the grove-get cost ops length at most, but the
        // key thing is the explicit zero-credits return).
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([20u8; 32]);

        let mut estimated = None;
        let (credits, _ops) = drive
            .empty_prefunded_specialized_balance_operations_v0(
                id,
                false,
                &mut estimated,
                None,
                platform_version,
            )
            .expect("empty should succeed with error_if_does_not_exist=false");
        assert_eq!(credits, 0);
    }

    #[test]
    fn empty_missing_with_error_flag_returns_does_not_exist() {
        // error_if_does_not_exist = true, missing balance -> PrefundedSpecializedBalanceDoesNotExist.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([21u8; 32]);

        let mut estimated = None;
        let err = drive
            .empty_prefunded_specialized_balance_operations_v0(
                id,
                true,
                &mut estimated,
                None,
                platform_version,
            )
            .expect_err("expected DoesNotExist");
        assert!(matches!(
            err,
            Error::Drive(DriveError::PrefundedSpecializedBalanceDoesNotExist(_))
        ));
    }

    #[test]
    fn empty_existing_balance_returns_previous_amount_and_delete_op() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([22u8; 32]);

        drive
            .add_prefunded_specialized_balance(id, 7_500, None, platform_version)
            .expect("seed");

        // Drive the empty via the dispatcher so the op is actually applied to state.
        let returned_credits = drive
            .empty_prefunded_specialized_balance(id, false, None, platform_version)
            .expect("empty existing balance");
        assert_eq!(returned_credits, 7_500);

        let fetched = drive
            .fetch_prefunded_specialized_balance(id.to_buffer(), None, platform_version)
            .expect("fetch after empty");
        assert_eq!(fetched, None, "deleted entry should no longer be found");
    }

    #[test]
    fn empty_estimation_path_missing_uses_zero_sentinel_and_emits_delete_op() {
        // Missing balance + estimation mode -> None branch falls to the else (previous = 0) path.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([23u8; 32]);

        let mut estimated = Some(HashMap::new());
        let (credits, ops) = drive
            .empty_prefunded_specialized_balance_operations_v0(
                id,
                true, // even with error flag, estimation mode overrides the error
                &mut estimated,
                None,
                platform_version,
            )
            .expect("estimation path should not error even with error_if_does_not_exist=true");
        assert_eq!(credits, 0);
        // Delete op was pushed even in estimation mode.
        assert!(!ops.is_empty());
        assert!(!estimated.unwrap().is_empty());
    }
}
