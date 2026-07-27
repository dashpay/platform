use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::Element::SumItem;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_identities_token_balances_v0(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, Option<TokenAmount>>, Error> {
        self.fetch_identities_token_balances_operations_v0(
            token_id,
            identity_ids,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_identities_token_balances_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, Option<TokenAmount>>, Error> {
        let path_query = Self::token_balances_for_identity_ids_query(token_id, identity_ids);

        self.grove_get_raw_path_query_with_optional(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .into_iter()
        .map(|(_, key, element)| {
            let identity_id: Identifier = key.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "identity id not 32 bytes".to_string(),
                ))
            })?;
            match element {
                Some(SumItem(value, ..)) => Ok((identity_id, Some(value as TokenAmount))),
                None => Ok((identity_id, None)),
                _ => Err(Error::Drive(DriveError::CorruptedDriveState(
                    "token tree for balances should contain only sum items".to_string(),
                ))),
            }
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_aggregate_mixed_balances_across_identities() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [150u8; 32];
        let contract_id = Identifier::from([151u8; 32]);
        let id_with_balance_a = [152u8; 32];
        let id_with_balance_b = [153u8; 32];
        let id_without_balance = [154u8; 32];

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
                id_with_balance_a,
                1_000,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to add balance A");

        drive
            .add_to_identity_token_balance(
                token_id,
                id_with_balance_b,
                2_500,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to add balance B");

        let balances = drive
            .fetch_identities_token_balances_v0(
                token_id,
                &[id_with_balance_a, id_with_balance_b, id_without_balance],
                None,
                platform_version,
            )
            .expect("expected fetch to succeed");

        assert_eq!(
            balances,
            BTreeMap::from([
                (Identifier::from(id_with_balance_a), Some(1_000)),
                (Identifier::from(id_with_balance_b), Some(2_500)),
                (Identifier::from(id_without_balance), None),
            ])
        );
    }

    #[test]
    fn should_return_none_for_every_identity_when_token_tree_missing() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        // Token tree never created
        let unknown_token = [160u8; 32];
        let identities = [[161u8; 32], [162u8; 32]];

        let balances = drive
            .fetch_identities_token_balances_v0(unknown_token, &identities, None, platform_version)
            .expect("expected fetch to succeed even when token missing");

        // Every returned entry must be None when the token tree is missing.
        for (_, balance) in balances.iter() {
            assert!(
                balance.is_none(),
                "expected all balances to be None when token tree missing"
            );
        }
    }
}
