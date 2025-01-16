use crate::drive::tokens::paths::{tokens_root_path_vec, TOKEN_IDENTITY_INFO_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    pub(super) fn prove_token_statuses_v0(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_token_statuses_operations_v0(
            token_ids,
            transaction,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_token_statuses_operations_v0(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let tokens_root = tokens_root_path_vec();

        let mut query = Query::new();

        for token_id in token_ids {
            query.insert_key(token_id.to_vec());
        }

        query.set_subquery_path(vec![vec![TOKEN_IDENTITY_INFO_KEY]]);

        let path_query = PathQuery::new(
            tokens_root,
            SizedQuery::new(query, Some(token_ids.len() as u16), None),
        );

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
