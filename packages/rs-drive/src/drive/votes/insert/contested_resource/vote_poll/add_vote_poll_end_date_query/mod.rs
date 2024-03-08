mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::fee::fee_result::FeeResult;

use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// We add vote poll references by end date in order to be able to check on every new block if
    /// any vote poll should be closed.
    pub fn add_vote_poll_end_date_query(
        &self,
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .contested_resource_insert
            .register_identity_vote_for_identity_queries
        {
            0 => self.add_vote_poll_end_date_query_v0(identity_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_identity_vote_for_identity_queries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
