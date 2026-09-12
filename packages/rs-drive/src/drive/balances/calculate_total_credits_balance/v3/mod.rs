use crate::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
use crate::drive::system::misc_path;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::balances::total_credits_balance::TotalCreditsBalance;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;
use grovedb_path::SubtreePath;

impl Drive {
    /// Verify that the sum tree identity credits + pool credits + refunds + address
    /// credits + shielded credits + contract credits are equal to the Total credits
    /// in the system.
    ///
    /// v3 adds the `ContractCredits` root sum tree (introduced at protocol v17 /
    /// drive v10) as a sixth term in the equation. The tree's aggregate only
    /// covers live contracts: a wiped contract's subtree is wrapped in a
    /// not-summed element and contributes nothing. Earlier calculators do not
    /// read it because the tree does not exist on pre-v17 chains.
    #[inline(always)]
    pub(super) fn calculate_total_credits_balance_v3(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<TotalCreditsBalance, Error> {
        let mut drive_operations = vec![];
        let path_holding_total_credits = misc_path();
        let total_credits_in_platform = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path_holding_total_credits).into(),
                TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                drive_version,
            )?
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits not found in Platform",
            )))?;

        let total_identity_balances = self.grove_get_sum_tree_total_value(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::Balances),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        let total_specialized_balances = self.grove_get_sum_tree_total_value(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::PreFundedSpecializedBalances),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        let total_in_pools = self.grove_get_sum_tree_total_value(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::Pools),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        let total_in_addresses = self.grove_get_sum_tree_total_value(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::AddressBalances),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        let total_in_shielded_balances = self.grove_get_sum_tree_total_value(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::ShieldedBalances),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        let total_in_contract_credits = self.grove_get_sum_tree_total_value(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::ContractCredits),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        Ok(TotalCreditsBalance {
            total_credits_in_platform,
            total_in_pools,
            total_identity_balances,
            total_specialized_balances,
            total_in_addresses,
            total_in_shielded_balances,
            total_in_contract_credits,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::contract::balances::{contract_credits_path, contract_credits_root_path};
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use grovedb::Element;
    use grovedb_path::SubtreePath;

    /// Credits a contract directly in the contract credits tree, bypassing
    /// the bucket operations that later changes add: a raw sum subtree for
    /// the contract holding one sum item. Enough to move the root aggregate.
    fn credit_contract_raw(
        drive: &Drive,
        contract_id: [u8; 32],
        amount: i64,
        platform_version: &PlatformVersion,
    ) {
        let grove_version = &platform_version.drive.grove_version;
        drive
            .grove
            .insert(
                SubtreePath::from(&contract_credits_root_path()),
                &contract_id,
                Element::empty_sum_tree(),
                None,
                None,
                grove_version,
            )
            .unwrap()
            .expect("expected to insert the contract sum tree");
        drive
            .grove
            .insert(
                SubtreePath::from(&contract_credits_path(&contract_id)),
                &[0, 1],
                Element::new_sum_item(amount),
                None,
                None,
                grove_version,
            )
            .unwrap()
            .expect("expected to insert the bucket sum item");
    }

    #[test]
    fn should_balance_credits_with_empty_contract_credits_root() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        let total = drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to calculate the total credits balance");

        assert_eq!(total.total_in_contract_credits, 0);
        assert!(total.ok().expect("no overflow"));
    }

    #[test]
    fn should_read_contract_credits_as_a_term_of_the_equation() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));

        credit_contract_raw(&drive, [1u8; 32], 700, platform_version);
        credit_contract_raw(&drive, [2u8; 32], 300, platform_version);

        // Credits in the contract trees without the matching system credits
        // are not balanced: the term is read, not assumed.
        let unbalanced = drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to calculate the total credits balance");
        assert_eq!(unbalanced.total_in_contract_credits, 1000);
        assert!(!unbalanced.ok().expect("no overflow"));

        drive
            .add_to_system_credits(1000, None, platform_version)
            .expect("expected to add to system credits");

        let balanced = drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to calculate the total credits balance");
        assert_eq!(balanced.total_in_contract_credits, 1000);
        assert!(balanced.ok().expect("no overflow"));
        assert_eq!(balanced.total_in_trees().expect("no overflow"), 1000);
    }

    #[test]
    fn should_exclude_a_not_summed_contract_tree_from_the_term() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let grove_version = &platform_version.drive.grove_version;

        credit_contract_raw(&drive, [1u8; 32], 700, platform_version);

        // A wiped contract keeps its subtree, wrapped so the root aggregate
        // ignores it. The retained amount is real storage but not live credit.
        let wiped_id = [9u8; 32];
        drive
            .grove
            .insert(
                SubtreePath::from(&contract_credits_root_path()),
                &wiped_id,
                Element::new_not_summed(Element::empty_sum_tree())
                    .expect("a sum tree can be wrapped"),
                None,
                None,
                grove_version,
            )
            .unwrap()
            .expect("expected to insert the wiped contract tree");
        drive
            .grove
            .insert(
                SubtreePath::from(&contract_credits_path(&wiped_id)),
                &[0, 1],
                Element::new_sum_item(5000),
                None,
                None,
                grove_version,
            )
            .unwrap()
            .expect("expected to insert the retained bucket");

        drive
            .add_to_system_credits(700, None, platform_version)
            .expect("expected to add to system credits");

        let total = drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to calculate the total credits balance");
        assert_eq!(
            total.total_in_contract_credits, 700,
            "the retained credits of the wiped contract must not be counted"
        );
        assert!(total.ok().expect("no overflow"));
    }
}
