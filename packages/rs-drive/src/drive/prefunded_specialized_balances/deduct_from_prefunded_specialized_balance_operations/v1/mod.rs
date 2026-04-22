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
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// The operations to add to the specialized balance
    #[inline(always)]
    pub(super) fn deduct_from_prefunded_specialized_balance_operations_v1(
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
                    return
                        Err(Error::Drive(
                            DriveError::PrefundedSpecializedBalanceDoesNotExist(format!(
                                "trying to deduct from a prefunded specialized balance {} that does not exist",
                                specialized_balance_id
                            )),
                        ));
                } else {
                    i64::MAX as u64
                }
            }
            Some(value) => value,
        };
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
    use std::collections::HashMap;

    #[test]
    fn v1_deduct_missing_in_stateful_mode_is_error() {
        // In v1, with estimated_costs_only = None (stateful), missing balance still errors.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([80u8; 32]);

        let mut estimated = None;
        let err = drive
            .deduct_from_prefunded_specialized_balance_operations_v1(
                id,
                1,
                &mut estimated,
                None,
                platform_version,
            )
            .expect_err("expected missing balance error");
        assert!(matches!(
            err,
            Error::Drive(DriveError::PrefundedSpecializedBalanceDoesNotExist(_))
        ));
    }

    #[test]
    fn v1_deduct_missing_in_stateless_mode_uses_sentinel_balance() {
        // In v1, when estimated_costs_only is Some(_), a missing balance is treated
        // as i64::MAX (sentinel) so estimation can proceed without an actual state entry.
        // This exercises the None => i64::MAX as u64 branch.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([81u8; 32]);

        let mut estimated = Some(HashMap::new());
        let ops = drive
            .deduct_from_prefunded_specialized_balance_operations_v1(
                id,
                1,
                &mut estimated,
                None,
                platform_version,
            )
            .expect("estimation should succeed even without existing balance");
        assert!(!ops.is_empty());
        assert!(!estimated.unwrap().is_empty());
    }

    #[test]
    fn v1_deduct_more_than_available_returns_not_enough() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let id = Identifier::from([82u8; 32]);

        drive
            .add_prefunded_specialized_balance(id, 5, None, platform_version)
            .expect("seed");

        let mut estimated = None;
        let err = drive
            .deduct_from_prefunded_specialized_balance_operations_v1(
                id,
                10,
                &mut estimated,
                None,
                platform_version,
            )
            .expect_err("expected not enough error");
        match err {
            Error::Drive(DriveError::PrefundedSpecializedBalanceNotEnough(avail, req)) => {
                assert_eq!(avail, 5);
                assert_eq!(req, 10);
            }
            other => panic!(
                "expected PrefundedSpecializedBalanceNotEnough, got: {:?}",
                other
            ),
        }
    }
}
