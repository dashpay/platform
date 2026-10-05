mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::ContractWarning;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Writes the warning list entry of `identity_id` on `contract_id` as `warnings`: the
    /// warnings it carried, with the new one last. Paid by `moderator_id`, whose identity the
    /// entry's storage flags name for the refund on removal.
    ///
    /// The caller must have checked that the contract keeps a warning list, and that
    /// `warnings` holds at most `SystemLimits::max_contract_warnings_per_identity`;
    /// `replaces_existing` says whether the identity already carries an entry, which is then
    /// replaced. Applies the operations when `apply` is true, otherwise only estimates.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity to warn.
    /// * `warnings`: The identity's warnings after this one, oldest first; at least one. The
    ///   length of each reason is the caller's to check.
    /// * `replaces_existing`: Whether an entry for the identity exists and is replaced.
    /// * `moderator_id`: The identity that pays for the entry.
    /// * `block_info`: The current block.
    /// * `apply`: Whether to apply or only estimate.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(FeeResult)` with the fee of the write.
    /// * `Err(Error)` when the version is unknown or GroveDB refuses an operation.
    #[allow(clippy::too_many_arguments)]
    pub fn add_contract_warning(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        warnings: &[ContractWarning],
        replaces_existing: bool,
        moderator_id: Identifier,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .add_contract_warning
        {
            0 => self.add_contract_warning_v0(
                contract_id,
                identity_id,
                warnings,
                replaces_existing,
                moderator_id,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contract_warning".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The low level operations of [`Drive::add_contract_warning`]. With layer information the
    /// operations are built for estimation only.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity to warn.
    /// * `warnings`: The identity's warnings after this one, oldest first; at least one.
    /// * `replaces_existing`: Whether an entry for the identity exists and is replaced.
    /// * `moderator_id`: The identity that pays for the entry.
    /// * `block_info`: The current block.
    /// * `estimated_costs_only_with_layer_info`: The estimation map for a dry run.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the operations.
    /// * `Err(Error)` when the version is unknown or GroveDB refuses an operation.
    #[allow(clippy::too_many_arguments)]
    pub fn add_contract_warning_operations(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        warnings: &[ContractWarning],
        replaces_existing: bool,
        moderator_id: Identifier,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .add_contract_warning
        {
            0 => self.add_contract_warning_operations_v0(
                contract_id,
                identity_id,
                warnings,
                replaces_existing,
                moderator_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contract_warning_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
