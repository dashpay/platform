use crate::util::batch::DriveOperation;

use crate::drive::Drive;
use crate::error::Error;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;

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
    #[inline(always)]
    pub(crate) fn apply_drive_operations_v0(
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

            let mut lowered = drive_op.into_low_level_drive_operations(
                self,
                &mut Some(&mut low_level_operations),
                &mut estimated_costs_only_with_layer_info,
                block_info,
                transaction,
                platform_version,
            )?;
            low_level_operations.append(&mut lowered);
        }

        let mut cost_operations = vec![];

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            low_level_operations,
            &mut cost_operations,
            &platform_version.drive,
        )?;

        // Execute drive operation callbacks after updating state
        for task in finalize_tasks {
            task.execute(self, platform_version)?;
        }

        Drive::calculate_fee(
            None,
            Some(cost_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            previous_fee_versions,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::util::batch::drive_op_batch::TokenOperationType;
    use crate::util::batch::DriveOperation::TokenOperation;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
    use dpp::version::PlatformVersion;

    /// Two tokens of one issuer, each with 1_000 in supply held by one identity.
    fn drive_with_two_tokens_of_one_issuer() -> (crate::drive::Drive, Identifier, [[u8; 32]; 2]) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::from([3u8; 32]);
        let tokens = [[1u8; 32], [2u8; 32]];
        for (position, token_id) in tokens.iter().enumerate() {
            drive
                .create_token_trees(
                    contract_id,
                    position as u16,
                    *token_id,
                    false,
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to create token trees");
            drive
                .token_mint(
                    *token_id,
                    [9u8; 32],
                    1_000,
                    false,
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to mint");
        }
        (drive, contract_id, tokens)
    }

    fn rollup(drive: &crate::drive::Drive, contract_id: Identifier) -> u128 {
        drive
            .fetch_contract_token_lifecycle(
                contract_id.to_buffer(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to read")
            .expect("expected a record")
            .issued_supply()
    }

    /// A batch is lowered in full before it is applied, so supply writes for two tokens of
    /// one issuer would each read the stored record and emit two replacements of the same
    /// key; the batch consistency check of the test drive rejects that. The batch handed
    /// down through lowering folds them into one.
    #[test]
    fn should_apply_supply_changes_of_two_tokens_of_one_issuer_in_one_batch() {
        let (drive, contract_id, [token_a, token_b]) = drive_with_two_tokens_of_one_issuer();
        let platform_version = PlatformVersion::latest();
        let holder = Identifier::from([9u8; 32]);
        let recipient = Identifier::from([8u8; 32]);

        // Distinct tokens only: two supply writes of the same token in one batch would also
        // write the token's supply leaf and holder leaf twice, which is the batch-wide
        // absolute-write hazard of every balance path and not what the issuer record adds.
        drive
            .apply_drive_operations(
                vec![
                    TokenOperation(TokenOperationType::TokenMint {
                        token_id: Identifier::from(token_a),
                        identity_balance_holder_id: holder,
                        mint_amount: 100,
                        allow_first_mint: false,
                        allow_saturation: false,
                    }),
                    TokenOperation(TokenOperationType::TokenMintMany {
                        token_id: Identifier::from(token_b),
                        recipients: vec![(holder, 1), (recipient, 1)],
                        mint_amount: 200,
                        allow_first_mint: false,
                    }),
                ],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected the mint batch to apply");

        assert_eq!(rollup(&drive, contract_id), 2_000 + 100 + 200);
        assert_eq!(
            drive
                .fetch_token_total_supply(token_a, None, platform_version)
                .expect("expected the supply"),
            Some(1_100)
        );
        assert_eq!(
            drive
                .fetch_token_total_supply(token_b, None, platform_version)
                .expect("expected the supply"),
            Some(1_200)
        );

        drive
            .apply_drive_operations(
                vec![
                    TokenOperation(TokenOperationType::TokenBurn {
                        token_id: Identifier::from(token_a),
                        identity_balance_holder_id: holder,
                        burn_amount: 50,
                    }),
                    TokenOperation(TokenOperationType::TokenBurn {
                        token_id: Identifier::from(token_b),
                        identity_balance_holder_id: recipient,
                        burn_amount: 25,
                    }),
                ],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected the burn batch to apply");

        assert_eq!(rollup(&drive, contract_id), 2_300 - 50 - 25);
        drive.assert_token_rollups_consistent(None, platform_version);
    }

    /// The same composition priced without state stays free of state and covers the applied
    /// fee, the invariant block execution relies on.
    #[test]
    fn should_estimate_a_batch_of_two_tokens_of_one_issuer_above_the_applied_fee() {
        let (drive, contract_id, [token_a, token_b]) = drive_with_two_tokens_of_one_issuer();
        let platform_version = PlatformVersion::latest();
        let holder = Identifier::from([9u8; 32]);
        let operations = || {
            vec![
                TokenOperation(TokenOperationType::TokenMint {
                    token_id: Identifier::from(token_a),
                    identity_balance_holder_id: holder,
                    mint_amount: 100,
                    allow_first_mint: false,
                    allow_saturation: false,
                }),
                TokenOperation(TokenOperationType::TokenBurn {
                    token_id: Identifier::from(token_b),
                    identity_balance_holder_id: holder,
                    burn_amount: 30,
                }),
            ]
        };

        let estimated = drive
            .apply_drive_operations(
                operations(),
                false,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected an estimate");
        assert_eq!(rollup(&drive, contract_id), 2_000);

        let applied = drive
            .apply_drive_operations(
                operations(),
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected to apply");
        assert_eq!(rollup(&drive, contract_id), 2_070);
        assert!(
            estimated.total_base_fee() >= applied.total_base_fee(),
            "estimated total {} is below applied total {}",
            estimated.total_base_fee(),
            applied.total_base_fee()
        );
    }

    /// Two issuers in one batch keep separate records.
    #[test]
    fn should_keep_the_records_of_two_issuers_apart_in_one_batch() {
        let (drive, contract_id, [token_a, _]) = drive_with_two_tokens_of_one_issuer();
        let platform_version = PlatformVersion::latest();
        let other_contract = Identifier::from([4u8; 32]);
        let other_token = [5u8; 32];
        drive
            .create_token_trees(
                other_contract,
                0,
                other_token,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");
        let holder = Identifier::from([9u8; 32]);

        drive
            .apply_drive_operations(
                vec![
                    TokenOperation(TokenOperationType::TokenMint {
                        token_id: Identifier::from(token_a),
                        identity_balance_holder_id: holder,
                        mint_amount: 7,
                        allow_first_mint: false,
                        allow_saturation: false,
                    }),
                    TokenOperation(TokenOperationType::TokenMint {
                        token_id: Identifier::from(other_token),
                        identity_balance_holder_id: holder,
                        mint_amount: 11,
                        allow_first_mint: false,
                        allow_saturation: false,
                    }),
                ],
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("expected the batch to apply");

        assert_eq!(rollup(&drive, contract_id), 2_007);
        assert_eq!(rollup(&drive, other_contract), 11);
        drive.assert_token_rollups_consistent(None, platform_version);
    }
}
