use crate::drive::contract::fee_pots::types::encode_last_claim;
use crate::drive::contract::paths::{contract_last_fee_claim_key, contract_other_path_vec};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use dpp::data_contract::document_type::action_fees::{ContractFeePot, ContractFeePotLastClaim};
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn set_contract_last_fee_claim_operations_v0(
        &self,
        contract_id: Identifier,
        pot: ContractFeePot,
        last_claim: &ContractFeePotLastClaim,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // A claim carries no document or contract operation, so nothing else in its batch
            // describes the layers above the contract.
            Self::add_estimation_costs_for_levels_up_to_contract(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            Self::add_estimation_costs_for_contract_other_tree(
                contract_id.to_buffer(),
                estimated_costs_only_with_layer_info,
            );
        }
        // The item carries no storage flags: it is never deleted, so there is no refund to
        // attribute, and every last claim has the same size, so the claim that replaces it
        // adds no bytes for anyone to own.
        let op = QualifiedGroveDbOp::insert_or_replace_op(
            contract_other_path_vec(contract_id.as_slice()),
            contract_last_fee_claim_key(pot).to_vec(),
            Element::new_item(encode_last_claim(last_claim)),
        )
        .dont_check_for_backwards_references();
        Ok(vec![GroveOperation(op)])
    }
}
