use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::GroupContractPosition;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;

impl Drive {
    /// Fetches if an identity has already signed in an action
    pub fn fetch_action_id_has_signer(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        signer_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_action_id_has_signer
        {
            0 => self.fetch_action_id_has_signer_v0(
                contract_id,
                group_contract_position,
                action_id,
                signer_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_id_has_signer".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches if an identity has already signed in an action with costs
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_action_id_has_signer_with_costs(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        signer_id: Identifier,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(bool, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_action_id_has_signer
        {
            0 => self.fetch_action_id_has_signer_with_costs_v0(
                contract_id,
                group_contract_position,
                action_id,
                signer_id,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_id_has_signer_with_costs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    #[allow(clippy::too_many_arguments)]
    // TODO: Is not using
    #[allow(dead_code)]
    pub(crate) fn fetch_action_id_has_signer_and_add_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        signer_id: Identifier,
        estimate_costs_only: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_action_id_has_signer
        {
            0 => self.fetch_action_id_has_signer_and_add_operations_v0(
                contract_id,
                group_contract_position,
                action_id,
                signer_id,
                estimate_costs_only,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_id_has_signer_and_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
