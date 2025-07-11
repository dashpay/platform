mod v0;
mod v1;

use crate::drive::Drive;
use std::collections::BTreeMap;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any votes poll should be closed. This will remove them to recoup space
    // TODO: Use type of struct
    #[allow(clippy::type_complexity)]
    pub fn remove_contested_resource_vote_poll_contenders_operations(
        &self,
        vote_polls: &[(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .cleanup
            .remove_contested_resource_vote_poll_contenders_operations
        {
            0 => self.remove_contested_resource_vote_poll_contenders_operations_v0(
                vote_polls,
                batch_operations,
                transaction,
                platform_version,
            ),
            1 => self.remove_contested_resource_vote_poll_contenders_operations_v1(
                vote_polls,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_contested_resource_vote_poll_contenders_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
