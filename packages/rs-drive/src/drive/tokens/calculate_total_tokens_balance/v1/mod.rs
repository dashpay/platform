use crate::drive::balances::TOTAL_TOKEN_SUPPLIES_STORAGE_KEY;
use crate::drive::system::misc_path;
use crate::drive::tokens::paths::{tokens_root_path, TOKEN_BALANCES_KEY};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::balances::credits::SumTokenAmount;
use dpp::balances::total_tokens_balance::TotalTokensBalance;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Generation 1: the two aggregates of v0 plus the destroyed supply ledger, read once.
    #[inline(always)]
    pub(super) fn calculate_total_tokens_balance_v1(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<TotalTokensBalance, Error> {
        let mut drive_operations = vec![];
        let path_holding_total_credits = misc_path();
        let total_tokens_in_platform = self.grove_get_big_sum_tree_total_value(
            (&path_holding_total_credits).into(),
            TOTAL_TOKEN_SUPPLIES_STORAGE_KEY,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let tokens_root_path = tokens_root_path();

        let total_identity_token_balances = self.grove_get_big_sum_tree_total_value(
            (&tokens_root_path).into(),
            &[TOKEN_BALANCES_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let total_destroyed_supply = self
            .fetch_token_destroyed_supply_operations(
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .ok_or_else(|| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "the destroyed token supply ledger is missing".to_string(),
                ))
            })?;
        let total_destroyed_supply =
            SumTokenAmount::try_from(total_destroyed_supply).map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "the destroyed token supply does not fit a signed 128 bit total".to_string(),
                ))
            })?;

        Ok(TotalTokensBalance {
            total_tokens_in_platform,
            total_identity_token_balances,
            total_destroyed_supply,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::tokens::lifecycle::encode_destroyed_supply;
    use crate::drive::tokens::paths::{
        token_contract_lifecycles_root_path, TOKEN_DESTROYED_SUPPLY_KEY,
    };
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn should_read_zero_totals_at_genesis() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let totals = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected totals");

        assert_eq!(totals.total_tokens_in_platform, 0);
        assert_eq!(totals.total_identity_token_balances, 0);
        assert_eq!(totals.total_destroyed_supply, 0);
        assert!(totals.ok().expect("expected a verdict"));
    }

    #[test]
    fn should_keep_the_equation_across_mint_burn_and_destruction() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let contract_id = Identifier::from([3u8; 32]);
        let token_id = [1u8; 32];
        let holder = [7u8; 32];

        drive
            .create_token_trees(
                contract_id,
                0,
                token_id,
                false,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");
        drive
            .token_mint(
                token_id,
                holder,
                900,
                false,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to mint");
        drive
            .token_burn(
                token_id,
                holder,
                100,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to burn");

        let totals = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected totals");
        assert_eq!(totals.total_tokens_in_platform, 800);
        assert_eq!(totals.total_identity_token_balances, 800);
        assert_eq!(totals.total_destroyed_supply, 0);
        assert!(totals.ok().expect("expected a verdict"));

        drive
            .destroy_token_issuer(
                contract_id.to_buffer(),
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the issuer");

        let totals = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected totals");
        assert_eq!(totals.total_tokens_in_platform, 800);
        assert_eq!(totals.total_identity_token_balances, 800);
        assert_eq!(totals.total_destroyed_supply, 800);
        assert_eq!(
            totals.active_supply().expect("expected an active supply"),
            0
        );
        assert!(totals.ok().expect("expected a verdict"));
    }

    #[test]
    fn should_fail_when_the_ledger_is_missing() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        drive
            .grove
            .delete(
                &token_contract_lifecycles_root_path(),
                &TOKEN_DESTROYED_SUPPLY_KEY,
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to delete the scalar");

        let result = drive.calculate_total_tokens_balance(None, platform_version);

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }

    #[test]
    fn should_report_a_destroyed_supply_above_the_raw_supply_as_not_ok() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        drive
            .grove
            .insert(
                &token_contract_lifecycles_root_path(),
                &TOKEN_DESTROYED_SUPPLY_KEY,
                Element::new_item(encode_destroyed_supply(1)),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("expected to overwrite the scalar");

        let totals = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected totals");

        assert_eq!(totals.total_destroyed_supply, 1);
        assert!(!totals.ok().expect("expected a verdict"));
    }
}
