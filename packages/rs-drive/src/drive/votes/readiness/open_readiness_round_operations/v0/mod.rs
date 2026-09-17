use crate::drive::votes::paths::{
    readiness_contract_tree_path_vec, readiness_contracts_tree_path,
    readiness_contracts_tree_path_vec, readiness_round_tree_path_vec,
    READINESS_CURRENT_ROUND_POINTER_KEY, READINESS_ROUND_RECORD_KEY,
    READINESS_ROUND_REPORTS_TREE_KEY,
};
use crate::drive::votes::readiness::open_readiness_round_operations::ReadinessRoundFunding;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::block::block_info::BlockInfo;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::{ReadinessRound, ReadinessRoundOpening};
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    pub(super) fn open_readiness_round_operations_v0(
        &self,
        opening: ReadinessRoundOpening,
        funding: ReadinessRoundFunding,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(ReadinessRound, Vec<LowLevelDriveOperation>), Error> {
        let round = ReadinessRound::new(self.config.network.magic(), opening, platform_version)
            ?;
        let contract_id = round.contract_id().to_buffer();
        let round_id = round.round_id();
        let mut drive_operations = vec![];

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_readiness(
                contract_id,
                round_id,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        // Is there a round to replace? Only a real run reads; an estimate prices the
        // replacement shape (the larger of the two) so the estimate covers both.
        let previous = if estimated_costs_only_with_layer_info.is_none() {
            self.fetch_readiness_round_operations(
                contract_id,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
        } else {
            None
        };
        let contract_tree_exists = if estimated_costs_only_with_layer_info.is_none() {
            let contracts_path = readiness_contracts_tree_path();
            self.grove_has_raw(
                (&contracts_path).into(),
                &contract_id,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
        } else {
            false
        };

        let mut refund = 0u64;
        if let Some(previous) = previous.as_ref() {
            let (retirement, retire_operations) = self.retire_readiness_round_operations(
                previous,
                funding.cleanup_reserve,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?;
            refund = retirement.refund;
            drive_operations.extend(retire_operations);
        }

        // The contract's tree and its pointer. The pointer is one operation on its path and
        // key whatever happened before: an insert when the contract had no tree, a replace
        // when a round is being replaced.
        if !contract_tree_exists {
            drive_operations.push(LowLevelDriveOperation::for_known_path_key_empty_tree(
                readiness_contracts_tree_path_vec(),
                contract_id.to_vec(),
                None,
            ));
        }
        let pointer = Element::new_item(round_id.to_vec());
        let pointer_op = if contract_tree_exists {
            QualifiedGroveDbOp::insert_or_replace_op(
                readiness_contract_tree_path_vec(contract_id),
                vec![READINESS_CURRENT_ROUND_POINTER_KEY as u8],
                pointer,
            )
        } else {
            QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                readiness_contract_tree_path_vec(contract_id),
                vec![READINESS_CURRENT_ROUND_POINTER_KEY as u8],
                pointer,
            )
        };
        drive_operations.push(GroveOperation(pointer_op));

        // The new round tree under its own key: record, empty reports count tree, no cursor.
        drive_operations.push(LowLevelDriveOperation::for_known_path_key_empty_tree(
            readiness_contract_tree_path_vec(contract_id),
            round_id.to_vec(),
            None,
        ));
        drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
            readiness_round_tree_path_vec(contract_id, round_id),
            vec![READINESS_ROUND_RECORD_KEY],
            Element::new_item(round.serialize_to_bytes()?),
        ));
        drive_operations.push(LowLevelDriveOperation::for_known_path_key_empty_count_tree(
            readiness_round_tree_path_vec(contract_id, round_id),
            vec![READINESS_ROUND_REPORTS_TREE_KEY],
            None,
        ));

        // The new fund.
        if funding.initial_funding > 0 {
            drive_operations.extend(self.add_readiness_fund_operations(
                round.funding_id(),
                funding.initial_funding,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        // One balance write for the payer: the refund of the retired fund netted against the
        // new funding. Two absolute writes on one balance key in one batch would collapse.
        if let Some(payer_id) = round.payer().identity_id() {
            let payer = payer_id.to_buffer();
            if funding.initial_funding > refund {
                drive_operations.extend(self.remove_from_identity_balance_operations(
                    payer,
                    funding.initial_funding - refund,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?);
            } else if refund > funding.initial_funding {
                drive_operations.extend(self.add_to_identity_balance_operations(
                    payer,
                    refund - funding.initial_funding,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?);
            }
        }

        Ok((round, drive_operations))
    }
}
