use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use grovedb::Element::SumItem;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_identity_token_balances_v0(
        &self,
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<TokenAmount>>, Error> {
        self.fetch_identity_token_balances_operations_v0(
            token_ids,
            identity_id,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn fetch_identity_token_balances_operations_v0(
        &self,
        token_ids: &[[u8; 32]],
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<TokenAmount>>, Error> {
        let path_query = Drive::token_balances_for_identity_id_query(token_ids, identity_id);

        self.grove_get_raw_path_query_with_optional(
            &path_query,
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .into_iter()
        .map(|(path, _, element)| {
            let token_id: [u8; 32] = path
                .get(2)
                .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                    "returned path item should always have a third part at index 2".to_string(),
                )))?
                .clone()
                .try_into()
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "token id not 32 bytes".to_string(),
                    ))
                })?;
            match element {
                Some(SumItem(value, ..)) => Ok((token_id, Some(value as TokenAmount))),
                None => Ok((token_id, None)),
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
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_return_mix_of_present_and_missing_balances() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id_a = [80u8; 32];
        let token_id_b = [81u8; 32];
        let token_id_c = [82u8; 32];
        let identity_id = [83u8; 32];
        let contract_id_a = Identifier::from([84u8; 32]);
        let contract_id_b = Identifier::from([85u8; 32]);
        let contract_id_c = Identifier::from([86u8; 32]);

        // Create trees for all three tokens but only populate balances for A and C
        for (tid, cid) in [
            (token_id_a, contract_id_a),
            (token_id_b, contract_id_b),
            (token_id_c, contract_id_c),
        ] {
            drive
                .create_token_trees(
                    cid,
                    0,
                    tid,
                    false,
                    false,
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to create token trees");
        }

        drive
            .add_to_identity_token_balance(
                token_id_a,
                identity_id,
                111,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to add balance for A");

        drive
            .add_to_identity_token_balance(
                token_id_c,
                identity_id,
                333,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to add balance for C");

        // Token B has no balance for this identity -> None
        let balances = drive
            .fetch_identity_token_balances_v0(
                &[token_id_a, token_id_b, token_id_c],
                identity_id,
                None,
                platform_version,
            )
            .expect("expected fetch to succeed");

        assert_eq!(
            balances,
            BTreeMap::from([
                (token_id_a, Some(111)),
                (token_id_b, None),
                (token_id_c, Some(333)),
            ])
        );
    }

    #[test]
    fn should_return_all_none_for_identity_without_any_balances() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [90u8; 32];
        let contract_id = Identifier::from([91u8; 32]);

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

        let unknown_identity = [92u8; 32];
        let balances = drive
            .fetch_identity_token_balances_v0(&[token_id], unknown_identity, None, platform_version)
            .expect("expected fetch to succeed");

        assert_eq!(balances, BTreeMap::from([(token_id, None)]));
    }
}
