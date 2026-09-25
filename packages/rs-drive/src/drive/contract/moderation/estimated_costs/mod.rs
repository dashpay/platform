mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimated layer information for creating a contract's moderation list trees:
    /// the levels up to the contract and the contract's own subtree.
    pub(crate) fn add_estimation_costs_for_contract_moderation_trees(
        contract_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .contract
            .moderation
            .add_estimation_costs_for_contract_moderation_trees
        {
            0 => Self::add_estimation_costs_for_contract_moderation_trees_v0(
                contract_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_contract_moderation_trees".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the estimated layer information for writing one entry of a contract's moderation
    /// list: the levels up to the contract, the contract's subtree and the list's tree.
    pub(crate) fn add_estimation_costs_for_contract_moderation_entry(
        contract_id: [u8; 32],
        list: ContractModerationList,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .contract
            .moderation
            .add_estimation_costs_for_contract_moderation_entry
        {
            0 => Self::add_estimation_costs_for_contract_moderation_entry_v0(
                contract_id,
                list,
                estimated_costs_only_with_layer_info,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_contract_moderation_entry".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the estimated layer information for writing or deleting moderation action counts
    /// of an elected contract: the levels up to the contract, the contract's subtree and the
    /// tree of the counts.
    pub(crate) fn add_estimation_costs_for_contract_moderation_action_counts(
        contract_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .contract
            .moderation
            .add_estimation_costs_for_contract_moderation_action_counts
        {
            0 => Self::add_estimation_costs_for_contract_moderation_action_counts_v0(
                contract_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_contract_moderation_action_counts".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the layers a contract insertion or update touches when it creates the trees of
    /// the document removal records.
    pub(crate) fn add_estimation_costs_for_contract_document_removal_trees(
        contract_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .contract
            .moderation
            .add_estimation_costs_for_contract_document_removal
        {
            0 => Self::add_estimation_costs_for_contract_document_removal_trees_v0(
                contract_id,
                estimated_costs_only_with_layer_info,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_contract_document_removal_trees".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the layers the record of a moderator's document deletion is written through.
    pub(crate) fn add_estimation_costs_for_contract_document_removal(
        contract_id: [u8; 32],
        document_type_name: &str,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .contract
            .moderation
            .add_estimation_costs_for_contract_document_removal
        {
            0 => Self::add_estimation_costs_for_contract_document_removal_v0(
                contract_id,
                document_type_name,
                estimated_costs_only_with_layer_info,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_contract_document_removal".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
