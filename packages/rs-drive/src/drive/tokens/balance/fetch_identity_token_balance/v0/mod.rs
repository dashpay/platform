use crate::drive::tokens::paths::token_balances_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use grovedb::Element::SumItem;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    pub(super) fn fetch_identity_token_balance_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        self.fetch_identity_token_balance_operations_v0(
            token_id,
            identity_id,
            true,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_identity_token_balance_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            // 8 is the size of an i64 used in sum trees
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(8),
            }
        };

        let balance_path = token_balances_path(&token_id);

        match self.grove_get_raw_optional(
            (&balance_path).into(),
            identity_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(SumItem(balance, _))) if balance >= 0 => Ok(Some(balance as TokenAmount)),

            Ok(None) => {
                // If we are applying (stateful), no balance means None.
                // If we are estimating (stateless), return Some(0) to indicate no cost or minimal cost scenario.
                if apply {
                    Ok(None)
                } else {
                    Ok(Some(0))
                }
            }
            Err(Error::GroveDB(e)) if matches!(e.as_ref(), grovedb::Error::PathKeyNotFound(_)) => {
                if apply {
                    Ok(None)
                } else {
                    Ok(Some(0))
                }
            }

            Ok(Some(SumItem(..))) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity token balance was present but was negative",
            ))),

            Ok(Some(_)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity token balance was present but was not a sum item",
            ))),

            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_return_none_for_non_existent_token_tree() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        // Token tree never created -> PathKeyNotFound -> None when apply=true
        let token_id = [200u8; 32];
        let identity_id = [201u8; 32];

        let balance = drive
            .fetch_identity_token_balance_v0(token_id, identity_id, None, platform_version)
            .expect("expected fetch to succeed");
        assert_eq!(balance, None);
    }

    #[test]
    fn should_return_none_for_unknown_identity_on_existing_token() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [210u8; 32];
        let contract_id = Identifier::from([211u8; 32]);

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

        // Identity has no balance entry -> returns None (Ok(None) branch, apply=true)
        let unknown_identity = [212u8; 32];
        let balance = drive
            .fetch_identity_token_balance_v0(token_id, unknown_identity, None, platform_version)
            .expect("expected fetch to succeed");
        assert_eq!(balance, None);
    }

    #[test]
    fn should_return_zero_in_stateless_mode_for_missing_balance() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let token_id = [220u8; 32];
        let identity_id = [221u8; 32];

        // Stateless mode: apply=false, no state, must return Some(0) (estimation branch)
        let mut drive_operations = vec![];
        let balance = drive
            .fetch_identity_token_balance_operations_v0(
                token_id,
                identity_id,
                false,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected estimation fetch to succeed");

        assert_eq!(balance, Some(0));
    }

    #[test]
    fn should_return_stored_balance_when_present() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [230u8; 32];
        let identity_id = [231u8; 32];
        let contract_id = Identifier::from([232u8; 32]);

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
            .add_to_identity_token_balance(
                token_id,
                identity_id,
                42_000,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to add balance");

        // Covers the Ok(Some(SumItem(balance, _))) where balance >= 0 branch
        let balance = drive
            .fetch_identity_token_balance_v0(token_id, identity_id, None, platform_version)
            .expect("expected fetch to succeed");
        assert_eq!(balance, Some(42_000));
    }
}
