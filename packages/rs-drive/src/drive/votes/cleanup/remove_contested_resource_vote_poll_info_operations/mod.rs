mod v0;

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
    /// We remove the entire vote poll
    pub fn remove_contested_resource_info_operations(
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
            .remove_contested_resource_info_operations
        {
            0 => self.remove_contested_resource_info_operations_v0(
                vote_polls,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_contested_resource_info_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
