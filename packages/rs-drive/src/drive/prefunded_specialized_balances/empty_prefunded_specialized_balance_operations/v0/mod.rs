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
