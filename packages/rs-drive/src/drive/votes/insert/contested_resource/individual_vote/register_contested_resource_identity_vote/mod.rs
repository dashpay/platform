mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::fee::fee_result::FeeResult;

use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::{ContestedDocumentResourceVoteType, Vote};
use grovedb::TransactionArg;

impl Drive {
    pub fn register_contested_resource_identity_vote(
        &self,
        vote: ContestedDocumentResourceVoteType,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .contested_resource_insert
            .register_identity_vote
        {
            0 => self.register_contested_resource_identity_vote_v0(
                vote,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "register_identity_vote".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
