use crate::drive::contract::paths::{contract_fee_pots_path, contract_fee_pots_path_vec};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::balances::credits::Credits;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn deduct_from_contract_fee_pot_operations_v0(
        &self,
        contract_id: Identifier,
        pot: ContractFeePot,
        amount: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_fee_pot_update(
                pot,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        let path = contract_fee_pots_path(pot);
        let previous_credits = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path).into(),
                contract_id.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .unwrap_or_default();
        let new_total = previous_credits.checked_sub(amount).ok_or_else(|| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "deducting more from a contract fee pot than it holds",
            ))
        })?;
        let op = QualifiedGroveDbOp::replace_op(
            contract_fee_pots_path_vec(pot),
            contract_id.to_vec(),
            Element::new_sum_item(new_total as i64),
        )
        .dont_check_for_backwards_references();
        drive_operations.push(GroveOperation(op));
        Ok(drive_operations)
    }
}
