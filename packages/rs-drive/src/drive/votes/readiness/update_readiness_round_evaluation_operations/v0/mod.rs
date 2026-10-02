use crate::drive::votes::paths::{readiness_round_tree_path_vec, READINESS_ROUND_RECORD_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::ReadinessRound;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    pub(super) fn update_readiness_round_evaluation_operations_v0(
        &self,
        round: &ReadinessRound,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let contract_id = round.contract_id().to_buffer();
        let round_id = round.round_id();
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness(
                contract_id,
                round_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }
        // The record grows when its evaluation mark or its status is first set, so the
        // rewrite is priced as an insert of the new bytes rather than a same-size replace.
        let op = QualifiedGroveDbOp::insert_or_replace_op(
            readiness_round_tree_path_vec(contract_id, round_id),
            vec![READINESS_ROUND_RECORD_KEY],
            Element::new_item(round.serialize_to_bytes()?),
        );
        Ok(vec![GroveOperation(op)])
    }
}
