use crate::drive::votes::paths::{YesNoVotePollPaths, YES_NO_VOTE_POLL_STORED_INFO_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo::PathKeyElement;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStoredInfo;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::{Element, TransactionArg};

impl Drive {
    pub(super) fn insert_stored_info_for_yes_no_vote_poll_v0(
        &self,
        vote_poll: &YesNoVotePoll,
        stored_info: YesNoVotePollStoredInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.insert_stored_info_for_yes_no_vote_poll_operations_v0(
            vote_poll,
            stored_info,
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut vec![],
            &platform_version.drive,
        )
    }

    pub(super) fn insert_stored_info_for_yes_no_vote_poll_operations_v0(
        &self,
        vote_poll: &YesNoVotePoll,
        stored_info: YesNoVotePollStoredInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let serialization = stored_info.serialize_consume_to_bytes()?;
        self.batch_insert::<0>(
            PathKeyElement((
                vote_poll.poll_path_vec()?,
                vec![YES_NO_VOTE_POLL_STORED_INFO_KEY],
                Element::new_item(serialization),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Ok(drive_operations)
    }
}
