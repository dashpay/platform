use crate::drive::contract::moderation::types::encode_moderation_action_count;
use crate::drive::contract::paths::contract_moderation_action_counts_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn set_contract_moderation_action_count_operations_v0(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        count: u32,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_moderation_action_counts(
                contract_id.to_buffer(),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }
        // No storage flags: every count has the same size, so a replacement adds no bytes for
        // anyone to own, and the settle that deletes the counts refunds nobody.
        let op = QualifiedGroveDbOp::insert_or_replace_op(
            contract_moderation_action_counts_path_vec(contract_id.as_slice()),
            identity_id.to_vec(),
            Element::new_item(encode_moderation_action_count(count)),
        )
        .dont_check_for_backwards_references();
        Ok(vec![GroveOperation(op)])
    }
}
