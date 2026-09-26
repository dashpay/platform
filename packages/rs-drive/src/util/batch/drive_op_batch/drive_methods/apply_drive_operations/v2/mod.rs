use crate::util::batch::DriveOperation;

use crate::drive::Drive;
use crate::error::Error;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;

use crate::util::batch::drive_op_batch::drive_methods::apply_drive_operations::v1::forfeit_storage_refunds;
use crate::util::batch::drive_op_batch::finalize_task::{
    DriveOperationFinalizationTasks, DriveOperationFinalizeTask,
};
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use std::collections::HashMap;

impl Drive {
    /// Applies a list of high level DriveOperations to the drive, and calculates the fee for them.
    ///
    /// # Arguments
    ///
    /// * `operations` - A vector of `DriveOperation`s to apply to the drive.
    /// * `apply` - A boolean flag indicating whether to apply the changes or only estimate costs.
    /// * `block_info` - A reference to information about the current block.
    /// * `transaction` - Transaction arguments.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `FeeResult` if the operations are successfully applied,
    /// otherwise an `Error`.
    ///
    /// If `apply` is set to true, it applies the low-level drive operations and updates side info accordingly.
    /// If not, it only estimates the costs and updates estimated costs with layer info.
    ///
    /// Generation 1 (protocol version 14) is generation 0, and a batch that carries a storage
    /// refund forfeiture ([`DriveOperation::forfeits_storage_refunds`], a moderator's document
    /// deletion) refunds nobody: the bytes it removes still leave the system, but whoever paid
    /// for them gets nothing back, and the credits stay in the storage pools they were
    /// distributed to. An estimate carries no refund to begin with, so `check_tx` sees the
    /// same fee with or without the forfeiture.
    ///
    /// Generation 2 (protocol version 15) is generation 1 with the owned transaction held
    /// until the batch is priced. From protocol version 15 pricing an owner-attributed storage
    /// removal without the fee history is an error; generation 1 committed its owned
    /// transaction before pricing, so a caller passing no transaction and no history had its
    /// writes persisted and the error returned. Now `Drive::calculate_fee` runs first, a
    /// pricing error drops the owned transaction with everything it wrote, and the finalize
    /// tasks still run after the commit. With a caller transaction nothing is committed by
    /// Drive in either generation.
    #[inline(always)]
    pub(crate) fn apply_drive_operations_v2(
        &self,
        operations: Vec<DriveOperation>,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        if operations.is_empty() {
            return Ok(FeeResult::default());
        }
        let forfeits_storage_refunds = operations
            .iter()
            .any(DriveOperation::forfeits_storage_refunds);
        // With no caller transaction, TTL preparation (direct drainage
        // writes), conversion reads, and the batch apply would each commit
        // on their own, so a conversion error after preparation would leave
        // drained buckets committed without the write. Span all of it with
        // one owned transaction and commit only once the batch applied.
        let caller_transaction = transaction;
        let owned_transaction =
            (apply && transaction.is_none()).then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(caller_transaction);
        if apply {
            self.prepare_drive_operations_time_range_ttl(
                &operations,
                block_info,
                transaction,
                platform_version,
            )?;
        }
        let mut low_level_operations = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let mut finalize_tasks: Vec<DriveOperationFinalizeTask> = Vec::new();

        for drive_op in operations {
            if let Some(tasks) = drive_op.finalization_tasks(platform_version)? {
                finalize_tasks.extend(tasks);
            }

            low_level_operations.append(
                &mut drive_op.into_low_level_drive_operations_after_ttl_drain(
                    self,
                    &mut estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )?,
            );
        }

        let mut cost_operations = vec![];

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            low_level_operations,
            &mut cost_operations,
            &platform_version.drive,
        )?;
        if forfeits_storage_refunds {
            forfeit_storage_refunds(&mut cost_operations);
        }

        // Price before committing: a pricing error drops the owned transaction with
        // everything it wrote, so nothing is persisted without its fee result.
        let fee_result = Drive::calculate_fee(
            None,
            Some(cost_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            previous_fee_versions,
        )?;

        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }

        // Execute drive operation callbacks after updating state. Nothing was written when
        // only estimating, so there is nothing to finalize. The tasks read through the
        // caller's transaction; an owned one was committed just above, and `caller_transaction`
        // is `None` exactly then, so they read committed state.
        if apply {
            for task in finalize_tasks {
                task.execute(self, caller_transaction, platform_version)?;
            }
        }

        Ok(fee_result)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::batch::drive_op_batch::GroupOperationType;
    use crate::util::batch::DriveOperation;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::data_contract::DataContract;
    use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
    use dpp::group::action_event::GroupActionEvent;
    use dpp::group::group_action::v0::GroupActionV0;
    use dpp::group::group_action::GroupAction;
    use dpp::group::group_action_status::GroupActionStatus;
    use dpp::identifier::Identifier;
    use dpp::tokens::token_event::TokenEvent;
    use dpp::version::fee::FeeVersion;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// A contract with one two-member group and an action the first member opened,
    /// so closing the action moves signer-flagged items: the one removal a batch can
    /// carry without a document fixture.
    fn drive_with_open_group_action(
        platform_version: &PlatformVersion,
    ) -> (Drive, Identifier, Identifier, Identifier) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let member_1 = Identifier::from([1; 32]);
        let member_2 = Identifier::from([2; 32]);
        let contract = DataContract::V1(DataContractV1 {
            id: Identifier::from([3; 32]),
            version: 0,
            owner_id: member_1,
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::from([(
                0,
                Group::V0(GroupV0 {
                    members: [(member_1, 1), (member_2, 2)].into(),
                    required_power: 3,
                }),
            )]),
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });
        let contract_id = contract.id();
        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert the contract");
        let action_id = Identifier::from([4; 32]);
        let action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: member_1,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, member_1, None)),
        });
        drive
            .add_group_action(
                contract_id,
                0,
                Some(action),
                false,
                action_id,
                member_1,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to open the action");
        (drive, contract_id, member_2, action_id)
    }

    fn close_action(
        contract_id: Identifier,
        member_2: Identifier,
        action_id: Identifier,
    ) -> Vec<DriveOperation<'static>> {
        vec![DriveOperation::GroupOperation(
            GroupOperationType::AddGroupAction {
                contract_id,
                group_contract_position: 0,
                initialize_with_insert_action_info: None,
                action_id,
                signer_identity_id: member_2,
                signer_power: 2,
                closes_group_action: true,
            },
        )]
    }

    fn is_closed(
        drive: &Drive,
        contract_id: Identifier,
        action_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> bool {
        drive
            .fetch_action_is_closed(
                contract_id,
                0,
                action_id,
                true,
                None,
                &mut vec![],
                platform_version,
            )
            .expect("expected to check if the action is closed")
    }

    #[test]
    fn should_not_commit_an_owned_transaction_when_the_batch_cannot_be_priced() {
        // No caller transaction and no fee history: closing the action frees
        // signer-flagged bytes, which the latest generation refuses to price. The
        // batch was applied inside an owned transaction that is dropped with the
        // error, so the action is still open afterwards.
        let platform_version = PlatformVersion::latest();
        let (drive, contract_id, member_2, action_id) =
            drive_with_open_group_action(platform_version);

        let result = drive.apply_drive_operations(
            close_action(contract_id, member_2, action_id),
            true,
            &BlockInfo::default(),
            None,
            platform_version,
            None,
        );
        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
            ),
            "pricing a flagged removal without the fee history must fail, got {:?}",
            result
        );
        assert!(
            !is_closed(&drive, contract_id, action_id, platform_version),
            "a batch that could not be priced must not be committed"
        );

        // The same batch with the block's history is priced and committed.
        let history: CachedEpochIndexFeeVersions = BTreeMap::from([(0, FeeVersion::first())]);
        let fee_result = drive
            .apply_drive_operations(
                close_action(contract_id, member_2, action_id),
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                Some(&history),
            )
            .expect("expected to close the action with the fee history");
        assert!(fee_result.fee_refunds.get(&[1; 32]).is_some());
        assert!(is_closed(&drive, contract_id, action_id, platform_version));
        let closed_signers = drive
            .fetch_action_signers(
                contract_id,
                0,
                GroupActionStatus::ActionClosed,
                action_id,
                None,
                platform_version,
            )
            .expect("expected the closed signers");
        assert_eq!(closed_signers.len(), 2);
    }

    #[test]
    fn should_price_and_commit_without_history_under_the_frozen_generation() {
        // Protocol version 14 selects generation 1, which commits before pricing and
        // prices fee version number 1 without a history: the close persists.
        let platform_version = PlatformVersion::get(14).expect("protocol version 14");
        let (drive, contract_id, member_2, action_id) =
            drive_with_open_group_action(platform_version);

        drive
            .apply_drive_operations(
                close_action(contract_id, member_2, action_id),
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("the frozen generation prices the shipped shortcut without a history");
        assert!(is_closed(&drive, contract_id, action_id, platform_version));
    }
}
