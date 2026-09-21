use crate::drive::votes::paths::RESOURCE_STORED_INFO_KEY_U8_32;
use crate::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderVotePollPaths;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::PathKeyElementInfo::PathKeyElement;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::IdentityContenderVotePollStoredInfo;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::{Element, TransactionArg};

impl Drive {
    pub(super) fn insert_stored_info_for_identity_contender_vote_poll_v0(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        stored_info: IdentityContenderVotePollStoredInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self
            .insert_stored_info_for_identity_contender_vote_poll_operations_v0(
                vote_poll,
                stored_info,
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

    pub(super) fn insert_stored_info_for_identity_contender_vote_poll_operations_v0(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        stored_info: IdentityContenderVotePollStoredInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.batch_insert::<0>(
            PathKeyElement((
                vote_poll.poll_path_vec()?,
                RESOURCE_STORED_INFO_KEY_U8_32.to_vec(),
                Element::new_item(stored_info.serialize_consume_to_bytes()?),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Ok(drive_operations)
    }
}
