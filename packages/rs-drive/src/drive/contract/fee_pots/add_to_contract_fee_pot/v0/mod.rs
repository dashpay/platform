use crate::drive::contract::paths::{
    contract_fee_pots_key, contract_fee_pots_path, contract_fee_pots_path_vec,
};
use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_path, prefunded_specialized_balances_path_vec,
};
use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::balances::credits::{Credits, MAX_CREDITS};
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn add_to_contract_fee_pot_operations_v0(
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
        let previous_credits = self.grove_get_raw_value_u64_from_encoded_var_vec(
            (&path).into(),
            contract_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        let new_total = previous_credits
            .unwrap_or_default()
            .checked_add(amount)
            .ok_or(Error::Fee(FeeError::Overflow(
                "adding to a contract fee pot would overflow credits",
            )))?;
        // As for every balance, i64::MAX itself is avoided.
        if new_total >= MAX_CREDITS {
            return Err(Error::Identity(IdentityError::CriticalBalanceOverflow(
                "trying to set a contract fee pot to over max credits amount (i64::MAX)",
            )));
        }
        // A chain that reached protocol version 14 on a build from before the fee pots never
        // ran the upgrade step that creates their trees, and was not born with them either.
        // The first fee a pot receives therefore checks that its tree is there, a billed read,
        // and creates it in the same batch when it is not. A pot that already holds credits
        // proves its tree, so no later fee pays for the check.
        if previous_credits.is_none() {
            let prefunded_path = prefunded_specialized_balances_path();
            let pots_tree_exists = self.grove_has_raw(
                (&prefunded_path).into(),
                contract_fee_pots_key(pot),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
            if !pots_tree_exists {
                drive_operations.push(GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
                    prefunded_specialized_balances_path_vec(),
                    contract_fee_pots_key(pot).to_vec(),
                    Element::empty_sum_tree(),
                )));
            }
        }
        let op = if previous_credits.is_some() {
            QualifiedGroveDbOp::replace_op(
                contract_fee_pots_path_vec(pot),
                contract_id.to_vec(),
                Element::new_sum_item(new_total as i64),
            )
        } else {
            QualifiedGroveDbOp::insert_or_replace_op(
                contract_fee_pots_path_vec(pot),
                contract_id.to_vec(),
                Element::new_sum_item(new_total as i64),
            )
        }
        .dont_check_for_backwards_references();
        drive_operations.push(GroveOperation(op));
        Ok(drive_operations)
    }
}
