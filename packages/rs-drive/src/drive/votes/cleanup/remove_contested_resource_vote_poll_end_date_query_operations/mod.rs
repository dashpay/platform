mod v0;
mod v1;
mod v2;

use crate::drive::Drive;
use std::collections::BTreeMap;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any votes poll should be closed. This will remove them to recoup space
    // TODO: Use type of struct
    #[allow(clippy::type_complexity)]
    pub fn remove_contested_resource_vote_poll_end_date_query_operations(
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
            .remove_contested_resource_vote_poll_end_date_query_operations
        {
            0 => self.remove_contested_resource_vote_poll_end_date_query_operations_v0(
                vote_polls,
                batch_operations,
                transaction,
                platform_version,
            ),
            1 => self.remove_contested_resource_vote_poll_end_date_query_operations_v1(
                vote_polls,
                batch_operations,
                transaction,
                platform_version,
            ),
            2 => self.remove_contested_resource_vote_poll_end_date_query_operations_v2(
                vote_polls,
                batch_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_contested_resource_vote_poll_end_date_query_operations".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::v2::tests::{add_end_date, setup, vote_poll, T1, T2};
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::drive::Drive;
    use crate::fees::op::LowLevelDriveOperation;
    use dpp::identifier::Identifier;
    use dpp::identity::TimestampMillis;
    use dpp::version::PlatformVersion;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use std::collections::BTreeMap;

    type EndedVotePoll<'a> = (
        &'a ContestedDocumentResourceVotePollWithContractInfo,
        &'a TimestampMillis,
        &'a BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
    );

    fn dispatched_operations(
        drive: &Drive,
        vote_polls: &[EndedVotePoll],
        platform_version: &PlatformVersion,
    ) -> Vec<LowLevelDriveOperation> {
        let mut operations = vec![];
        drive
            .remove_contested_resource_vote_poll_end_date_query_operations(
                vote_polls,
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected end date removal operations");
        operations
    }

    fn v1_operations(
        drive: &Drive,
        vote_polls: &[EndedVotePoll],
        platform_version: &PlatformVersion,
    ) -> Vec<LowLevelDriveOperation> {
        let mut operations = vec![];
        drive
            .remove_contested_resource_vote_poll_end_date_query_operations_v1(
                vote_polls,
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected v1 end date removal operations");
        operations
    }

    #[test]
    fn should_keep_the_end_date_removal_of_protocol_version_13() {
        let platform_version_13 = PlatformVersion::get(13).expect("expected protocol version 13");
        let platform_version = PlatformVersion::latest();
        let (drive, dpns_contract) = setup(platform_version_13);
        let a = vote_poll(&dpns_contract, "a0000");
        let b = vote_poll(&dpns_contract, "b0000");
        let c = vote_poll(&dpns_contract, "c0000");
        add_end_date(&drive, &a, T1, platform_version_13);
        add_end_date(&drive, &b, T2, platform_version_13);
        add_end_date(&drive, &c, T2, platform_version_13);
        let no_votes = BTreeMap::new();
        let ended: [EndedVotePoll; 2] = [(&a, &T1, &no_votes), (&b, &T2, &no_votes)];

        assert_eq!(
            dispatched_operations(&drive, &ended, platform_version_13),
            v1_operations(&drive, &ended, platform_version_13),
            "protocol version 13 removes end dates with v1"
        );
        assert_ne!(
            dispatched_operations(&drive, &ended, platform_version),
            v1_operations(&drive, &ended, platform_version),
            "the latest protocol version removes end dates with v2"
        );
    }
}
