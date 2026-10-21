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
    use crate::drive::Drive;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;

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
}
