use crate::drive::object_size_info::DocumentAndContractInfo;
use crate::drive::Drive;

use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Adds a document to a contract.
    #[inline(always)]
    pub(super) fn add_document_for_contract_v0(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        override_document: bool,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.add_document_for_contract_apply_and_add_to_operations(
            document_and_contract_info,
            override_document,
            &block_info,
            true,
            apply,
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
        )?;
        Ok(fees)
    }
}
