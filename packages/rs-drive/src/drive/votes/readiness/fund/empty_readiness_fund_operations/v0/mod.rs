use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_readiness_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn empty_readiness_fund_operations_v0(
        &self,
        fund_id: Identifier,
        error_if_does_not_exist: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Credits, Vec<LowLevelDriveOperation>), Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness_fund_update(
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }
        // The checked read: a stored fund must be a nonnegative sum item. An estimate prices
        // the read without state and sees an empty fund.
        let previous_credits = match self.fetch_readiness_fund_operations(
            fund_id.to_buffer(),
            estimated_costs_only_with_layer_info.is_none(),
            transaction,
            &mut drive_operations,
            platform_version,
        )? {
            None => {
                if estimated_costs_only_with_layer_info.is_none() {
                    return if error_if_does_not_exist {
                        Err(Error::Drive(
                            DriveError::PrefundedSpecializedBalanceDoesNotExist(format!(
                                "trying to empty a readiness fund {} that does not exist",
                                fund_id
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
        let delete_op = QualifiedGroveDbOp::delete_op(
            prefunded_specialized_balances_for_readiness_path_vec(),
            fund_id.to_vec(),
        );
        drive_operations.push(GroveOperation(delete_op));
        Ok((previous_credits, drive_operations))
    }
}
