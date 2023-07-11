use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::block::block_info::BlockInfo;
use crate::fee::calculate_fee;
use dpp::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use dpp::data_contract::DataContract;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::result::FeeResult;

impl Drive {
    // TODO(doc): Elaborate more on associated fee? Is it always a refund (negative fee)
    //  Or could be positive as well?
    /// Deletes a document and returns the associated fee.
    pub(super) fn delete_document_for_contract_v0(
        &self,
        document_id: [u8; 32],
        contract: &DataContract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        self.delete_document_for_contract_apply_and_add_to_operations(
            document_id,
            contract,
            document_type_name,
            owner_id,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }
}