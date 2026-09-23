use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::group::GroupMemberPower;
use dpp::data_contract::GroupContractPosition;
use dpp::fee::fee_result::FeeResult;
use dpp::group::group_action::GroupAction;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds an action to the state and prices it, committing nothing until the price is
    /// known.
    ///
    /// Generation 1 differs from generation 0 in one thing: when the caller supplies no
    /// transaction and the operations are applied, the write and its pricing share one
    /// owned transaction that is committed only after `Drive::calculate_fee` succeeded.
    /// Generation 0 applied the batch (committing it on its own without a caller
    /// transaction) and priced it afterwards, so from protocol version 15, where pricing an
    /// owner-attributed storage removal without the fee history is an error, closing an
    /// action through this wrapper persisted the closure and then failed. The wrapper still
    /// passes no fee history, so such a close still fails at protocol version 15; it now
    /// fails before anything is written. Production closes actions through
    /// `apply_drive_operations`, which forwards the block's history. With a caller
    /// transaction nothing is committed by Drive in either generation.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_group_action_v1(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        initialize_with_insert_action_info: Option<GroupAction>,
        closes_group_action: bool,
        action_id: Identifier,
        signer_identity_id: Identifier,
        signer_power: GroupMemberPower,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let owned_transaction =
            (apply && transaction.is_none()).then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(transaction);

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.add_group_action_add_to_operations_v1(
            contract_id,
            group_contract_position,
            initialize_with_insert_action_info,
            closes_group_action,
            action_id,
            signer_identity_id,
            signer_power,
            block_info,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        // A pricing error drops the owned transaction with everything it wrote.
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }
        Ok(fees)
    }

    #[allow(clippy::too_many_arguments)]
    /// Adds group creation operations to drive operations
    pub(super) fn add_group_action_add_to_operations_v1(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        initialize_with_insert_action_info: Option<GroupAction>,
        closes_group_action: bool,
        action_id: Identifier,
        signer_identity_id: Identifier,
        signer_power: GroupMemberPower,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.add_group_action_operations(
            contract_id,
            group_contract_position,
            initialize_with_insert_action_info,
            closes_group_action,
            action_id,
            signer_identity_id,
            signer_power,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }
}
