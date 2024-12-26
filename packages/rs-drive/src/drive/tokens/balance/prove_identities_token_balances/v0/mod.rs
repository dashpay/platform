use crate::drive::tokens::token_balances_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    pub(super) fn prove_identities_token_balances_v0(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_identities_token_balances_operations_v0(
            token_id,
            identity_ids,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_identities_token_balances_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let tokens_root = token_balances_path_vec(token_id);

        let mut query = Query::new();

        for identity_id in identity_ids {
            query.insert_key(identity_id.to_vec());
        }

        let path_query = PathQuery::new(
            tokens_root,
            SizedQuery::new(query, Some(identity_ids.len() as u16), None),
        );

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
