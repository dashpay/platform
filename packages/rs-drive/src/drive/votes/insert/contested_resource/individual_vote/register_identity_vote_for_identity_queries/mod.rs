mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use dpp::prelude::Identifier;

impl Drive {

    /// We register the identity vote to be able to query the current votes of an identity, or to
    /// be able to remove votes from a "disabled" identity (ie a masternode that was removed from
    /// the list).
    pub fn register_identity_vote_for_identity_queries(
        &self,
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.vote.contested_resource_insert.register_identity_vote_for_identity_queries {
            0 => self.register_identity_vote_for_identity_queries_v0(
                identity_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_identity_vote_for_identity_queries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}