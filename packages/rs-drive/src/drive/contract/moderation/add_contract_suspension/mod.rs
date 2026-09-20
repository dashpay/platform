mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Puts `identity_id` on the suspension list of `contract_id`, paid by `moderator_id`, whose
    /// identity the entry's storage flags name for the refund on removal.
    ///
    /// The caller must have checked that the contract keeps a suspension list; `replaces_existing` says
    /// whether the identity already carries a suspension, which is then replaced. Applies the operations when `apply` is true, otherwise only estimates.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity to suspend.
    /// * `until`: The block time, in milliseconds, at which the suspension lapses.
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
    pub fn add_contract_suspension(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        until: TimestampMillis,
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
            .add_contract_suspension
        {
            0 => self.add_contract_suspension_v0(
                contract_id,
                identity_id,
                until,
                replaces_existing,
                moderator_id,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contract_suspension".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The low level operations of [`Drive::add_contract_suspension`]. With layer information the
    /// operations are built for estimation only.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity to suspend.
    /// * `until`: The block time, in milliseconds, at which the suspension lapses.
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
    pub fn add_contract_suspension_operations(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        until: TimestampMillis,
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
            .add_contract_suspension
        {
            0 => self.add_contract_suspension_operations_v0(
                contract_id,
                identity_id,
                until,
                replaces_existing,
                moderator_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_contract_suspension_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
