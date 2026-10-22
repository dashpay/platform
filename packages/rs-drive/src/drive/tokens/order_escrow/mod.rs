//! The escrow of the token marketplace (protocol version 15).
//!
//! An open listing holds the tokens it offers in `[Tokens, TOKEN_ORDER_ESCROW_KEY, token id,
//! order id]`, a SumItem of the token's escrow SumTree, and an open offer holds the credits it
//! offers in `[PreFundedSpecializedBalances, PREFUNDED_BALANCES_FOR_TOKEN_ORDERS, order id]`. Both
//! roots are inserted by `insert_token_order_escrow_trees`.

mod credits;
mod estimated_costs;
mod insert_trees;
mod tokens;

#[cfg(test)]
mod tests {
    use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_path;
    use crate::drive::tokens::paths::tokens_root_path;
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::document_type::action_fees::ContractFeePot;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;
    use grovedb::batch::KeyInfoPath;
    use grovedb::EstimatedLayerCount::EstimatedLevel;
    use grovedb::EstimatedLayerSizes::AllSubtrees;
    use grovedb::EstimatedSumTrees::AllBigSumTrees;
    use grovedb::{EstimatedLayerInformation, TreeType};
    use std::collections::HashMap;

    fn apply(drive: &Drive, operations: Vec<LowLevelDriveOperation>) {
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("expected to apply the operations");
    }

    #[test]
    fn should_hold_set_and_release_what_a_listing_escrows() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let token_id = [5; 32];
        let order_id = [9; 32];
        let fetch = |drive: &Drive, token_id: [u8; 32]| {
            drive
                .fetch_token_order_escrow(token_id, order_id, None, &mut vec![], platform_version)
                .expect("expected to fetch the escrow")
        };

        // A token that never had a listing has no escrow tree, and nothing in escrow.
        assert_eq!(fetch(&drive, token_id), None);

        apply(
            &drive,
            drive
                .add_to_token_order_escrow_operations(
                    token_id,
                    order_id,
                    500,
                    &mut None,
                    None,
                    &mut None,
                    platform_version,
                )
                .expect("expected the escrow operations"),
        );
        assert_eq!(fetch(&drive, token_id), Some(500));
        assert_eq!(fetch(&drive, [6; 32]), None);

        apply(
            &drive,
            drive
                .set_token_order_escrow_operations(
                    token_id,
                    order_id,
                    200,
                    &mut None,
                    platform_version,
                )
                .expect("expected the escrow operations"),
        );
        assert_eq!(fetch(&drive, token_id), Some(200));

        apply(
            &drive,
            drive
                .remove_token_order_escrow_operations(
                    token_id,
                    order_id,
                    &mut None,
                    platform_version,
                )
                .expect("expected the escrow operations"),
        );
        assert_eq!(fetch(&drive, token_id), None);
    }

    #[test]
    fn should_hold_set_and_release_what_an_offer_escrows() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let order_id = [9; 32];
        let fetch = |drive: &Drive| {
            drive
                .fetch_credit_order_escrow(order_id, None, &mut vec![], platform_version)
                .expect("expected to fetch the escrow")
        };

        assert_eq!(fetch(&drive), None);
        apply(
            &drive,
            drive
                .add_to_credit_order_escrow_operations(
                    order_id,
                    70_000,
                    &mut None,
                    platform_version,
                )
                .expect("expected the escrow operations"),
        );
        assert_eq!(fetch(&drive), Some(70_000));
        apply(
            &drive,
            drive
                .set_credit_order_escrow_operations(order_id, 30_000, &mut None, platform_version)
                .expect("expected the escrow operations"),
        );
        assert_eq!(fetch(&drive), Some(30_000));
        apply(
            &drive,
            drive
                .remove_credit_order_escrow_operations(order_id, &mut None, platform_version)
                .expect("expected the escrow operations"),
        );
        assert_eq!(fetch(&drive), None);
    }

    /// Listing moves tokens from a balance into escrow: the supply is unchanged, and the token
    /// conservation check of protocol version 15 counts the escrow. The calculator of protocol
    /// version 14, which reads no escrow, would see tokens missing.
    #[test]
    fn should_keep_the_token_supply_balanced_when_tokens_move_into_escrow() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let token_id = [5; 32];
        let identity_id = [6; 32];
        let block_info = BlockInfo::default();

        drive
            .create_token_trees(
                Identifier::from([4; 32]),
                0,
                token_id,
                false,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to create the token trees");
        drive
            .token_mint(
                token_id,
                identity_id,
                1000,
                true,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to mint");

        let mut operations = drive
            .remove_from_identity_token_balance_operations(
                token_id,
                identity_id,
                400,
                &mut None,
                None,
                platform_version,
            )
            .expect("expected the balance operations");
        operations.extend(
            drive
                .add_to_token_order_escrow_operations(
                    token_id,
                    [9; 32],
                    400,
                    &mut None,
                    None,
                    &mut None,
                    platform_version,
                )
                .expect("expected the escrow operations"),
        );
        apply(&drive, operations);

        let total = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected to calculate the total tokens balance");
        assert_eq!(total.total_tokens_in_platform, 1000);
        assert_eq!(total.total_identity_token_balances, 600);
        assert_eq!(total.total_token_order_escrow_balances, 400);
        assert!(total.ok().expect("expected no overflow"));

        let before_the_marketplace = drive
            .calculate_total_tokens_balance(
                None,
                PlatformVersion::get(14).expect("expected protocol version 14"),
            )
            .expect("expected to calculate the total tokens balance");
        assert!(!before_the_marketplace.ok().expect("expected no overflow"));
    }

    fn fetch_listing(drive: &Drive, token_id: [u8; 32], order_id: [u8; 32]) -> Option<u64> {
        drive
            .fetch_token_order_escrow(
                token_id,
                order_id,
                None,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the escrow")
    }

    fn fetch_offer(drive: &Drive, order_id: [u8; 32]) -> Option<u64> {
        drive
            .fetch_credit_order_escrow(order_id, None, &mut vec![], PlatformVersion::latest())
            .expect("expected to fetch the escrow")
    }

    /// Two listings of a token that has none, built for one batch: the second sees the escrow
    /// tree the first queues and does not insert it again.
    #[test]
    fn should_insert_a_tokens_escrow_tree_once_for_two_listings_in_one_batch() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let token_id = [5; 32];

        let mut batch = drive
            .add_to_token_order_escrow_operations(
                token_id,
                [1; 32],
                10,
                &mut None,
                None,
                &mut None,
                platform_version,
            )
            .expect("expected the first listing's operations");
        let second = drive
            .add_to_token_order_escrow_operations(
                token_id,
                [2; 32],
                20,
                &mut None,
                None,
                &mut Some(&mut batch),
                platform_version,
            )
            .expect("expected the second listing's operations");
        batch.extend(second);
        apply(&drive, batch);

        assert_eq!(fetch_listing(&drive, token_id, [1; 32]), Some(10));
        assert_eq!(fetch_listing(&drive, token_id, [2; 32]), Some(20));
    }

    /// Without the operations queued before it, the second listing inserts the tree again, and
    /// the batch is refused: what the shared operations prevent.
    #[test]
    fn should_refuse_a_batch_inserting_a_tokens_escrow_tree_twice() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let token_id = [5; 32];
        let mut batch = vec![];
        for order_id in [[1; 32], [2; 32]] {
            batch.extend(
                drive
                    .add_to_token_order_escrow_operations(
                        token_id,
                        order_id,
                        10,
                        &mut None,
                        None,
                        &mut None,
                        platform_version,
                    )
                    .expect("expected the listing's operations"),
            );
        }
        assert!(drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                batch,
                &mut vec![],
                &platform_version.drive,
            )
            .is_err());
    }

    /// Every escrow write can be priced without state: the operations built for estimation
    /// cost something to apply and change nothing.
    #[test]
    fn should_estimate_every_escrow_write_without_writing_it() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let token_id = [5; 32];
        let order_id = [9; 32];

        type Build<'a> = Box<
            dyn Fn(
                    &mut Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>,
                ) -> Vec<LowLevelDriveOperation>
                + 'a,
        >;
        let writes: Vec<(&str, Build)> = vec![
            (
                "add a listing (the token's first)",
                Box::new(|estimation| {
                    drive
                        .add_to_token_order_escrow_operations(
                            token_id,
                            order_id,
                            500,
                            estimation,
                            None,
                            &mut None,
                            platform_version,
                        )
                        .expect("expected the operations")
                }),
            ),
            (
                "set a listing",
                Box::new(|estimation| {
                    drive
                        .set_token_order_escrow_operations(
                            token_id,
                            order_id,
                            200,
                            estimation,
                            platform_version,
                        )
                        .expect("expected the operations")
                }),
            ),
            (
                "remove a listing",
                Box::new(|estimation| {
                    drive
                        .remove_token_order_escrow_operations(
                            token_id,
                            order_id,
                            estimation,
                            platform_version,
                        )
                        .expect("expected the operations")
                }),
            ),
            (
                "add an offer",
                Box::new(|estimation| {
                    drive
                        .add_to_credit_order_escrow_operations(
                            order_id,
                            70_000,
                            estimation,
                            platform_version,
                        )
                        .expect("expected the operations")
                }),
            ),
            (
                "set an offer",
                Box::new(|estimation| {
                    drive
                        .set_credit_order_escrow_operations(
                            order_id,
                            30_000,
                            estimation,
                            platform_version,
                        )
                        .expect("expected the operations")
                }),
            ),
            (
                "remove an offer",
                Box::new(|estimation| {
                    drive
                        .remove_credit_order_escrow_operations(
                            order_id,
                            estimation,
                            platform_version,
                        )
                        .expect("expected the operations")
                }),
            ),
        ];

        for (name, build) in writes {
            let mut estimation = Some(HashMap::new());
            let operations = build(&mut estimation);
            let mut costs = vec![];
            drive
                .apply_batch_low_level_drive_operations(
                    estimation,
                    None,
                    operations,
                    &mut costs,
                    &platform_version.drive,
                )
                .unwrap_or_else(|error| panic!("{name}: expected to estimate: {error:?}"));
            let fee = Drive::calculate_fee(
                None,
                Some(costs),
                &BlockInfo::default().epoch,
                drive.config.epochs_per_era,
                platform_version,
                None,
            )
            .unwrap_or_else(|error| panic!("{name}: expected a fee: {error:?}"));
            assert!(
                fee.processing_fee > 0,
                "{name}: an estimated write costs something"
            );
            assert_eq!(fetch_listing(&drive, token_id, order_id), None, "{name}");
            assert_eq!(fetch_offer(&drive, order_id), None, "{name}");
        }
    }

    /// The escrow estimation describes the Tokens layer at the escrow key's depth, and keeps a
    /// deeper description already registered for the same batch.
    #[test]
    fn should_keep_the_deeper_tokens_layer_estimate() {
        let platform_version = PlatformVersion::latest();
        let tokens_layer_level =
            |estimation: &HashMap<KeyInfoPath, EstimatedLayerInformation>| match estimation
                .get(&KeyInfoPath::from_known_path(tokens_root_path()))
                .map(|layer| &layer.estimated_layer_count)
            {
                Some(EstimatedLevel(level, _)) => *level,
                other => panic!("expected an estimated level, got {other:?}"),
            };

        // After the token balance estimation (level 0, the balances key), the escrow's is kept.
        let mut estimation = HashMap::new();
        Drive::add_estimation_costs_for_token_balances(
            [5; 32],
            &mut estimation,
            &platform_version.drive,
        )
        .expect("expected the balance estimation");
        assert_eq!(tokens_layer_level(&estimation), 0);
        Drive::add_estimation_costs_for_token_order_escrow(
            [5; 32],
            &mut estimation,
            platform_version,
        )
        .expect("expected the escrow estimation");
        assert_eq!(tokens_layer_level(&estimation), 3);

        // A deeper description stays.
        let mut estimation = HashMap::from([(
            KeyInfoPath::from_known_path(tokens_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(5, false),
                estimated_layer_sizes: AllSubtrees(1, AllBigSumTrees, None),
            },
        )]);
        Drive::add_estimation_costs_for_token_order_escrow(
            [5; 32],
            &mut estimation,
            platform_version,
        )
        .expect("expected the escrow estimation");
        assert_eq!(tokens_layer_level(&estimation), 5);
    }

    /// Both escrows hold signed 8-byte values: `i64::MAX` is held, one more is refused.
    #[test]
    fn should_hold_up_to_i64_max_in_either_escrow() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let token_id = [5; 32];
        let largest = i64::MAX as u64;

        apply(
            &drive,
            drive
                .add_to_token_order_escrow_operations(
                    token_id,
                    [1; 32],
                    largest,
                    &mut None,
                    None,
                    &mut None,
                    platform_version,
                )
                .expect("expected i64::MAX to be held"),
        );
        assert_eq!(fetch_listing(&drive, token_id, [1; 32]), Some(largest));
        assert!(drive
            .add_to_token_order_escrow_operations(
                token_id,
                [2; 32],
                largest + 1,
                &mut None,
                None,
                &mut None,
                platform_version,
            )
            .is_err());
        assert!(drive
            .set_token_order_escrow_operations(
                token_id,
                [1; 32],
                largest + 1,
                &mut None,
                platform_version,
            )
            .is_err());

        apply(
            &drive,
            drive
                .add_to_credit_order_escrow_operations(
                    [3; 32],
                    largest,
                    &mut None,
                    platform_version,
                )
                .expect("expected i64::MAX to be held"),
        );
        assert_eq!(fetch_offer(&drive, [3; 32]), Some(largest));
        assert!(drive
            .add_to_credit_order_escrow_operations(
                [4; 32],
                largest + 1,
                &mut None,
                platform_version
            )
            .is_err());
        assert!(drive
            .set_credit_order_escrow_operations([3; 32], largest + 1, &mut None, platform_version)
            .is_err());
    }

    /// The escrow methods are active from protocol version 15: at 14 every one of them refuses
    /// with `VersionNotActive` instead of building operations for trees that do not exist there.
    #[test]
    fn should_refuse_every_escrow_method_before_protocol_version_15() {
        let latest = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(latest));
        let token_id = [5; 32];
        let order_id = [9; 32];

        let calls = |platform_version: &PlatformVersion| -> Vec<(&str, Result<(), Error>)> {
            let mut estimation = HashMap::new();
            vec![
                (
                    "add_tokens",
                    drive
                        .add_to_token_order_escrow_operations(
                            token_id,
                            order_id,
                            500,
                            &mut None,
                            None,
                            &mut None,
                            platform_version,
                        )
                        .map(|_| ()),
                ),
                (
                    "set_tokens",
                    drive
                        .set_token_order_escrow_operations(
                            token_id,
                            order_id,
                            200,
                            &mut None,
                            platform_version,
                        )
                        .map(|_| ()),
                ),
                (
                    "remove_tokens",
                    drive
                        .remove_token_order_escrow_operations(
                            token_id,
                            order_id,
                            &mut None,
                            platform_version,
                        )
                        .map(|_| ()),
                ),
                (
                    "fetch_tokens",
                    drive
                        .fetch_token_order_escrow(
                            token_id,
                            order_id,
                            None,
                            &mut vec![],
                            platform_version,
                        )
                        .map(|_| ()),
                ),
                (
                    "add_credits",
                    drive
                        .add_to_credit_order_escrow_operations(
                            order_id,
                            70_000,
                            &mut None,
                            platform_version,
                        )
                        .map(|_| ()),
                ),
                (
                    "set_credits",
                    drive
                        .set_credit_order_escrow_operations(
                            order_id,
                            30_000,
                            &mut None,
                            platform_version,
                        )
                        .map(|_| ()),
                ),
                (
                    "remove_credits",
                    drive
                        .remove_credit_order_escrow_operations(
                            order_id,
                            &mut None,
                            platform_version,
                        )
                        .map(|_| ()),
                ),
                (
                    "fetch_credits",
                    drive
                        .fetch_credit_order_escrow(order_id, None, &mut vec![], platform_version)
                        .map(|_| ()),
                ),
                (
                    "estimated_costs_for_tokens",
                    Drive::add_estimation_costs_for_token_order_escrow(
                        token_id,
                        &mut estimation,
                        platform_version,
                    ),
                ),
                (
                    "estimated_costs_for_credits",
                    Drive::add_estimation_costs_for_credit_order_escrow(
                        &mut estimation,
                        platform_version,
                    ),
                ),
            ]
        };

        let before = PlatformVersion::get(14).expect("expected protocol version 14");
        for (name, result) in calls(before) {
            assert!(
                matches!(
                    result,
                    Err(Error::Drive(DriveError::VersionNotActive { .. }))
                ),
                "{name} at protocol version 14: expected VersionNotActive, got {result:?}"
            );
        }
        for (name, result) in calls(latest) {
            assert!(
                result.is_ok(),
                "{name} at the latest protocol version: {result:?}"
            );
        }
    }

    /// What an escrow write costs, priced without state (`estimated`) and applied to the state
    /// it writes to (`applied`).
    fn estimated_and_applied_fees(
        drive: &Drive,
        build: impl Fn(
            &mut Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>,
        ) -> Vec<LowLevelDriveOperation>,
    ) -> (FeeResult, FeeResult) {
        let platform_version = PlatformVersion::latest();
        let fee = |costs: Vec<LowLevelDriveOperation>| {
            Drive::calculate_fee(
                None,
                Some(costs),
                &BlockInfo::default().epoch,
                drive.config.epochs_per_era,
                platform_version,
                None,
            )
            .expect("expected a fee")
        };

        let mut estimation = Some(HashMap::new());
        let operations = build(&mut estimation);
        let mut estimated_costs = vec![];
        drive
            .apply_batch_low_level_drive_operations(
                estimation,
                None,
                operations,
                &mut estimated_costs,
                &platform_version.drive,
            )
            .expect("expected to estimate");

        let operations = build(&mut None);
        let mut applied_costs = vec![];
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut applied_costs,
                &platform_version.drive,
            )
            .expect("expected to apply");

        (fee(estimated_costs), fee(applied_costs))
    }

    /// Characterization: the estimate of each escrow write covers what applying it costs on the
    /// initial state, where the offers' escrow key sits on the third level of the prefunded
    /// specialized balances Merk and a listing's on the fourth of the Tokens Merk.
    #[test]
    fn should_estimate_at_least_what_an_escrow_write_costs() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let token_id = [5; 32];

        let (estimated, applied) = estimated_and_applied_fees(&drive, |estimation| {
            drive
                .add_to_credit_order_escrow_operations(
                    [9; 32],
                    70_000,
                    estimation,
                    platform_version,
                )
                .expect("expected the operations")
        });
        assert!(
            estimated.processing_fee >= applied.processing_fee,
            "an offer's escrow: estimated {estimated:?}, applied {applied:?}"
        );
        assert!(estimated.storage_fee >= applied.storage_fee);

        let (estimated, applied) = estimated_and_applied_fees(&drive, |estimation| {
            drive
                .set_credit_order_escrow_operations([9; 32], 30_000, estimation, platform_version)
                .expect("expected the operations")
        });
        assert!(
            estimated.processing_fee >= applied.processing_fee,
            "an offer's escrow after a fill: estimated {estimated:?}, applied {applied:?}"
        );

        let (estimated, applied) = estimated_and_applied_fees(&drive, |estimation| {
            drive
                .add_to_token_order_escrow_operations(
                    token_id,
                    [1; 32],
                    500,
                    estimation,
                    None,
                    &mut None,
                    platform_version,
                )
                .expect("expected the operations")
        });
        assert!(
            estimated.processing_fee >= applied.processing_fee,
            "a token's first listing: estimated {estimated:?}, applied {applied:?}"
        );
        assert!(estimated.storage_fee >= applied.storage_fee);

        let (estimated, applied) = estimated_and_applied_fees(&drive, |estimation| {
            drive
                .set_token_order_escrow_operations(
                    token_id,
                    [1; 32],
                    200,
                    estimation,
                    platform_version,
                )
                .expect("expected the operations")
        });
        assert!(
            estimated.processing_fee >= applied.processing_fee,
            "a listing after a fill: estimated {estimated:?}, applied {applied:?}"
        );
    }

    /// The offers' escrow is described at its key's depth in the prefunded specialized balances
    /// layer, and from protocol version 15 the voting balance and fee pot estimations of the
    /// same batch keep that deeper description, whichever registers first.
    #[test]
    fn should_keep_the_deeper_prefunded_balances_layer_estimate() {
        let platform_version = PlatformVersion::latest();
        let prefunded_layer_level =
            |estimation: &HashMap<KeyInfoPath, EstimatedLayerInformation>| match estimation
                .get(&KeyInfoPath::from_known_path(
                    prefunded_specialized_balances_path(),
                ))
                .map(|layer| &layer.estimated_layer_count)
            {
                Some(EstimatedLevel(level, _)) => *level,
                other => panic!("expected an estimated level, got {other:?}"),
            };
        let voting = |estimation: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
                      platform_version: &PlatformVersion| {
            Drive::add_estimation_costs_for_prefunded_specialized_balance_update(
                estimation,
                &platform_version.drive,
            )
            .expect("expected the voting balance estimation")
        };
        let fee_pot = |estimation: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
                       platform_version: &PlatformVersion| {
            Drive::add_estimation_costs_for_contract_fee_pot_update(
                ContractFeePot::Owner,
                estimation,
                &platform_version.drive,
            )
            .expect("expected the fee pot estimation")
        };
        let escrow = |estimation: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>| {
            Drive::add_estimation_costs_for_credit_order_escrow(estimation, platform_version)
                .expect("expected the escrow estimation")
        };

        type Sibling = fn(&mut HashMap<KeyInfoPath, EstimatedLayerInformation>, &PlatformVersion);
        let siblings: [Sibling; 2] = [voting, fee_pot];
        for sibling in siblings {
            // The escrow first, the sibling after: the sibling keeps the escrow's depth.
            let mut estimation = HashMap::new();
            escrow(&mut estimation);
            assert_eq!(prefunded_layer_level(&estimation), 2);
            sibling(&mut estimation, platform_version);
            assert_eq!(prefunded_layer_level(&estimation), 2);

            // The sibling first, the escrow after.
            let mut estimation = HashMap::new();
            sibling(&mut estimation, platform_version);
            assert_eq!(prefunded_layer_level(&estimation), 1);
            escrow(&mut estimation);
            assert_eq!(prefunded_layer_level(&estimation), 2);

            // Protocol version 14's siblings describe the layer at their keys' level, as before.
            let mut estimation = HashMap::new();
            escrow(&mut estimation);
            sibling(
                &mut estimation,
                PlatformVersion::get(14).expect("expected protocol version 14"),
            );
            assert_eq!(prefunded_layer_level(&estimation), 1);
        }
    }

    /// A batch moving tokens between a balance and escrow describes the Tokens layer at the
    /// escrow key's depth whichever of the two operations is built first. At protocol version 14
    /// the balance estimation built last lowers it to the balances key, as it always did.
    #[test]
    fn should_keep_the_escrow_depth_whichever_of_balance_and_escrow_is_built_first() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let token_id = [5; 32];
        let tokens_layer_level =
            |estimation: &Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>| match estimation
                .as_ref()
                .expect("expected an estimation")
                .get(&KeyInfoPath::from_known_path(tokens_root_path()))
                .map(|layer| &layer.estimated_layer_count)
            {
                Some(EstimatedLevel(level, _)) => *level,
                other => panic!("expected an estimated level, got {other:?}"),
            };
        let balance = |estimation: &mut Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>,
                       platform_version: &PlatformVersion| {
            drive
                .remove_from_identity_token_balance_operations(
                    token_id,
                    [7; 32],
                    500,
                    estimation,
                    None,
                    platform_version,
                )
                .expect("expected the balance operations");
        };
        let escrow = |estimation: &mut Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>| {
            drive
                .add_to_token_order_escrow_operations(
                    token_id,
                    [9; 32],
                    500,
                    estimation,
                    None,
                    &mut None,
                    platform_version,
                )
                .expect("expected the escrow operations");
        };

        let mut estimation = Some(HashMap::new());
        balance(&mut estimation, platform_version);
        escrow(&mut estimation);
        assert_eq!(tokens_layer_level(&estimation), 3, "the balance first");

        let mut estimation = Some(HashMap::new());
        escrow(&mut estimation);
        balance(&mut estimation, platform_version);
        assert_eq!(tokens_layer_level(&estimation), 3, "the escrow first");

        let mut estimation = Some(HashMap::new());
        escrow(&mut estimation);
        balance(
            &mut estimation,
            PlatformVersion::get(14).expect("expected protocol version 14"),
        );
        assert_eq!(tokens_layer_level(&estimation), 0, "protocol version 14");
    }

    /// An offer's batch debits the buyer's balance and credits the offer's escrow: built in
    /// either order, the estimation describes the root at the escrow's depth (3) and the
    /// prefunded specialized balances layer at the escrow key's (2). At protocol version 14 the
    /// balance estimation built last lowers the root to the balances' level (1), as it always
    /// did.
    #[test]
    fn should_keep_the_credit_escrow_depths_whichever_of_debit_and_escrow_is_built_first() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let level_at = |estimation: &Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>,
                        path: KeyInfoPath| match estimation
            .as_ref()
            .expect("expected an estimation")
            .get(&path)
            .map(|layer| &layer.estimated_layer_count)
        {
            Some(EstimatedLevel(level, _)) => *level,
            other => panic!("expected an estimated level, got {other:?}"),
        };
        let root = || KeyInfoPath::from_known_path([]);
        let prefunded = || KeyInfoPath::from_known_path(prefunded_specialized_balances_path());
        let debit = |estimation: &mut Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>,
                     platform_version: &PlatformVersion| {
            drive
                .remove_from_identity_balance_operations(
                    [7; 32],
                    70_000,
                    estimation,
                    None,
                    platform_version,
                )
                .expect("expected the debit operations");
        };
        let escrow = |estimation: &mut Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>| {
            drive
                .add_to_credit_order_escrow_operations(
                    [9; 32],
                    70_000,
                    estimation,
                    platform_version,
                )
                .expect("expected the escrow operations");
        };

        let mut estimation = Some(HashMap::new());
        debit(&mut estimation, platform_version);
        escrow(&mut estimation);
        assert_eq!(level_at(&estimation, root()), 3, "the debit first");
        assert_eq!(level_at(&estimation, prefunded()), 2, "the debit first");

        let mut estimation = Some(HashMap::new());
        escrow(&mut estimation);
        debit(&mut estimation, platform_version);
        assert_eq!(level_at(&estimation, root()), 3, "the escrow first");
        assert_eq!(level_at(&estimation, prefunded()), 2, "the escrow first");

        let mut estimation = Some(HashMap::new());
        escrow(&mut estimation);
        debit(
            &mut estimation,
            PlatformVersion::get(14).expect("expected protocol version 14"),
        );
        assert_eq!(level_at(&estimation, root()), 1, "protocol version 14");
    }
}
