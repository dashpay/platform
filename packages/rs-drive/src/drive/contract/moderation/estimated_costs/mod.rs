mod v0;

use crate::drive::contract::moderation::types::ContractDocumentRecords;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::FeatureVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimated layer information for creating a contract's moderation list trees:
    /// the levels up to the contract and the contract's own subtree.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose moderation trees are created.
    /// * `estimated_costs_only_with_layer_info`: The estimation map the layers are added to.
    /// * `drive_version`: The drive version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the layers are added to the map.
    /// * `Err(Error)` when the method version, or that of a nested estimation, is unknown.
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
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the list belongs to.
    /// * `list`: The moderation list the entry is written to.
    /// * `estimated_costs_only_with_layer_info`: The estimation map the layers are added to.
    /// * `drive_version`: The drive version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the layers are added to the map.
    /// * `Err(Error)` when the method version, or that of a nested estimation, is unknown.
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
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The elected contract the counts belong to.
    /// * `estimated_costs_only_with_layer_info`: The estimation map the layers are added to.
    /// * `drive_version`: The drive version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the layers are added to the map.
    /// * `Err(Error)` when the method version, or that of a nested estimation, is unknown.
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

    /// Adds the layers the trees of one kind of a contract's document records are created
    /// through: the moderation layers above them, and the tree of all of them.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose records trees are created.
    /// * `records`: The kind of records: removal records, or settled-deletion approvals.
    /// * `estimated_costs_only_with_layer_info`: The estimation map the layers are added to.
    /// * `drive_version`: The drive version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the layers are added to the map.
    /// * `Err(Error)` when the method version, or that of a nested estimation, is unknown.
    pub(crate) fn add_estimation_costs_for_contract_document_record_trees(
        contract_id: [u8; 32],
        records: ContractDocumentRecords,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match Self::contract_document_record_estimation_version(records, drive_version) {
            0 => Self::add_estimation_costs_for_contract_document_record_trees_v0(
                contract_id,
                records,
                estimated_costs_only_with_layer_info,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_contract_document_record_trees".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the layers one record of a contract's document records is written through: the
    /// removal record of a moderator's deletion, or the approvals of a settled document's
    /// deletion.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the document is of.
    /// * `records`: The kind of record.
    /// * `document_type_name`: The document's type, whose records tree holds the record.
    /// * `estimated_value_size`: The size the records the write walks past are estimated at:
    ///   a typical removal record, with what its type keeps
    ///   (`types::estimated_document_removal_value_size`), or a typical approvals record
    ///   (`types::estimated_settled_deletion_value_size`).
    /// * `estimated_costs_only_with_layer_info`: The estimation map the layers are added to.
    /// * `drive_version`: The drive version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the layers are added to the map.
    /// * `Err(Error)` when the method version, or that of a nested estimation, is unknown.
    pub(crate) fn add_estimation_costs_for_contract_document_record(
        contract_id: [u8; 32],
        records: ContractDocumentRecords,
        document_type_name: &str,
        estimated_value_size: u32,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match Self::contract_document_record_estimation_version(records, drive_version) {
            0 => Self::add_estimation_costs_for_contract_document_record_v0(
                contract_id,
                records,
                document_type_name,
                estimated_value_size,
                estimated_costs_only_with_layer_info,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_contract_document_record".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The version of the estimation of one kind of document records: each kind keeps its own.
    fn contract_document_record_estimation_version(
        records: ContractDocumentRecords,
        drive_version: &DriveVersion,
    ) -> FeatureVersion {
        let moderation = &drive_version.methods.contract.moderation;
        match records {
            ContractDocumentRecords::Removals => {
                moderation.add_estimation_costs_for_contract_document_removal
            }
            ContractDocumentRecords::SettledDeletions => {
                moderation.add_estimation_costs_for_contract_settled_deletion
            }
        }
    }
}
