use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use std::collections::HashMap;

use crate::drive::Drive;
use crate::error::document::DocumentError;
use dpp::block::block_info::BlockInfo;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    /// Deletes a document and returns the associated fee.
    /// The contract CBOR is given instead of the contract itself.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn delete_document_for_contract_id_v0(
        &self,
        document_id: Identifier,
        contract_id: Identifier,
        document_type_name: &str,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract_id.to_buffer(),
                Some(&block_info.epoch),
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Document(DocumentError::DataContractNotFound))?;

        let contract = &contract_fetch_info.contract;

        self.delete_document_for_contract_apply_and_add_to_operations(
            document_id,
            contract,
            document_type_name,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            previous_fee_versions,
        )?;

        Ok(fees)
    }
}
