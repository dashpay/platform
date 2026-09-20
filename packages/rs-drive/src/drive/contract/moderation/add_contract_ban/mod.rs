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
    /// Puts `identity_id` on the banlist of `contract_id`, paid by `moderator_id`, whose
    /// identity the entry's storage flags name for the refund on removal.
    ///
    /// The caller must have checked that the contract keeps a banlist and that the identity is
    /// not on it. Applies the operations when `apply` is true, otherwise only estimates.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity to ban.
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
    pub fn add_contract_ban(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
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
            .add_contract_ban
        {
            0 => self.add_contract_ban_v0(
                contract_id,
                identity_id,
                moderator_id,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contract_ban".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The low level operations of [`Drive::add_contract_ban`]. With layer information the
    /// operations are built for estimation only.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity to ban.
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
    pub fn add_contract_ban_operations(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
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
            .add_contract_ban
        {
            0 => self.add_contract_ban_operations_v0(
                contract_id,
                identity_id,
                moderator_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contract_ban_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
