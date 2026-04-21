use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_mint_many_v0(
        &self,
        token_id: Identifier,
        recipients: Vec<(Identifier, u64)>,
        issuance_amount: u64,
        allow_first_mint: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.token_mint_many_add_to_operations_v0(
            token_id,
            recipients,
            issuance_amount,
            allow_first_mint,
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
            None,
        )?;

        Ok(fees)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_mint_many_add_to_operations_v0(
        &self,
        token_id: Identifier,
        recipients: Vec<(Identifier, u64)>,
        issuance_amount: u64,
        allow_first_mint: bool,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.token_mint_many_operations_v0(
            token_id,
            recipients,
            issuance_amount,
            allow_first_mint,
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

    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_mint_many_operations_v0(
        &self,
        token_id: Identifier,
        mut recipients: Vec<(Identifier, u64)>,
        total_mint_amount: u64,
        allow_first_mint: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        let weight_sum = recipients
            .iter_mut()
            .map(|(_, weight)| {
                // We do this so we can't overflow
                if *weight > u32::MAX as u64 {
                    *weight = u32::MAX as u64
                }
                *weight
            })
            .sum::<u64>();
        let total_mint_amount_u128 = total_mint_amount as u128;

        let mut balance_left = total_mint_amount;

        for (i, (identity_id, weight)) in recipients.iter().enumerate() {
            let amount = if i == recipients.len() - 1 {
                balance_left
            } else {
                let amount = total_mint_amount_u128
                    .saturating_mul(*weight as u128)
                    .div_ceil(weight_sum as u128) as u64;
                balance_left -= amount;
                amount
            };

            drive_operations.extend(self.add_to_identity_token_balance_operations(
                token_id.to_buffer(),
                identity_id.to_buffer(),
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        // Update total supply

        drive_operations.extend(
            self.add_to_token_total_supply_operations(
                token_id.to_buffer(),
                total_mint_amount,
                allow_first_mint,
                false,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?
            .0,
        );

        Ok(drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::DataContract;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// Helper that creates a drive with a fresh token contract containing a
    /// single token at position 0, and returns the drive + token id.
    fn setup_drive_with_token() -> (crate::drive::Drive, [u8; 32]) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract = DataContract::V1(DataContractV1 {
            id: Default::default(),
            version: 0,
            owner_id: Default::default(),
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
            groups: Default::default(),
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(
                    TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
                ),
            )]),
            keywords: Vec::new(),
            description: None,
        });

        let token_id = contract
            .token_id(0)
            .expect("expected token at position 0")
            .to_buffer();

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        (drive, token_id)
    }

    /// Inserts a fresh random identity and returns its id buffer.
    fn insert_identity(drive: &crate::drive::Drive, seed: u64) -> [u8; 32] {
        let platform_version = PlatformVersion::latest();
        let identity = Identity::random_identity(3, Some(seed), platform_version)
            .expect("expected a platform identity");
        let id = identity.id().to_buffer();
        drive
            .add_new_identity(
                identity,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add identity");
        id
    }

    #[test]
    fn should_mint_many_to_multiple_recipients_with_proportional_weights() {
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 1);
        let id_b = insert_identity(&drive, 2);
        let id_c = insert_identity(&drive, 3);

        let total = 1000u64;
        let recipients = vec![
            (Identifier::new(id_a), 1),
            (Identifier::new(id_b), 2),
            (Identifier::new(id_c), 2),
        ];

        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                recipients,
                total,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected mint_many to succeed");

        let balance_a = drive
            .fetch_identity_token_balance(token_id, id_a, None, platform_version)
            .expect("fetch balance a")
            .expect("balance a exists");
        let balance_b = drive
            .fetch_identity_token_balance(token_id, id_b, None, platform_version)
            .expect("fetch balance b")
            .expect("balance b exists");
        let balance_c = drive
            .fetch_identity_token_balance(token_id, id_c, None, platform_version)
            .expect("fetch balance c")
            .expect("balance c exists");

        // Weight sum is 5. First two recipients use div_ceil(amount*weight, 5).
        // a: div_ceil(1000*1, 5) = 200, b: div_ceil(1000*2, 5) = 400, c: the
        // remainder = 1000 - 200 - 400 = 400.
        assert_eq!(balance_a, 200);
        assert_eq!(balance_b, 400);
        assert_eq!(balance_c, 400);

        // Confirm the total supply was updated.
        let total_supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("fetch total supply")
            .expect("total supply present");
        assert_eq!(total_supply, total);

        // All issued tokens must be fully distributed.
        assert_eq!(balance_a + balance_b + balance_c, total);
    }

    #[test]
    fn should_mint_many_to_single_recipient() {
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 10);
        let recipients = vec![(Identifier::new(id_a), 100)];

        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                recipients,
                5000,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected mint_many to succeed");

        let balance = drive
            .fetch_identity_token_balance(token_id, id_a, None, platform_version)
            .expect("fetch balance")
            .expect("balance present");
        assert_eq!(balance, 5000);

        let total_supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("fetch total supply")
            .expect("total supply present");
        assert_eq!(total_supply, 5000);
    }

    #[test]
    fn should_mint_many_when_some_recipients_have_weight_zero() {
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 20);
        let id_b = insert_identity(&drive, 21);
        let id_c = insert_identity(&drive, 22);

        // Weight sum of the non-zero entries is 10. B has weight 0 and should
        // receive nothing. C is last, so it takes the remainder.
        let total = 1000u64;
        let recipients = vec![
            (Identifier::new(id_a), 3),
            (Identifier::new(id_b), 0),
            (Identifier::new(id_c), 7),
        ];

        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                recipients,
                total,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected mint_many to succeed");

        let balance_a = drive
            .fetch_identity_token_balance(token_id, id_a, None, platform_version)
            .expect("fetch balance a")
            .expect("balance a exists");
        let balance_b = drive
            .fetch_identity_token_balance(token_id, id_b, None, platform_version)
            .expect("fetch balance b")
            .expect("balance b exists");
        let balance_c = drive
            .fetch_identity_token_balance(token_id, id_c, None, platform_version)
            .expect("fetch balance c")
            .expect("balance c exists");

        // a = div_ceil(1000*3, 10) = 300, b = div_ceil(1000*0, 10) = 0,
        // c (last) = 1000 - 300 - 0 = 700.
        assert_eq!(balance_a, 300);
        assert_eq!(balance_b, 0);
        assert_eq!(balance_c, 700);
        assert_eq!(balance_a + balance_b + balance_c, total);
    }

    #[test]
    fn should_clamp_recipient_weight_above_u32_max_to_u32_max() {
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 30);
        let id_b = insert_identity(&drive, 31);

        // a's requested weight exceeds u32::MAX. Internally it must be
        // clamped to u32::MAX, which means both recipients end up with equal
        // effective weight so the split should be ~50/50 (remainder to last).
        let total = 1000u64;
        let huge_weight = (u32::MAX as u64) + 1_000_000;
        let recipients = vec![
            (Identifier::new(id_a), huge_weight),
            (Identifier::new(id_b), u32::MAX as u64),
        ];

        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                recipients,
                total,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected mint_many to succeed");

        let balance_a = drive
            .fetch_identity_token_balance(token_id, id_a, None, platform_version)
            .expect("fetch balance a")
            .expect("balance a exists");
        let balance_b = drive
            .fetch_identity_token_balance(token_id, id_b, None, platform_version)
            .expect("fetch balance b")
            .expect("balance b exists");

        // Effective weight sum after clamp = 2 * u32::MAX. First recipient:
        // div_ceil(1000 * u32::MAX, 2 * u32::MAX) = 500. Second (last) gets
        // the remainder = 500.
        assert_eq!(balance_a, 500);
        assert_eq!(balance_b, 500);
        assert_eq!(balance_a + balance_b, total);
    }

    #[test]
    fn should_update_total_supply_across_multiple_mint_many_calls() {
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 40);
        let id_b = insert_identity(&drive, 41);

        // First mint creates the supply entry (allow_first_mint = true).
        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                vec![(Identifier::new(id_a), 1), (Identifier::new(id_b), 1)],
                200,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("first mint");

        let after_first = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("fetch total supply")
            .expect("total supply present");
        assert_eq!(after_first, 200);

        // Subsequent mints should not require allow_first_mint because the
        // supply entry already exists.
        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                vec![(Identifier::new(id_a), 3), (Identifier::new(id_b), 1)],
                400,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("second mint");

        let after_second = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("fetch total supply")
            .expect("total supply present");
        assert_eq!(after_second, 600);

        // Balances should reflect the sum of the two mints.
        // First mint: weight sum 2, total 200 -> a = 100, b = 100.
        // Second mint: weight sum 4, total 400 -> a = div_ceil(400*3, 4) = 300,
        //              b (last) = 400 - 300 = 100.
        let balance_a = drive
            .fetch_identity_token_balance(token_id, id_a, None, platform_version)
            .expect("fetch balance a")
            .expect("balance a exists");
        let balance_b = drive
            .fetch_identity_token_balance(token_id, id_b, None, platform_version)
            .expect("fetch balance b")
            .expect("balance b exists");
        assert_eq!(balance_a, 400);
        assert_eq!(balance_b, 200);
        assert_eq!(balance_a + balance_b, after_second);
    }

    #[test]
    fn should_succeed_with_allow_first_mint_false_when_supply_already_exists() {
        // A contract insert pre-creates the per-token total-supply entry.
        // Once the entry exists, mint_many with allow_first_mint=false must
        // still succeed because it's an "add to existing supply" call.
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 50);

        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                vec![(Identifier::new(id_a), 1)],
                100,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("mint_many with allow_first_mint=false must succeed when supply exists");

        let balance = drive
            .fetch_identity_token_balance(token_id, id_a, None, platform_version)
            .expect("fetch balance")
            .expect("balance present");
        assert_eq!(balance, 100);
    }

    #[test]
    fn should_mint_many_amount_zero_leaves_supply_and_balances_zero() {
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 60);
        let id_b = insert_identity(&drive, 61);

        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                vec![(Identifier::new(id_a), 1), (Identifier::new(id_b), 1)],
                0,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected mint_many of 0 to succeed");

        let balance_a = drive
            .fetch_identity_token_balance(token_id, id_a, None, platform_version)
            .expect("fetch balance a");
        let balance_b = drive
            .fetch_identity_token_balance(token_id, id_b, None, platform_version)
            .expect("fetch balance b");

        // Minting zero should either leave the per-identity balance absent
        // or set it to zero. Either is acceptable; a positive balance would
        // be a bug.
        assert!(balance_a.unwrap_or(0) == 0);
        assert!(balance_b.unwrap_or(0) == 0);

        let supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("fetch supply");
        assert!(supply.unwrap_or(0) == 0);
    }

    #[test]
    fn should_mint_many_when_recipient_identity_does_not_exist() {
        // Drive-level mint_many does not verify that recipients are existing
        // identities — that check is higher up the stack. The raw operation
        // should still succeed and create the balance entry.
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let stranger = Identifier::new([0xAB; 32]);

        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                vec![(stranger, 1)],
                777,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected mint_many to succeed even without a registered identity");

        let supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("fetch supply")
            .expect("supply present");
        assert_eq!(supply, 777);
    }

    #[test]
    fn add_to_operations_v0_populates_drive_operations() {
        // Exercises the token_mint_many_add_to_operations_v0 entry point
        // directly (the v0 fn that takes an out-parameter for drive ops).
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 70);

        let mut drive_operations = Vec::new();
        drive
            .token_mint_many_add_to_operations_v0(
                Identifier::new(token_id),
                vec![(Identifier::new(id_a), 1)],
                123,
                true,
                true,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected add_to_operations to succeed");

        // The call populates low-level drive operations that were applied.
        assert!(
            !drive_operations.is_empty(),
            "expected drive_operations to be populated after applying the batch"
        );

        let balance = drive
            .fetch_identity_token_balance(token_id, id_a, None, platform_version)
            .expect("fetch balance")
            .expect("balance present");
        assert_eq!(balance, 123);
    }

    #[test]
    fn operations_v0_returns_ops_without_applying_state() {
        // token_mint_many_operations_v0 returns the list of pending ops
        // without applying them. With apply=false (None transaction) state
        // must remain unchanged until apply_batch is called.
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 80);

        // Capture the supply as it exists immediately after contract insert
        // (base_supply=0 means the entry exists at value 0).
        let supply_before = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("fetch supply before")
            .unwrap_or(0);

        let mut estimated_costs_only_with_layer_info = Some(std::collections::HashMap::new());
        let ops = drive
            .token_mint_many_operations_v0(
                Identifier::new(token_id),
                vec![(Identifier::new(id_a), 1)],
                999,
                true,
                &mut estimated_costs_only_with_layer_info,
                None,
                platform_version,
            )
            .expect("expected operations to be produced");

        assert!(
            !ops.is_empty(),
            "expected mint_many_operations_v0 to produce at least one op"
        );

        // Supply must not have advanced by 999 — operations were produced
        // but never applied.
        let supply_after = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("fetch supply after")
            .unwrap_or(0);
        assert_eq!(
            supply_before, supply_after,
            "operations_v0 should not mutate on-disk supply"
        );

        // Balance likewise should be absent or unchanged.
        let balance = drive
            .fetch_identity_token_balance(token_id, id_a, None, platform_version)
            .expect("fetch balance")
            .unwrap_or(0);
        assert_eq!(balance, 0, "balance must not have been applied");
    }

    #[test]
    fn should_mint_many_with_many_recipients_and_conserve_total() {
        // A stress-style test: ensure that across several recipients with
        // varied weights the sum of distributed balances exactly equals
        // the issuance amount (the last recipient takes the rounding
        // remainder).
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let ids: Vec<[u8; 32]> = (0..7).map(|i| insert_identity(&drive, 100 + i)).collect();
        let weights: Vec<u64> = vec![1, 2, 3, 4, 5, 6, 7];
        let total = 12_345u64;

        let recipients: Vec<(Identifier, u64)> = ids
            .iter()
            .zip(weights.iter())
            .map(|(id, w)| (Identifier::new(*id), *w))
            .collect();

        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                recipients,
                total,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected mint_many to succeed");

        let mut sum = 0u64;
        for id in &ids {
            let balance = drive
                .fetch_identity_token_balance(token_id, *id, None, platform_version)
                .expect("fetch balance")
                .unwrap_or(0);
            sum += balance;
        }
        assert_eq!(
            sum, total,
            "sum of all recipient balances must equal issuance amount"
        );

        let supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("fetch supply")
            .expect("supply present");
        assert_eq!(supply, total);
    }

    #[test]
    fn should_mint_many_with_equal_weights_distributes_evenly_with_last_absorbing_remainder() {
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 200);
        let id_b = insert_identity(&drive, 201);
        let id_c = insert_identity(&drive, 202);

        // 100 / 3 does not divide evenly. Per div_ceil, the first two get
        // 34, and the remainder falls to the last recipient.
        let total = 100u64;
        drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                vec![
                    (Identifier::new(id_a), 1),
                    (Identifier::new(id_b), 1),
                    (Identifier::new(id_c), 1),
                ],
                total,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("mint_many ok");

        let balance_a = drive
            .fetch_identity_token_balance(token_id, id_a, None, platform_version)
            .expect("a")
            .expect("a exists");
        let balance_b = drive
            .fetch_identity_token_balance(token_id, id_b, None, platform_version)
            .expect("b")
            .expect("b exists");
        let balance_c = drive
            .fetch_identity_token_balance(token_id, id_c, None, platform_version)
            .expect("c")
            .expect("c exists");

        assert_eq!(balance_a, 34);
        assert_eq!(balance_b, 34);
        assert_eq!(balance_c, 32);
        assert_eq!(balance_a + balance_b + balance_c, total);
    }

    #[test]
    fn should_mint_many_fee_result_is_non_zero_on_apply() {
        // Exercises the token_mint_many_v0 path that computes a FeeResult.
        let (drive, token_id) = setup_drive_with_token();
        let platform_version = PlatformVersion::latest();

        let id_a = insert_identity(&drive, 300);

        let fee_result = drive
            .token_mint_many_v0(
                Identifier::new(token_id),
                vec![(Identifier::new(id_a), 1)],
                500,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("mint_many ok");

        // Applying writes should cost something in processing and/or storage.
        assert!(
            fee_result.processing_fee > 0 || fee_result.storage_fee > 0,
            "expected non-zero fee for applied mint_many, got {:?}",
            fee_result
        );
    }
}
