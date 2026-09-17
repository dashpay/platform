use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_readiness_path,
    prefunded_specialized_balances_for_readiness_path_vec,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn deduct_from_readiness_fund_operations_v0(
        &self,
        fund_id: Identifier,
        amount: Credits,
        reserve: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness_fund_update(
                estimated_costs_only_with_layer_info,
                platform_version,
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

        let path = prefunded_specialized_balances_for_readiness_path();
        let previous_credits = match self.grove_get_raw_value_u64_from_encoded_var_vec(
            (&path).into(),
            fund_id.as_slice(),
            direct_query_type,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )? {
            None => {
                if estimated_costs_only_with_layer_info.is_none() {
                    return Err(Error::Drive(
                        DriveError::PrefundedSpecializedBalanceDoesNotExist(format!(
                            "trying to deduct from a readiness fund {} that does not exist",
                            fund_id
                        )),
                    ));
                } else {
                    i64::MAX as u64
                }
            }
            Some(value) => value,
        };
        let spendable = previous_credits.saturating_sub(reserve);
        if amount > spendable {
            return Err(Error::Drive(DriveError::PrefundedSpecializedBalanceNotEnough(
                spendable, amount,
            )));
        }
        let new_total = previous_credits
            .checked_sub(amount)
            .ok_or(Error::Drive(DriveError::PrefundedSpecializedBalanceNotEnough(
                previous_credits,
                amount,
            )))?;
        let replace_op = QualifiedGroveDbOp::replace_op(
            prefunded_specialized_balances_for_readiness_path_vec(),
            fund_id.to_vec(),
            Element::new_sum_item(new_total as i64),
        );
        drive_operations.push(GroveOperation(replace_op));
        Ok(drive_operations)
    }
}
