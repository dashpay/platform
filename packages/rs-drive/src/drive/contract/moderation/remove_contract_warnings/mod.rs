mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Takes `identity_id` off the warning list of `contract_id`: every warning it carries
    /// goes. The storage refund goes to the identity the entry's storage flags name.
    ///
    /// The caller must have checked that the entry exists. Applies the operations when
    /// `apply` is true, otherwise only estimates.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity whose warnings are cleared.
    /// * `block_info`: The current block.
    /// * `apply`: Whether to apply or only estimate.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(FeeResult)` with the fee of the deletion.
    /// * `Err(Error)` when the version is unknown or GroveDB refuses an operation.
    #[allow(clippy::too_many_arguments)]
    pub fn remove_contract_warnings(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
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
            .remove_contract_warnings
        {
            0 => self.remove_contract_warnings_v0(
                contract_id,
                identity_id,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_contract_warnings".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The low level operations of [`Drive::remove_contract_warnings`]. With layer information the
    /// operations are built for estimation only.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity whose warnings are cleared.
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
    pub fn remove_contract_warnings_operations(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
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
            .remove_contract_warnings
        {
            0 => self.remove_contract_warnings_operations_v0(
                contract_id,
                identity_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_contract_warnings_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
