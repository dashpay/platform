use crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::drive::votes::paths::{VotePollPaths, RESOURCE_FINISHED_INFO_KEY_U8};
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::serialization::PlatformSerializable;
use dpp::voting::vote_outcomes::finalized_contested_document_vote_poll_stored_info::FinalizedContestedDocumentVotePollStoredInfo;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn insert_record_of_finished_vote_poll_v0(
        &self,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        finalized_contested_document_vote_poll_stored_info: FinalizedContestedDocumentVotePollStoredInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.insert_record_of_finished_vote_poll_operations_v0(
            vote_poll,
            finalized_contested_document_vote_poll_stored_info,
            platform_version,
        )?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(())
    }

    pub(super) fn insert_record_of_finished_vote_poll_operations_v0(
        &self,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        finalized_contested_document_vote_poll_stored_info: FinalizedContestedDocumentVotePollStoredInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let serialization =
            finalized_contested_document_vote_poll_stored_info.serialize_consume_to_bytes()?;
        let vote_poll_root_path = vote_poll.contenders_path(platform_version)?;

        self.batch_insert::<0>(
            PathKeyElement((
                vote_poll_root_path.clone(),
                vec![RESOURCE_FINISHED_INFO_KEY_U8],
                Element::new_item(serialization),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(drive_operations)
    }
}
